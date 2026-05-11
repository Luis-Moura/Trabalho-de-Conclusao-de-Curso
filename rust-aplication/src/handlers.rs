use actix_web::{web, HttpResponse};
use rand::Rng;
use sqlx::PgPool;
use std::fmt;

use crate::models::{ShortenRequest, ShortenResponse};

// ── Constantes de geração de código ──────────────────────────────────────────

const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const CODE_LEN: usize = 7;

/// Gera um código base62 de 7 caracteres usando o RNG local da thread.
/// `thread_rng()` é por-thread — sem lock, custo zero de contenção.
fn generate_code() -> String {
    let mut rng = rand::thread_rng();
    (0..CODE_LEN)
        .map(|_| BASE62[rng.gen_range(0..62)] as char)
        .collect()
}

// ── Tipo de erro da aplicação ─────────────────────────────────────────────────

#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    NotFound,
    CollisionExhausted,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(e) => write!(f, "Database error: {e}"),
            AppError::NotFound => write!(f, "Short code not found"),
            AppError::CollisionExhausted => write!(f, "Failed to generate unique code after retries"),
        }
    }
}

impl actix_web::ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::NotFound => HttpResponse::NotFound().finish(),
            _ => HttpResponse::InternalServerError().finish(),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e)
    }
}

// ── Estrutura auxiliar para leitura da query SELECT ───────────────────────────

#[derive(sqlx::FromRow)]
struct UrlRecord {
    original_url: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /shorten
/// Recebe {"url": "..."} e retorna 201 Created com {"shortUrl": "http://host/XXXXXXX"}.
/// Rejeita colisões de chave única (SQLSTATE 23505) e retenta até 3 vezes.
pub async fn post_shorten(
    pool: web::Data<PgPool>,
    base_url: web::Data<String>,
    body: web::Json<ShortenRequest>,
) -> Result<HttpResponse, AppError> {
    for _ in 0..3 {
        let code = generate_code();

        let result = sqlx::query(
            "INSERT INTO urls (short_code, original_url) VALUES ($1, $2)",
        )
        .bind(&code)
        .bind(&body.url)
        .execute(pool.get_ref())
        .await;

        match result {
            Ok(_) => {
                let short_url = format!("{}/{}", base_url.get_ref(), code);
                return Ok(HttpResponse::Created().json(ShortenResponse { short_url }));
            }
            Err(ref e) => {
                // SQLSTATE 23505 = unique_violation — apenas retentar
                let is_collision = matches!(
                    e,
                    sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("23505")
                );
                if is_collision {
                    continue;
                }
                return Err(AppError::Database(result.unwrap_err()));
            }
        }
    }

    Err(AppError::CollisionExhausted)
}

/// GET /{code}
/// Busca o código no banco e retorna 302 Found com o header Location.
/// Retorna 404 se o código não existe.
pub async fn get_redirect(
    pool: web::Data<PgPool>,
    code: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let record = sqlx::query_as::<_, UrlRecord>(
        "SELECT original_url FROM urls WHERE short_code = $1",
    )
    .bind(code.as_str())
    .fetch_optional(pool.get_ref())
    .await
    .map_err(AppError::Database)?;

    match record {
        Some(r) => Ok(HttpResponse::Found()
            .insert_header(("Location", r.original_url))
            .finish()),
        None => Err(AppError::NotFound),
    }
}
