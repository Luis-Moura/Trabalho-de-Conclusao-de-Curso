use sqlx::PgPool;

/// Cria a tabela e o índice único na primeira execução.
/// Idempotente: usa IF NOT EXISTS em ambas as instruções.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS urls (
            id           BIGSERIAL    PRIMARY KEY,
            short_code   VARCHAR(10)  NOT NULL,
            original_url TEXT         NOT NULL,
            created_at   TIMESTAMP    NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_urls_short_code ON urls (short_code)",
    )
    .execute(pool)
    .await?;

    Ok(())
}
