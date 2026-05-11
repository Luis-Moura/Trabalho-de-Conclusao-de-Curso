CREATE TABLE IF NOT EXISTS urls (
    id           BIGSERIAL PRIMARY KEY,
    short_code   VARCHAR(10)  NOT NULL,
    original_url TEXT         NOT NULL,
    created_at   TIMESTAMP    NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_urls_short_code ON urls (short_code);
