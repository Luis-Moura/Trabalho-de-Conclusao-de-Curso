## Primeira Versão

Na primeira versão, o .workers(2) no Rust não havia sido definido, o que causava um efeito interessante, pois o rust não tem conciencia de container, por isso ao iniciar, o Actix-web criou 12 workers de alta performance, prontos para processar requisições em paralelo. Com isso tinham cerca de 15 a 20 threads do Rust altamente ativas tentando processar as 1.500 requisições simultâneas. Como o Docker cortava o acesso à CPU na metade do tempo de um único núcleo, o sistema operacional entrava em colapso tentando ser "justo" com todas as threads. Ele pausava a Thread 1 para dar um microssegundo para a Thread 2, pausava a 2 para a 3, e assim por diante. Esse processo de pausar e retomar threads chama-se Context Switching (Troca de Contexto). Ou talvez tudo isso seja um azar do rust por ter rodado em um momento e hora errada KKKKKKKk

Outro ponto interessante é que nessa primeira verão o Rust teve uma vantagem injusta na geração do *short_code*, pois a função thread_rng() do Rust aloca o gerador de números aleatórios diretamente no contexto da thread atual (Thread-Local Storage - TLS). Ou seja, cada thread do Tokio tem o seu próprio gerador particular. Não existe lock, não existe fila, não existe contenção. Além disso, a macro *gen_range* é altamente otimizada. Já no Java o *SecureRandom* no Java é thread-safe. Isso significa que, para evitar que duas threads leiam o mesmo estado, ele utiliza locks (sincronização) internos. Quando o k6 injeta 1.500 requisições por segundo, centenas de threads do Tomcat tentam chamar random.nextBytes() ao mesmo tempo. O resultado é que elas formam uma fila, esperando a liberação do lock. Isso gera um bloqueio massivo de CPU (lock contention) que prejudica severamente a latência e o throughput do Spring Boot.

Resultados Rust:
- Rust p(50): 273.1µs
- Rust p(95): 513.48µs
- Rust p(99): 671.07µs
- vus_max: 500
- dropped_iterations: 0
- http_req_failed: 0.00%
- checks_total: 1261798 - 2336.656141/s

Resultados Java
- Rust p(50): 325.21µs
- Rust p(95): 2.05ms
- Rust p(99): 136.27ms
- vus_max: 679
- dropped_iterations: 222
- http_req_failed: 0.00%
- checks_total: 1261354 2335.833963/s

---

## Segunda Versão

Na segunda versão, o objetivo foi eliminar as vantagens injustas que distorceram os resultados da V1, tornando a comparação mais honesta entre as duas tecnologias.

### Correções aplicadas

**1. Pool de conexões equalizado**

Na V1, o Rust não tinha limite explícito de conexões ao banco, enquanto o Spring Boot usa HikariCP com `maximum-pool-size=10` por padrão. Agora o Rust define explicitamente `max_connections(10)` no `PgPoolOptions`, igualando a pressão exercida sobre o PostgreSQL por ambas as aplicações.

**2. Limites de CPU e memória iguais no Docker**

Ambos os containers agora rodam com os mesmos recursos alocados:
- `cpus: "0.5"` (meio núcleo)
- `memory: 512M` (reserva: 256M / 0.25 CPU)

Na V1, a ausência de limites no Rust permitia que ele consumisse CPU irrestrita, enquanto o Java já tinha restrições naturais do JVM. Isso criava uma comparação injusta de hardware.

**3. Lógica de colisão de `short_code`**

Ambas as aplicações agora implementam retry loop com até 3 tentativas em caso de colisão de chave única (`SQLSTATE 23505` no Rust / `DataIntegrityViolationException` no Java). Na V1 uma colisão provavelmente resultava em erro 500.

**4. Script k6 corrigido**

O script de teste tinha dois bugs que comprometiam a medição real:
- Endpoint errado (faltava `/shorten` na URL do POST)
- Campo errado na leitura do JSON de resposta (`shortUrl` vs `url`)

### O que permanece diferente (intencional)

A diferença fundamental de geração de `short_code` foi mantida, pois é parte do comportamento natural de cada ecossistema:
- **Rust**: `thread_rng()` — RNG por thread (Thread-Local Storage), sem lock, sem contenção
- **Java**: `SecureRandom` — thread-safe com sincronização interna, gerando contenção sob alta carga

### Resultados Rust

- p(50): 281.89µs - 265.25µs
- p(95): 527.9µs - 494.45µs
- p(99): 1.04ms - 624.2µs
- vus_max: 500
- dropped_iterations: 0
- http_req_failed: 0
- checks_total: 1261797 - 1261798

### Resultados Java 2x

- p(50): 339.14µs - 308.39µs
- p(95): 8.79ms - 654.81µs
- p(99): 112.27ms - 95.49ms
- vus_max: 569 - 554
- dropped_iterations: 83 - 58
- http_req_failed: 0
- checks_total: 1261682

---

## Observação: Não-Determinismo nos Resultados

Uma observação crítica ao longo dos testes: **os resultados variam significativamente entre execuções da mesma imagem Docker, sem nenhuma alteração de código ou configuração.** Isso é especialmente pronunciado no Java.

Exemplo concreto nos resultados da V2 (Java, 2 execuções):
- p(95): **8.79ms** vs **654.81µs** → diferença de ~13x
- p(99): **112.27ms** vs **95.49ms**
- dropped_iterations: **83** vs **58**

O Rust, por contraste, apresentou variação muito menor entre execuções (p95: 527.9µs vs 494.45µs).

### Por que isso acontece no Java?

**1. JVM JIT Compilation (principal causa)**

A JVM não executa bytecode diretamente — ela usa compilação em camadas (tiered compilation):
- **Nível 0**: Interpretação pura (lenta)
- **Nível 1-3**: Compilador C1 (rápido, pouca otimização)
- **Nível 4**: Compilador C2 (lento de compilar, muito otimizado)

Quando o k6 começa a injetar carga, a JVM ainda está coletando dados de profiling. Durante esse "aquecimento", os mesmos métodos têm latências diferentes. Se o teste começa antes do JIT atingir o nível 4 em caminhos críticos (handler HTTP, serialização JSON, pool HikariCP), os percentis altos explodem. O timing exato em que o C2 assume varia por execução.

**2. Garbage Collector (GC)**

Java gerencia memória via GC. Pausas de GC (mesmo com G1GC ou ZGC) são imprevisíveis em timing — dependem da pressão de memória no momento exato do teste. Uma execução pode completar sem pause-the-world; outra pode sofrer um GC completo justo no meio da carga, causando spike nos percentis altos.

**3. SecureRandom lock contention**

Já documentado na V1: `SecureRandom` usa lock interno. A quantidade exata de contenção varia por scheduling de threads do OS em cada execução. Em condições de alta carga (1.500 req/s), pequenas diferenças no scheduling produzem grandes diferenças em filas de lock.

**4. HikariCP connection pool warm-up**

O pool de conexões do HikariCP inicializa as conexões de forma lazy e ajusta seu tamanho dinamicamente. A primeira execução pode criar conexões sob carga, adicionando latência irregular.

**5. Estado do PostgreSQL**

O buffer pool do Postgres (shared_buffers) fica mais quente a cada execução. A primeira execução pode ter mais cache misses no banco; a segunda aproveita dados já em memória.

### Por que o Rust é mais estável?

- **Sem JIT**: código nativo gerado em tempo de compilação — comportamento é o mesmo da primeira à última requisição
- **Sem GC**: memória gerenciada via ownership, sem pausas imprevisíveis
- **Sem lock no RNG**: `thread_rng()` usa Thread-Local Storage, sem contenção
- **Comportamento determinístico desde a primeira requisição**

### Implicação para o TCC

Resultados de benchmark Java (e JVM em geral) requerem **descarte das primeiras execuções** (warm-up) e **múltiplas execuções** para calcular médias confiáveis. Comparar uma execução Java "fria" com Rust que é sempre "quente" é uma fonte de viés. Para resultados justos, o ideal seria rodar o k6 com uma fase de warm-up separada antes do período de medição real.

---

## Terceira Versão

Objetivo: eliminar as três fontes de viés identificadas na observação acima — RNG assimétrico, ausência de warm-up medido da JVM e pool de conexões assimétrico.

### Correções aplicadas

**1. Java: `SecureRandom` → `ThreadLocalRandom`**

`SecureRandom` foi substituído por `ThreadLocalRandom.current()` no `UrlShortenerService.generateCode()`. `ThreadLocalRandom` é o equivalente Java exato de `thread_rng()` do Rust: gerador por-thread (Thread-Local Storage), sem lock, sem contenção sob carga. Não é criptograficamente seguro — igual ao comportamento do Rust. Elimina a assimetria de contenção que inflava artificialmente a latência Java.

**2. k6: fase de warm-up não medida (60s)**

O script k6 ganhou um cenário `warmup` separado (60s a 100 req/s) que roda antes do cenário `test`. As métricas são separadas por tag `{ phase: "warmup" }` vs `{ phase: "test" }`, e os thresholds só se aplicam à fase `test`. Isso garante que a JVM já passou pelo ciclo C1→C2 de compilação JIT antes de qualquer medição começar, eliminando o spike de latência das primeiras execuções.

**3. Rust: `min_connections(5)` no pool sqlx**

Adicionado `.min_connections(5)` ao `PgPoolOptions` no `main.rs`, igualando o comportamento do HikariCP com `minimum-idle=5` do Java. Ambos os pools agora pré-aquecem 5 conexões ao banco na inicialização, eliminando o overhead de criação de conexão sob carga inicial.

### O que permanece diferente (intencional)

- **Runtime**: Rust compila para binário nativo; Java roda na JVM. Essa diferença é a essência do estudo.
- **Framework HTTP**: Actix-web (async/Tokio) vs Spring Boot (Tomcat/threads). Também intencional.
- **Modelo de memória**: ownership/zero GC no Rust vs GC no Java. Intencional.

### Mudanças nos arquivos

| Arquivo | Mudança |
|---------|---------|
| `java-aplication/src/main/java/.../service/UrlShortenerService.java` | `SecureRandom` removido; `generateCode()` usa `ThreadLocalRandom.current()` |
| `stress-test-k6.js` | Cenário `warmup` adicionado; cenário `test` com `startTime: "60s"`; thresholds filtradas por `phase:test` |
| `rust-aplication/src/main.rs` | `.min_connections(5)` adicionado ao `PgPoolOptions` |

### Resultados Rust

- p(50): 271.62µs
- p(95): 513.2µs
- p(99): 21.26ms
- vus_max: 631
- dropped_iterations: 0 
- http_req_failed: 0
- checks_total: 1273310

### Resultados Java

- p(50): 346.98µs
- p(95): 589.23ms
- p(99): 988.44ms
- vus_max: 3007
- dropped_iterations: 3358
- http_req_failed: 0
- checks_total: 1267084

---

