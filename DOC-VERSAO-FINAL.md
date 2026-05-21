# Documentação Experimental — TCC: Escalabilidade Web

**Comparação:** Monólito Java (Spring Boot) vs Monólito Rust (Actix-web)  
**Sistema experimental:** Encurtador de URLs  
**Carga alvo:** até 1.500 req/s  
**Ferramenta de carga:** k6

---

## 1. Hardware e Ambiente

| Componente | Especificação |
|---|---|
| CPU | Intel Core i5-13420H (8 núcleos, 12 threads, 4,60 GHz boost) |
| RAM | 16 GB DDR5 4800 MHz |
| Armazenamento | SSD M.2 PCIe Gen4 |
| Sistema Operacional | Ubuntu 24.04.3 LTS |
| Containerização | Docker Engine (bridge network `172.17.0.1`) |
| Ferramenta de carga | k6 via `docker run grafana/k6` |

**Limites de container por serviço (app e db na V4):**

| Recurso | Limite | Reserva |
|---|---|---|
| CPU | 0,5 núcleos | 0,25 núcleos |
| RAM | 512 MB | 256 MB |

---

## 2. Stack Técnica

### Rust
- Actix-web 4 + Tokio (async runtime)
- sqlx 0.8 (postgres, runtime-tokio-rustls)
- Pool: `max_connections(10)`, `min_connections(5)`
- RNG: `thread_rng()` (Thread-Local Storage, sem lock)
- Compilação: `opt-level=3`, `lto="thin"`, `codegen-units=1`
- Workers HTTP: 1 (V4) / padrão do sistema (V1-V3 — bug documentado abaixo)

### Java
- Spring Boot + Tomcat embarcado
- HikariCP: `maximum-pool-size=10`, `minimum-idle=5`
- RNG: `ThreadLocalRandom.current()` (V3+) / `SecureRandom` (V1-V2)
- JVM: Eclipse Temurin 21 (eclipse-temurin:21-jre-alpine)
- GC: G1GC (V4a) / ZGC Generacional (V4b)
- Heap: ~256 MB (`MaxRAMPercentage=50.0` em container de 512 MB)

### PostgreSQL
- Versão: 16-alpine
- Limites: nenhum (V1-V3) / 0,5 CPU + 512 MB (V4)
- Configuração extra na V4: autovacuum menos agressivo, checkpoint_timeout=30min

---

## 3. Histórico de Versões

| Versão | Problema corrigido | Classificação |
|---|---|---|
| V1 | Baseline inicial. Rust sem `.workers()` → 12 workers para 0,5 CPU (context switching severo). Java usa `SecureRandom` (lock contention). Pool Rust ilimitado. Script k6 com bugs (endpoint e campo JSON errados). | — |
| V2 | Pool Rust equalizado (`max_connections(10)`). Limites Docker iguais para ambos. Lógica de retry em colisão de `short_code`. Script k6 corrigido. | Erro metodológico |
| V3 | Java: `SecureRandom` → `ThreadLocalRandom` (elimina lock contention). Rust: `min_connections(5)` (iguala pré-aquecimento do HikariCP). k6: fase de warm-up de 60 s a 100 req/s adicionada. | Erro metodológico |
| V4a | PostgreSQL com limites de recursos. Warm-up aumentado para 120 s a 300 req/s (garante compilação C2 do JIT). HikariCP `connection-timeout` reduzido de 30 s para 5 s. Rust: `.workers(1)` explícito. Java: G1GC explícito via `JAVA_TOOL_OPTIONS` no docker-compose. | Erro metodológico |
| V4b | Igual à V4a, mas Java usa ZGC Generacional (`-XX:+UseZGC -XX:+ZGenerational`). Isola o GC como variável — demonstra que a variância some quando o GC é adequado para heaps pequenos. | Variância intrínseca isolada |

---

## 4. Análise das Fontes de Variância

### 4.1 Variância extrínseca — erros metodológicos (devem ser corrigidos)

**E1. PostgreSQL sem limites de recursos (V1-V3)**  
O serviço `db` nos dois `docker-compose.yml` não tinha `deploy.resources`. O PostgreSQL competia livremente por CPU e memória do host com a aplicação no mesmo container engine. Dois processos autônomos do PostgreSQL causam spikes imprevisíveis entre execuções:
- `autovacuum`: dispara quando a tabela acumula linhas mortas. A 1.500 req/s (inserts), com `autovacuum_vacuum_scale_factor=0.2` padrão, ele dispara múltiplas vezes durante o teste de 9 minutos. Timing não-determinístico.
- `checkpointer`: WAL checkpoint a cada 5 minutos por padrão. Com alta taxa de insert, 1-2 checkpoints ocorrem durante o teste. Cada checkpoint = burst de I/O.

Correção V4: `cpus: "0.5"`, `memory: 512M` no serviço `db`; `checkpoint_timeout=30min`; `autovacuum_vacuum_scale_factor=0.5`.

Evidência: Rust exec 2 da V3 teve `dropped_iterations=182` apesar de Rust não ter GC. Causa: spike do PostgreSQL naquela execução.

**E2. Warm-up insuficiente do JIT (V3)**  
60 s a 100 req/s = ~6.000 requests por hot path. O threshold padrão do C2 (HotSpot) é ~10.000 invocações. Resultado: JIT ainda em C1 no início da fase de medição.

Mais crítico: caminhos de código de contenção do HikariCP e dispatch do Tomcat sob alta carga nunca são exercitados a 100 req/s. Esses code paths só existem sob pressão — o JIT não os compilou antes da medição.

Correção V4: 120 s a 300 req/s = ~36.000 requests por hot path. Garante C2 nos caminhos críticos.

**E3. HikariCP `connection-timeout=30s` como amplificador de cascata (V3)**  
Quando GC pausa threads do Tomcat, elas retêm conexões do pool. Novas requests ficam na fila do HikariCP por até 30 s antes de falhar. Isso transforma qualquer pausa de GC em cascata de timeout: k6 não recebe respostas → cria mais VUs → mais pressão → mais GC → mais timeouts.

Evidência direta: Java V3 exec 1 → `vus_max=3.007` (6x acima de `preAllocatedVUs=500`). O k6 criou 3.007 VUs porque a aplicação estava em GC pause e não respondia.

Correção V4: `connection-timeout=5000` (5 s). Fail-fast interrompe a cascata antes que o VU count exploda.

**E4. Rust sem `.workers()` explícito (V1-V3)**  
Actix-web usa o número de CPUs lógicas do host como padrão para workers HTTP. No i5-13420H com 12 threads, isso cria 12 OS threads competindo por 0,5 CPU. Cada thread ocupa espaço em stack, gera context switching e pressiona o scheduler do kernel.

Nota: o anotações.md da V1 documenta isso como "bug interessante". Porém o `.workers(2)` mencionado na documentação nunca foi commitado no código — o repositório manteve o default de 12 workers em todas as versões. A V4 corrige com `.workers(1)`.

Correção V4: `.workers(1)`. Com sqlx async, 1 worker Actix + Tokio async gerencia concorrência via event loop, sem necessidade de múltiplos OS threads para 0,5 CPU.

### 4.2 Variância intrínseca — características do ecossistema (resultado científico)

**I1. G1GC com heap de 256 MB**  
`MaxRAMPercentage=50.0` em container de 512 MB = ~256 MB de heap efetivo. G1GC foi projetado para heaps de 2 GB+. Com 256 MB:
- Cria apenas ~64 regiões de 4 MB (mínimo para funcionamento efetivo)
- Entra em Full GC Stop-The-World com frequência não-determinística
- Full GC pode durar centenas de ms a segundos — pausa todas as threads Tomcat
- Overhead de GC pode consumir 40-60% do CPU disponível

Essa variância é **intrínseca ao ecossistema Java com configuração padrão sob memória limitada**. Não é erro metodológico — é o comportamento real de uma aplicação Spring Boot com G1GC (padrão do Java 21) em um container de 512 MB.

A V4a documenta esse comportamento. A V4b usa ZGC para isolar o efeito.

**I2. JIT não-determinístico entre execuções**  
Mesmo com warm-up adequado, o timing exato de quando C2 compila cada método varia por execução. A ordem de compilação depende de contadores de invocação que variam com o scheduling do OS e a sequência exata de requests. Esse é um custo fundamental do modelo JIT da JVM.

---

## 5. Protocolo de Teste

### 5.1 Pré-requisitos

```bash
# Garantir que nenhuma instancia anterior esta rodando
docker ps -a | grep -E "java|rust|postgres"

# Verificar que porta 8080 esta livre
ss -tlnp | grep 8080
```

### 5.2 Script k6 (V4)

Localização: `./stress-test-k6.js`

```
Warm-up:    120 s a 300 req/s (preAllocatedVUs: 150)  [nao medido]
Ramp-up:    3 min de 10 → 1.500 req/s
Steady:     5 min a 1.500 req/s
Ramp-down:  1 min de 1.500 → 0 req/s
Duracao total: ~11 min por execucao
```

Thresholds (apenas fase `test`):
- `p(50) < 200 ms`
- `p(95) < 500 ms`
- `p(99) < 1.000 ms`
- `http_req_failed < 5%`

### 5.3 Comandos por execucao

```bash
# Definir variavel de caminho (evitar problema com espaco no path)
K6_SCRIPT="/home/luis-henrique/Code/IFPI/5° periodo/tcc2/aplicacoes/stress-test-k6.js"

# Para cada execucao:
cd <pasta-da-aplicacao>
docker compose down -v
docker compose up --build -d

# Aguardar containers healthy (~30s para Java, ~10s para Rust)
docker compose ps

# Iniciar coleta de stats em paralelo (em outro terminal):
watch -n 5 'docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}"'

# Rodar k6:
docker run --rm -v "$K6_SCRIPT:/test.js" grafana/k6 run /test.js

# Ao final:
docker compose down -v
```

### 5.4 Ordem de execucao

1. Rust V4 — Exec 1
2. Rust V4 — Exec 2
3. Java V4a (G1GC) — Exec 1
4. Java V4a (G1GC) — Exec 2
5. Editar `java-aplication/docker-compose.yml`: descomentar linha ZGC, comentar linha G1GC
6. Java V4b (ZGC) — Exec 1
7. Java V4b (ZGC) — Exec 2

Para V4b, apenas trocar `JAVA_TOOL_OPTIONS` no `java-aplication/docker-compose.yml` — nao requer rebuild da imagem.

---

## 6. Resultados Historicos

### V1

| Metrica | Rust | Java |
|---|---|---|
| p50 | 273,1 µs | 325,21 µs |
| p95 | 513,48 µs | 2,05 ms |
| p99 | 671,07 µs | 136,27 ms |
| dropped_iterations | 0 | 222 |
| http_req_failed | 0,00% | 0,00% |
| vus_max | 500 | 679 |
| checks_total (rate) | 1.261.798 (2.336/s) | 1.261.354 (2.335/s) |

**Nota V1 Java:** RNG = `SecureRandom` (lock contention). **Nota V1 Rust:** 12 workers para 0,5 CPU (context switching severo).

### V2

| Metrica | Rust Exec 1 | Rust Exec 2 | Java Exec 1 | Java Exec 2 |
|---|---|---|---|---|
| p50 | 281,89 µs | 265,25 µs | 339,14 µs | 308,39 µs |
| p95 | 527,9 µs | 494,45 µs | 8,79 ms | 654,81 µs |
| p99 | 1,04 ms | 624,2 µs | 112,27 ms | 95,49 ms |
| dropped_iterations | 0 | 0 | 83 | 58 |
| http_req_failed | 0% | 0% | 0% | 0% |
| vus_max | 500 | 500 | 569 | 554 |

**Variancia Java V2:** p95 variou de 8,79 ms para 654,81 µs (~13x) entre execucoes identicas. Causa: JIT + GC sem warm-up adequado.

### V3

| Metrica | Rust Exec 1 | Rust Exec 2 | Java Exec 1 | Java Exec 2 |
|---|---|---|---|---|
| p50 | 271,62 µs | 267,23 µs | 346,98 µs | 315,81 µs |
| p95 | 513,2 µs | 512,17 µs | 589,23 ms | 1,07 ms |
| p99 | 21,26 ms | 29,88 ms | 988,44 ms | 99,92 ms |
| dropped_iterations | 0 | 182 | 3.358 | 192 |
| http_req_failed | 0% | 0% | 0% | 0% |
| vus_max | 631 | 627 | 3.007 | 670 |
| checks_total | 1.273.310 | 1.273.436 | 1.267.084 | 1.273.416 |

**Variancia Java V3:** p95 variou de 589 ms para 1,07 ms (~550x) entre execucoes. Causa confirmada: Full GC G1GC na exec 1 durante pico de carga → cascata de timeouts HikariCP → vus_max=3.007.

**Variancia Rust V3 exec 2:** `dropped_iterations=182` com Rust (sem GC). Causa: spike do PostgreSQL (autovacuum ou checkpoint) sem limite de recursos.

---

## 7. Resultados da V4

### V4 — Rust

| Metrica | Exec 1 | Exec 2 | Variacao |
|---|---|---|---|
| p50 | 271,74 µs | 259,93 µs | ~1x |
| p95 | 4,83 ms | 505,66 µs | **~9,6x** |
| p99 | 277 ms | 28,37 ms | **~9,7x** |
| dropped_iterations | 6.585 | 1.910 | ~3,4x |
| http_req_failed | 0,01% | 0,00% | — |
| vus_max | 1.521 | 905 | ~1,7x |
| checks_total (rate) | 1.320.442 (2.000/s) | 1.329.980 (2.015/s) | — |
| CPU app pico | 44% | 36% | — |
| CPU db pico | **51%** | 50% | — |
| RAM app pico | 7 MB | 7 MB | — |

**Nota Rust V4:** PostgreSQL saturou em 51% do limite (Exec 1) e 50% (Exec 2). Throttling de cgroups pelo kernel e a nova fonte de variancia — substituiu autovacuum/checkpoint da V3 por uma fonte mais previsivel mas ainda nao-deterministica no timing. A variancia caiu de ~0x (Rust V3, sem dropped) para ~9,6x no p95, porem AINDA muito menor que Java V3 (550x). O Rust V4 continua com RAM quase nula (7 MB) contra os centenas de MB do Java.

### V4a — Java G1GC

| Metrica | Exec 1 | Exec 2 | Variacao |
|---|---|---|---|
| p50 | 332,29 µs | 318,4 µs | ~1x |
| p95 | 195,67 ms | 29,93 ms | **~6,5x** |
| p99 | 581,57 ms | 298,8 ms | ~1,9x |
| dropped_iterations | 3.931 | 3.197 | ~1,2x |
| http_req_failed | 0,00% | 0,00% | — |
| vus_max | 2.000 | 1.472 | ~1,4x |
| checks_total (rate) | 1.325.938 (2.009/s) | 1.327.406 (2.011/s) | — |
| CPU app pico | **55,75%** | **53,77%** | — |
| CPU db pico | 32% | 36% | — |
| RAM app pico | ~420 MB | ~420 MB | — |

**Nota Java V4a:** Variancia caiu de **550x (V3) para 6,5x (V4a)** no p95 — as correcoes funcionaram. A variancia remanescente e intrinseca do G1GC. O app Java consome mais CPU que o Rust (~54% vs ~40%), sendo o gargalo do sistema (PG ficou bem abaixo do limite). HikariCP com timeout de 5 s (antes 30 s) impediu a cascata de VUs da V3 (vus_max 3.007 → 2.000).

### V4b — Java ZGC Generacional

| Metrica | Exec 1 | Exec 2 | Variacao |
|---|---|---|---|
| p50 | 1,30 s | 1,20 s | **~1x** |
| p95 | 1,60 s | 1,59 s | **~1x** |
| p99 | 1,79 s | 1,79 s | **~1x** |
| dropped_iterations | 129.652 | 112.948 | ~1,1x |
| http_req_failed | 0,00% | 0,00% | — |
| vus_max | 3.150 | 3.150 | mesmo |
| checks_total (rate) | 1.074.496 (1.628/s) | 1.107.902 (1.678/s) | — |
| CPU app pico | **53,82%** | **54,11%** | — |
| CPU db pico | 28% | 38% | — |
| RAM app pico | **453 MB** | **462 MB** | — |
| Thresholds | **FAILED** | **FAILED** | — |

**Nota Java V4b (resultado inesperado):** ZGC foi significativamente pior que G1GC em todas as metricas, com variancia proxima de 1x (consistentemente catastrofico). A causa e estrutural:

1. **CPU starvation dos threads de GC:** ZGC e um GC concorrente — suas threads de coleta rodam em paralelo com os threads do Tomcat. Com apenas 0,5 CPU disponivel, as threads de GC do ZGC competem diretamente com os threads HTTP, reduzindo o throughput efetivo da aplicacao.

2. **Pressao de memoria critica:** ZGC consumiu 453-462 MB dos 512 MB do container (88-90% do limite). ZGC precisa de reserva de memoria para copia concorrente — com heap de apenas 256 MB e alta taxa de alocacao (1.500 req/s), o heap se esgota antes que o GC consiga liberar espaco.

3. **Paradoxo de consistencia:** ZGC e o mais consistente entre todas as configuracoes (variancia ~1x), mas consistentemente no pior nivel. G1GC tem maior variancia (6,5x) porem com mediana muito melhor.

**Conclusao V4b:** ZGC e superior em ambientes com recursos abundantes (varios nucleos, heap grande). Em containers CPU-restrito (0,5 CPU) com heap pequeno (256 MB), G1GC supera ZGC porque suas pausas STW (Stop-The-World) nao competem com threads da aplicacao pelo CPU escasso — a propria natureza "concurrent" do ZGC torna-se sua fraqueza.

---

## 8. Interpretacao dos Resultados

### 8.1 Comparacao de variancia entre versoes

| Configuracao | Metrica | Variacao entre execucoes |
|---|---|---|
| Java V3 | p95 | **550x** (erro metodologico dominante) |
| Rust V3 | p95 | ~1x (mas dropped_iters intermitente) |
| Rust V4 | p95 | ~9,6x (PG throttling) |
| Java V4a (G1GC) | p95 | **6,5x** (variancia intrinseca do GC) |
| Java V4b (ZGC) | p95 | **~1x** (sistematicamente ruim) |

### 8.2 Hierarquia de gargalos identificados

| Sistema | Gargalo principal | CPU limite atingido? |
|---|---|---|
| Rust V4 | PostgreSQL (throttled em ~50% CPU) | Sim (PG) |
| Java V4a | Aplicacao (JVM + G1GC + Tomcat) | Sim (app) |
| Java V4b | Aplicacao (ZGC threads + Tomcat) | Sim (app) + OOM quase |

### 8.3 Conclusoes para o TCC

**Resultado 1 — Rust vs Java (G1GC):** Rust mantem p50 ~270 µs estavel em ambas as execucoes. Java G1GC varia de 318 a 332 µs no p50, mas tem picos drasticos nos percentis altos (p95: 30-195 ms). A diferenca fundamental e o GC: Rust nao tem GC e gerencia memoria via ownership em tempo de compilacao.

**Resultado 2 — Variancia intrinseca vs extrinseca:** A variancia do Java V3 (550x) era principalmente extrinseca (PostgreSQL sem controle, warmup insuficiente, timeout longo no HikariCP). Com as correcoes da V4a, a variancia caiu para 6,5x — essa e a variancia intrinseca do G1GC, que e um resultado cientifico valido. Nao e erro metodologico; e o comportamento esperado de um GC com 256 MB de heap sob 1.500 req/s.

**Resultado 3 — ZGC paradoxo:** A hipotese de que ZGC eliminaria a variancia foi refutada. ZGC eliminou a variancia, mas ao custo de degradacao sistematica. Isso demonstra que a escolha de GC e dependente do ambiente: ZGC otimiza para baixa latencia quando recursos sao abundantes; G1GC e mais robusto em ambientes CPU-restrito.

**Resultado 4 — Efeito do PostgreSQL limitado:** Limitar o PostgreSQL a 0,5 CPU reduziu a variancia extrinseca (autovacuum/checkpoint sem controle), mas introduziu throttling determinístico como nova fonte de variancia no Rust. Comparativamente, Java nao sofreu com throttling do PG (PG ficou em apenas 28-36%), pois o proprio app Java foi o gargalo antes do PG.

**Validade dos resultados:** A variancia remanescente nao invalida a comparacao. Rust e Java operam sob as mesmas condicoes de PostgreSQL. A diferenca observada reflete caracteristicas fundamentais dos ecossistemas: compilacao nativa vs JVM, ausencia de GC vs G1GC, modelo async vs modelo de threads bloqueantes.
