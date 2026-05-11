# ── Build stage ───────────────────────────────────────────────────────────────
FROM rust:alpine AS builder

# musl-dev: cabeçalhos da libc musl para linking estático
RUN apk add --no-cache musl-dev

WORKDIR /app

# 1. Copiar apenas os manifestos para cache de dependências
COPY Cargo.toml ./

# 2. Compilar dependências com um main.rs fictício
#    Se o Cargo.lock existir no contexto, será usado para reprodutibilidade
COPY Cargo.loc[k] ./
RUN mkdir src && \
    echo 'fn main() {}' > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/deps/rust_aplication*

# 3. Compilar o código-fonte real
COPY src ./src
RUN cargo build --release

# ── Runner stage ──────────────────────────────────────────────────────────────
FROM alpine:3.20 AS runner

# Usuário não-root (mesmo padrão da aplicação Java)
RUN addgroup -S appgroup && adduser -S appuser -G appgroup
USER appuser
WORKDIR /app

COPY --from=builder /app/target/release/rust-aplication .

EXPOSE 8080
CMD ["./rust-aplication"]
