use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;

mod db;
mod handlers;
mod models;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let base_url = std::env::var("APP_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let port: u16 = std::env::var("APP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("APP_PORT must be a valid port number");

    // Pool equivalente ao HikariCP: max 10 conexões, 5 pré-aquecidas (igual ao minimum-idle).
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    db::run_migrations(&pool)
        .await
        .expect("Failed to run database migrations");

    let pool     = web::Data::new(pool);
    let base_url = web::Data::new(base_url);

    println!("Listening on 0.0.0.0:{port}");

    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .app_data(base_url.clone())
            .route("/shorten",  web::post().to(handlers::post_shorten))
            .route("/{code}",   web::get().to(handlers::get_redirect))
    })
    .workers(1)
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
