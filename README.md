# Escalabilidade em Sistemas Web: Monólitos vs. Arquiteturas Distribuídas

> **Trabalho de Conclusão de Curso**  
> Luis Henrique de Moura Santos  
> Curso de Análise e Desenvolvimento de Sistemas — IFPI Campus Picos  
> Orientador: 

---

## Sobre o Projeto

Este repositório contém as implementações de um estudo comparativo sobre **escalabilidade em sistemas web**. O objetivo é investigar como a escolha de **arquitetura** e **linguagem de programação** afeta o desempenho de uma aplicação real sob alta carga.

O sistema escolhido como objeto experimental é um **encurtador de URLs** — simples o suficiente para ser implementado múltiplas vezes de forma equivalente, mas intensivo em I/O e sensível a latência, o que o torna ideal para isolar variáveis de desempenho.

Todas as versões expõem os mesmos dois endpoints:
- `POST /shorten` — recebe uma URL longa, retorna a URL encurtada
- `GET /{code}` — redireciona (`302`) para a URL original

---

## As Implementações

| Pasta | Arquitetura | Linguagem | Framework | Status |
|---|---|---|---|---|
| `java-aplication/` | Monolítica | Java 21 | Spring Boot + Tomcat | ✅ Pronta |
| `rust-aplication/` | Monolítica | Rust | Actix-web + Tokio | ✅ Pronta |
| `distributed-java/` | Distribuída (microserviços) | Java 21 | Spring Boot + Nginx | 🚧 Em desenvolvimento |

A versão distribuída é composta por três componentes em containers isolados:

- **API Gateway (Nginx):** ponto de entrada, roteia por método HTTP
- **Writer Service:** Spring Boot responsável pela criação e persistência de URLs
- **Reader Service:** Spring Boot focado em redirecionamento de alta vazão, com **3 réplicas** balanceadas pelo Gateway

---

## Hipóteses

1. **Rust terá menor latência de cauda (p95/p99)** — o gerenciamento determinístico de memória via *Ownership* elimina pausas de Garbage Collection.
2. **Distribuído Java superará o monólito Java em cargas extremas** — a replicação seletiva do Reader Service permitirá maior throughput próximo a 1.500 req/s.
3. **Monólitos ganham em cargas baixas** — o overhead de rede e balanceamento do Gateway penaliza a versão distribuída em baixa concorrência.
4. **Rust consumirá menos memória** — mesmo sob estresse máximo, o footprint será significativamente inferior ao das versões Java.

---

## Ambiente de Testes

- **SO:** Ubuntu 24.04.3 LTS
- **CPU:** Intel Core i5-13420H (8 núcleos / 12 threads, até 4,60 GHz)
- **RAM:** 16 GB DDR5 4800 MHz
- **Armazenamento:** SSD 512 GB M.2 PCIe Gen4

Cada container tem recursos limitados para garantir isonomia nos testes:

| Recurso | Limite | Reserva mínima |
|---|---|---|
| CPU | 0,5 vCPU | 0,25 vCPU |
| RAM | 512 MB | 256 MB |

---

## Documentação

- [Como executar as aplicações e rodar os testes de carga](docs/executando.md)

---

## Estrutura do Repositório

```
.
├── java-aplication/          # Monólito Java (Spring Boot)
├── rust-aplication/          # Monólito Rust (Actix-web)
├── distributed-java/         # 🚧 Microserviços Java (em desenvolvimento)
├── stress-test-k6.js         # Script de teste de carga (k6)
├── docs/
│   └── executando.md         # Guia de execução e testes
└── README.md
```
