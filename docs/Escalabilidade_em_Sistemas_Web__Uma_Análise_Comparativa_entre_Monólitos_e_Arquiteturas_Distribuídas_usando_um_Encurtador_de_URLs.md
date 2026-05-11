# Escalabilidade em Sistemas Web: Monólitos vs. Arquiteturas Distribuídas

> Projeto de TCC — Luis Henrique de Moura Santos  
> Curso de Análise e Desenvolvimento de Sistemas — IFPI Campus Picos  
> Orientador: Prof. Esp. Jorge Luis Lima

---

## Visão Geral

Este repositório faz parte de um estudo comparativo sobre **escalabilidade em sistemas web**, desenvolvido como Trabalho de Conclusão de Curso. O objetivo central é investigar como a escolha de **arquitetura** e **linguagem de programação** afeta o desempenho de uma aplicação real sob alta carga.

O sistema escolhido como objeto experimental é um **encurtador de URLs** — simples o suficiente para ser implementado três vezes de forma equivalente, mas intensivo em I/O e sensível a latência, o que o torna ideal para isolar variáveis de desempenho.

---

## As Três Implementações

| Implementação         | Arquitetura  | Linguagem | Framework   |
|-----------------------|--------------|-----------|-------------|
| `monolith-java`       | Monolítica   | Java      | Spring Boot + Tomcat |
| `monolith-rust`       | Monolítica   | Rust      | Actix-web + Tokio    |
| `distributed-java`    | Distribuída  | Java      | Spring Boot + Nginx  |

Todas as versões expõem os mesmos endpoints:
- `POST /` — cria uma URL encurtada
- `GET /{code}` — redireciona para a URL original

---

## Arquitetura Distribuída (detalhes)

A versão distribuída é composta por três componentes em containers isolados:

- **API Gateway (Nginx):** ponto de entrada; roteia por método HTTP
- **Writer Service:** Spring Boot responsável pela criação e persistência de URLs
- **Reader Service:** Spring Boot focado em redirecionamento de alta vazão — roda com **3 réplicas** balanceadas pelo Gateway

---

## Infraestrutura de Dados

Todas as versões compartilham a mesma camada de dados para garantir isonomia nos testes:

- **PostgreSQL** como banco relacional, com índice único na coluna de códigos encurtados
- Connection pools equalizados entre as versões: **HikariCP** (Java) e **SQLx** (Rust)

---

## Ambiente de Testes

Os experimentos rodam em ambiente controlado via **Docker / Docker Compose**:

- **SO:** Ubuntu 24.04.3 LTS
- **CPU:** Intel Core i5-13420H (8 núcleos / 12 threads, até 4,60 GHz)
- **RAM:** 16 GB DDR5 4800 MHz
- **Armazenamento:** SSD 512 GB M.2 PCIe Gen4

Cada container tem recursos limitados via Docker Compose:
- **Limite:** 0,5 CPU e 512 MB RAM por instância
- **Reserva mínima:** garantida no boot do container

---

## Metodologia de Testes

A ferramenta de carga utilizada é o **[k6](https://k6.io)**, com perfil de rampa progressiva de **10 até 1.500 req/s**.

### Métricas coletadas

| Métrica     | Descrição |
|-------------|-----------|
| `p50`       | Latência mediana |
| `p95`       | Percentil 95 de latência |
| `p99`       | Percentil 99 de latência (cauda) |
| Throughput  | Requisições bem-sucedidas por segundo |
| Taxa de erro | Percentual de falhas e timeouts |

---

## Hipóteses

1. **Rust terá menor latência de cauda (p95/p99)** — o gerenciamento determinístico de memória via *Ownership* elimina pausas de Garbage Collection (*stop-the-world*).
2. **Distribuído Java superará o monólito Java em cargas extremas** — a replicação seletiva do Reader Service permitirá maior throughput próximo a 1.500 req/s.
3. **Monólitos ganham em cargas baixas** — o overhead de rede e balanceamento do Gateway penaliza a versão distribuída em cenários de baixa concorrência.
4. **Rust consumirá menos memória** — mesmo sob estresse máximo, o footprint de memória será significativamente inferior ao das versões Java.

---

## Contexto Técnico para o Claude Code

Se você está usando o Claude Code neste repositório, aqui está o que é relevante:

- **Cada pasta é uma implementação independente.** Não há código compartilhado entre elas por design — a equivalência funcional é intencional para fins de comparação justa.
- **O banco de dados é externo a todas as implementações.** O `docker-compose.yml` na raiz sobe o PostgreSQL compartilhado.
- **Os testes de carga ficam em `/load-tests/`** e são escritos em JavaScript para o k6.
- **Não altere os limites de CPU/RAM nos `docker-compose.yml`** sem atualizar a documentação de metodologia — esses valores são parte do protocolo experimental.
- **A lógica de negócio deve ser idêntica entre as versões.** Se corrigir um bug ou mudar comportamento em uma implementação, aplique nas outras três.

---

## Referências Principais

- KLEPPMANN, M. *Designing Data-Intensive Applications*. O'Reilly, 2017.
- NEWMAN, S. *Building Microservices*. O'Reilly, 2015.
- RICHARDSON, C. *Microservices Patterns*. Manning, 2018.
- KLABNIK, S.; NICHOLS, C. *The Rust Programming Language*. No Starch Press, 2018.
- TANENBAUM, A. S.; VAN STEEN, M. *Distributed Systems*. 3. ed. Pearson, 2017.