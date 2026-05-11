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

- p(50): 
- p(95): 
- p(99): 
- vus_max: 
- dropped_iterations: 
- http_req_failed: 
- checks_total: 

### Resultados Java

- p(50): 
- p(95): 
- p(99): 
- vus_max: 
- dropped_iterations: 
- http_req_failed: 
- checks_total: 

---

