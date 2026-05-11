# Como Executar as Aplicações e os Testes de Carga

Pré-requisito único: **Docker** e **Docker Compose** instalados.  
Nenhuma linguagem, runtime ou ferramenta precisa estar instalada na máquina host.

---

## Executando as Aplicações

Cada implementação é auto-contida: sobe sua própria instância do PostgreSQL. **Rode apenas uma por vez na porta 8080.**

### Monólito Java (`java-aplication/`)

```bash
cd java-aplication

# Sobe banco + aplicação (build automático; primeira vez pode demorar alguns minutos)
docker compose up --build

# Em segundo plano
docker compose up --build -d
```

Para parar e limpar os volumes do banco:
```bash
docker compose down -v
```

---

### Monólito Rust (`rust-aplication/`)

```bash
cd rust-aplication

# Sobe banco + aplicação (build automático; primeira vez compila o Rust, pode demorar)
docker compose up --build

# Em segundo plano
docker compose up --build -d
```

Para parar e limpar os volumes do banco:
```bash
docker compose down -v
```

---

### Arquitetura Distribuída — Microserviços Java (`distributed-java/`) 🚧

> **Esta implementação ainda está em desenvolvimento.**

```bash
# TODO: comandos serão adicionados quando a implementação estiver completa
cd distributed-java

docker compose up --build -d
```

---

## Verificando se a Aplicação Subiu

Após o `docker compose up`, aguarde os containers ficarem saudáveis e teste:

```bash
# Criar uma URL encurtada
curl -X POST http://localhost:8080/shorten \
  -H "Content-Type: application/json" \
  -d '{"url": "https://www.exemplo.com/pagina-muito-longa"}'

# Resposta esperada — 201 Created:
# {"shortUrl": "http://localhost:8080/abc123"}
```

```bash
# Redirecionar (substitua abc123 pelo código retornado acima)
curl -v http://localhost:8080/abc123
# Resposta esperada: 302 Found com header Location apontando para a URL original
```

---

## Testes de Carga com k6 (via Docker)

O script `stress-test-k6.js` na raiz do repositório executa uma rampa progressiva de **10 até 1.500 req/s**, combinando escrita (`POST /shorten`) e leitura (`GET /{code}`).

### Perfil de carga

| Estágio | Duração | Taxa alvo |
|---|---|---|
| Rampa de subida | 3 min | 10 → 1.500 req/s |
| Carga sustentada | 5 min | 1.500 req/s |
| Rampa de descida | 1 min | 1.500 → 0 req/s |

### Thresholds definidos

| Métrica | Limite |
|---|---|
| Latência p50 | < 200 ms |
| Latência p95 | < 500 ms |
| Latência p99 | < 1.000 ms |
| Taxa de erros | < 5% |

---

### Rodando o teste

**1. Suba a aplicação que deseja testar** (seção acima) e aguarde estar saudável.

**2. A partir da raiz do repositório, rode o k6 via Docker:**

```bash
docker run --rm -i \
  -v "$(pwd)/stress-test-k6.js:/stress-test-k6.js" \
  grafana/k6 run /stress-test-k6.js
```

> **Por que `172.17.0.1`?**  
> O script aponta para `172.17.0.1:8080`, que é o IP do host na bridge padrão do Docker no Linux. Isso permite que o container do k6 alcance os containers da aplicação que estão ouvindo na porta `8080` do host.  
> Se estiver em macOS ou Windows, substitua `172.17.0.1` por `host.docker.internal` no arquivo `stress-test-k6.js`.

**3.** As métricas aparecem em tempo real no terminal. Ao final, o k6 exibe o resumo completo com p50, p95, p99, throughput e taxa de erros.

---

### Salvando os resultados em JSON

```bash
docker run --rm -i \
  -v "$(pwd)/stress-test-k6.js:/stress-test-k6.js" \
  -v "$(pwd)/resultados:/resultados" \
  grafana/k6 run --out json=/resultados/resultado.json /stress-test-k6.js
```

Os arquivos de resultado ficam na pasta `resultados/` na raiz do repositório.

---

## Anotações Importantes

- **Não altere os limites de CPU/RAM** nos `docker-compose.yml` sem atualizar a metodologia — esses valores (`0.5 CPU / 512 MB`) fazem parte do protocolo experimental.
- **A lógica de negócio deve ser idêntica entre as versões.** Se corrigir um bug em uma implementação, aplique nas outras.
- Cada implementação tem seu banco independente. Para comparação justa, sempre suba uma versão por vez e limpe os volumes (`docker compose down -v`) entre os testes.
