# Protocolo Rust Clean Architecture — Especificação Normativa

## Padrão oficial para projetos Rust

> Documento normativo para projetos Rust que adotem Clean Architecture com Cargo Workspaces.
> Aplicável a APIs, CLIs, workers, serviços assíncronos e bibliotecas internas.

---

## Sumário

- [1. Objetivo](#1-objetivo)
- [2. Princípios obrigatórios](#2-princípios-obrigatórios)
- [3. Estrutura obrigatória do workspace](#3-estrutura-obrigatória-do-workspace)
- [4. Dependências entre crates](#4-dependências-entre-crates)
- [5. Responsabilidades por camada](#5-responsabilidades-por-camada)
  - [5.1 Domain](#51-domain)
  - [5.2 Application](#52-application)
  - [5.3 Infrastructure](#53-infrastructure)
  - [5.4 Bootstrap](#54-bootstrap)
  - [5.5 Presentation](#55-presentation)
- [6. Ports: onde definir](#6-ports-onde-definir)
- [7. Async traits: regra obrigatória](#7-async-traits-regra-obrigatória)
  - [7.1 Estratégia A — dispatch estático](#71-estratégia-a--dispatch-estático)
  - [7.2 Estratégia B — dispatch dinâmico](#72-estratégia-b--dispatch-dinâmico)
- [8. Modelagem de tipos](#8-modelagem-de-tipos)
- [8.1 Gestão de memória e alocação eficiente](#81-gestão-de-memória-e-alocação-eficiente)
- [9. Construção de objetos](#9-construção-de-objetos)
- [10. Erros](#10-erros)
  - [10.1 Domain](#101-domain)
  - [10.2 Application](#102-application)
  - [10.3 Infrastructure](#103-infrastructure)
  - [10.4 Bootstrap e entrypoints](#104-bootstrap-e-entrypoints)
- [11. Conversões](#11-conversões)
- [12. Configuração](#12-configuração)
- [13. Observabilidade](#13-observabilidade)
- [14. Políticas de entrada](#14-políticas-de-entrada)
- [15. Lints e política de qualidade](#15-lints-e-política-de-qualidade)
- [16. Uso de `unsafe`](#16-uso-de-unsafe)
- [17. Testes](#17-testes)
- [18. Docker e build](#18-docker-e-build)
- [19. `.cargo/config.toml`](#19-cargoconfigtoml)
- [20. Checklist obrigatório de CI](#20-checklist-obrigatório-de-ci)
- [21. Proibições explícitas](#21-proibições-explícitas)
- [22. Regra de decisão arquitetural](#22-regra-de-decisão-arquitetural)
- [23. Exceções permitidas](#23-exceções-permitidas)
- [24. Resumo normativo](#24-resumo-normativo)

---

## 1. Objetivo

Este protocolo define a estrutura mínima obrigatória para projetos Rust com:

- separação clara de responsabilidades;
- baixo acoplamento entre camadas;
- composição explícita;
- invariantes de domínio modeladas no sistema de tipos;
- compatibilidade com Rust stable e edition 2024 ou superior.

Quando o projeto declarar `rust-version` ou utilizar `rust-toolchain.toml`, a versão
mínima exigida e a toolchain efetiva devem estar alinhadas com este protocolo.

---

## 2. Princípios obrigatórios

1. **O domínio é o centro do sistema.**
2. **Estados inválidos devem ser irrepresentáveis sempre que possível.**
3. **Dependências devem apontar para dentro.**
4. **IO, framework, banco, HTTP e runtime não pertencem ao domínio.**
5. **Toda composição de dependências deve ocorrer no startup.**
6. **Nenhum caso de uso pode criar suas dependências concretas internamente.**
7. **Erros de negócio não podem ser confundidos com erros técnicos.**
8. **Toda fronteira entre camadas deve ser explícita.**
9. **Conversões falíveis usam `TryFrom`; conversões infalíveis usam `From`.**
10. **Código de produção não usa `unwrap()` nem `expect()` fora de casos excepcionalmente justificados e documentados.**

---

## 3. Estrutura obrigatória do workspace

Todo projeto deve ser um Cargo Workspace com um crate por bounded context. As dependências apontam para o centro do sistema.

### Layout canônico por crate

Todo crate com lógica de negócio **DEVE** adotar o seguinte layout interno:

```text
crates/<prefixo>-<nome>/
├── Cargo.toml
└── src/
    ├── lib.rs                  # re-exports públicos do crate
    ├── domain/                 # camada interna — regras de negócio puras
    │   ├── mod.rs
    │   ├── models.rs           # entidades, value objects, aggregates
    │   ├── errors.rs           # erros tipados de domínio
    │   └── ports.rs            # traits de domínio, se houver
    ├── application/            # camada de orquestração — casos de uso
    │   ├── mod.rs
    │   ├── use_cases.rs        # orquestração de fluxos de negócio
    │   └── ports.rs            # traits de dependências externas
    └── infrastructure/         # camada externa — adapters concretos
        ├── mod.rs
        └── <adapter>.rs        # impl dos ports (db, http, fs, fila, cache)
```

### Estrutura do workspace

Todo projeto **DEVE** seguir um padrão próximo ao abaixo, onde cada bounded context é
um crate independente. Não há dependência cruzada entre bounded contexts. A composição
acontece exclusivamente no entrypoint de bootstrap, via container explícito de dependências.

```text
project_root/
  Cargo.toml                    # [workspace]; members = ["crates/*"]
  Cargo.lock
  rust-toolchain.toml           # opcional; fixa a toolchain do projeto
  Dockerfile                    # opcional
  migrations/                   # opcional; migrations do sistema
  tests/                        # integração, contratos e smoke tests
  crates/
    shared/                     # abstrações transversais reutilizadas no workspace
      Cargo.toml
      src/
        lib.rs
        domain/
        application/
        infrastructure/

    <modulo-a>/                 # bounded context A
      Cargo.toml
      src/
        lib.rs
        domain/
        application/
        infrastructure/

    <modulo-b>/                 # bounded context B
      Cargo.toml
      src/
        lib.rs
        domain/
        application/
        infrastructure/

    adapter-foo/                # opcional; adapters especializados
    worker-support/             # opcional; crates auxiliares

    server/                     # opcional; composição HTTP, bootstrap e jobs
      Cargo.toml
      src/
        lib.rs
        main.rs                 # load_config → init_telemetry → bootstrap → serve
        app_state.rs            # opcional; re-export do container de dependências
        bootstrap/
          mod.rs
          app_state.rs          # implementação do container de dependências
          dependencies.rs       # composição de adapters, clients e serviços
        router.rs               # composição de rotas
        database/               # pool, migrations e checks de schema
        errors/                 # mapeamentos de erro do entrypoint
        presentation/           # handlers/controladores por módulo
        middleware/
        config/
        security/
        seeds/                  # opcional
        services/               # opcional
        api_docs.rs             # opcional
        telemetry/
        jobs/
        test_support/
```

### Padrão de re-export por crate

Todo bounded context **DEVE** re-exportar apenas sua API pública em `lib.rs`, escondendo detalhes internos:

```rust
pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::ports::ModuloRepositoryTrait;
pub use domain::models::ModuloEntity;
pub use infrastructure::sqlx_repository::SqlxModuloRepository;
```

### Padrão do container de dependências

O container de dependências é responsável por instanciar repositórios, serviços,
clients e ports dinâmicos no startup, distribuindo-os via `Arc` ou outro mecanismo
explícito de composição:

```rust
pub struct AppState {
    pub db: Arc<PgPool>,
    pub config: Arc<AppConfig>,
    pub user_repository: Arc<UserRepositoryImpl>,
    pub order_repository: Arc<OrderRepositoryImpl>,
    pub email_sender: Arc<Box<dyn EmailSender>>,
    pub event_bus: Arc<Box<dyn EventBus>>,
}
```

### Regras estruturais

- `domain/` é obrigatório em todo crate de bounded context.
- `application/` é obrigatório em todo crate de bounded context.
- `infrastructure/` é obrigatório sempre que houver IO externo.
- Traits de repositório e serviços externos pertencem a `application/ports.rs`, não a `domain/`.
- O crate `shared` centraliza abstrações reutilizadas por múltiplos bounded contexts.
- O entrypoint de composição é o único responsável por compor bounded contexts, adapters e serviços transversais no bootstrap.
- Bounded contexts não se cruzam diretamente. Dependências entre eles passam pelo bootstrap.
- Entrypoints adicionais, como `cli`, `worker` ou `http`, são aceitáveis desde que reutilizem o mesmo padrão de composição explícita.

---

## 4. Dependências entre crates

### Grafo permitido

```text
http    ─────► application ─────► domain
cli     ─────► application ─────► domain
worker  ─────► application ─────► domain

bootstrap ───► application ─────► domain
bootstrap ───► infrastructure
bootstrap ───► domain

infrastructure ─► domain
infrastructure ─► application   # apenas para implementar ports da application
```

### Proibições de dependência

- `domain` não depende de nenhuma crate interna externa ao próprio bounded context, exceto abstrações compartilhadas estritamente necessárias.
- `application` não depende de `infrastructure`.
- `application` não depende de `http`, `cli`, `worker` ou `bootstrap`.
- `presentation` não contém regra de negócio.
- `infrastructure` não define regra de negócio.

---

## 5. Responsabilidades por camada

### 5.1 Domain

**Contém somente:**

- entidades;
- value objects;
- enums de negócio;
- invariantes;
- erros de domínio;
- comportamento puro do negócio.

**Domain não pode conter:**

- SQL;
- HTTP;
- leitura de env vars;
- cliente externo;
- serialização voltada a protocolo externo, salvo necessidade real;
- `anyhow` como erro principal de domínio.

### 5.2 Application

**Contém:**

- casos de uso;
- comandos;
- queries;
- ports;
- orquestração;
- transações de aplicação;
- erros de aplicação.

**Application não pode conter:**

- acesso direto a banco;
- acesso direto a rede;
- handlers HTTP;
- parsing de CLI;
- framework web;
- criação manual de clients concretos.

### 5.3 Infrastructure

**Contém:**

- repositórios concretos;
- clients HTTP;
- filas;
- storage;
- cache;
- parsers externos;
- implementações dos ports.

**Infrastructure não pode conter:**

- regra de negócio central;
- decisão de fluxo de caso de uso;
- validação semântica que pertença ao domínio.

### 5.4 Bootstrap

**Contém:**

- leitura e validação de configuração;
- init de tracing;
- construção de pools;
- construção de clients;
- montagem de adapters;
- composição dos casos de uso;
- container da aplicação.

**Bootstrap existe para:**

- evitar duplicação de wiring entre API, CLI e worker;
- impedir acoplamento lateral entre presentation e infrastructure.

O bootstrap também pode concentrar registries, filas, schedulers, telemetria,
health checks, migrations e dados iniciais, desde que essa orquestração não
contamine `domain` nem `application` com detalhes operacionais.

### 5.5 Presentation

**Contém:**

- handlers;
- controllers;
- rotas;
- parsing de entrada;
- serialização de saída;
- mapeamento de erros para protocolo externo.

**Presentation não pode conter:**

- SQL;
- regra de negócio complexa;
- criação de dependência concreta;
- leitura bruta de configuração;
- lógica de persistência.

---

## 6. Ports: onde definir

**Ports pertencem ao domain quando:**

- representam capacidade essencial do modelo de negócio.

**Ports pertencem ao application quando:**

- representam dependência operacional do caso de uso.

**Exemplos típicos de ports da application:**

- `UserRepository`
- `Clock`
- `IdGenerator`
- `EmailSender`
- `UnitOfWork`
- `EventPublisher`

### Regra para ports

Na dúvida, port de persistência, mensageria, relógio, geração de ID e integração externa deve ficar na `application`, não no `domain`.

---

## 7. Async traits: regra obrigatória

O projeto deve escolher explicitamente uma das duas estratégias abaixo para cada contrato assíncrono.

### 7.1 Estratégia A — dispatch estático

**Use:**

- `async fn` em traits;
- generics para injeção de dependência.

**Use esta estratégia quando:**

- o conjunto de implementações é conhecido em compilação;
- não há necessidade real de dispatch dinâmico.

### 7.2 Estratégia B — dispatch dinâmico

**Use:**

- trait object-safe;
- `async-trait` ou retorno boxed, quando necessário.

**Use esta estratégia quando:**

- for necessário `Arc<dyn Trait>`;
- implementações precisarem ser escolhidas dinamicamente em runtime.

### Regra inviolável

É proibido adotar como padrão do projeto a combinação ingênua de `async fn` em trait com `Arc<dyn Trait>` sem garantir object safety.

---

## 8. Modelagem de tipos

### Regras obrigatórias de modelagem

- Use newtypes para representar conceitos de domínio.
- Evite `String` solta quando houver semântica específica.
- Evite `bool` ambíguo em assinaturas públicas.
- Prefira enums explícitos a flags booleanas.
- Aplique `#[must_use]` onde ignorar o retorno representar risco real de bug.

### Exemplos de direção correta

- `Email` em vez de `String`
- `UserId` em vez de `Uuid` cru em APIs internas críticas
- `Compression::Enabled` em vez de `compress: bool`

### 8.1 Gestão de memória e alocação eficiente

Em caminhos de alto volume (*hot paths*) — superfícies de rede, serialização de
payloads, laços de processamento por requisição — o projeto **DEVE** minimizar
alocações desnecessárias no heap. As práticas abaixo são **obrigatórias quando
houver impacto mensurável de performance** e **opcionais em caminhos frios**, onde
clareza e simplicidade prevalecem. Nenhuma destas técnicas justifica abstração
prematura (ver [§22](#22-regra-de-decisão-arquitetural)): aplique-as onde há ganho
real, comprovado por *profiling* ou por característica conhecida do caminho quente.

A camada onde cada técnica é aceitável segue o protocolo geral: tipos e otimizações
de modelagem podem aparecer em `domain` e `application`; `Bytes`, arenas e pools de
buffer ligados a IO pertencem a `infrastructure`; `Arc` de estado compartilhado é
composto no `bootstrap` (ver [container de dependências](#padrão-do-container-de-dependências)).

#### 1. `Cow<'a, T>` — *Clone-on-Write*

Empreste dados sempre que possível e aloque memória somente quando realmente
necessário. Em vez de clonar `String` ou dados em toda requisição, use
`Cow<'a, str>` para evitar alocações em caminhos de alto volume — a alocação só
ocorre quando o dado precisa ser efetivamente mutado ou possuído.

```rust
use std::borrow::Cow;

// Só aloca quando há algo a normalizar; caso contrário, empresta.
fn normalizar(entrada: &str) -> Cow<'_, str> {
    if entrada.contains(' ') {
        Cow::Owned(entrada.replace(' ', "_"))
    } else {
        Cow::Borrowed(entrada)
    }
}
```

#### 2. `SmallVec` para coleções pequenas

Se uma coleção raramente ultrapassa poucos elementos (cabeçalhos, parâmetros de
query, *spans* temporários), use `SmallVec<[T; N]>` no lugar de `Vec<T>`. Isso
mantém os dados na pilha (*stack*) na maioria dos casos, recorrendo ao heap apenas
quando o limite `N` é excedido. `N` deve ser dimensionado pelo caso comum medido.

```rust
use smallvec::SmallVec;

// Até 8 cabeçalhos vivem na stack; acima disso, faz spill para o heap.
let mut headers: SmallVec<[Header; 8]> = SmallVec::new();
```

#### 3. `Bytes` para dados binários/JSON

Ao lidar com payloads grandes, use o tipo `Bytes` do crate homônimo. Ele permite
fatias com contagem de referências (*reference-counted*), possibilitando múltiplos
consumidores sem duplicar os dados. Pertence a `infrastructure`/`presentation` — o
`domain` não deve depender de representações de transporte.

```rust
use bytes::Bytes;

// Fatias compartilham o buffer subjacente; não há cópia do payload.
let payload: Bytes = ler_corpo().await?;
let cabecalho = payload.slice(0..16);   // mesma região de memória, +1 refcount
```

#### 4. Preferir `Arc<T>` a `.clone()` em estado compartilhado

Clones de configurações, schemas e caches **imutáveis e compartilhados** devem ser
substituídos por `Arc<T>`. Isso reduz drasticamente a pressão no alocador em
sistemas de alto volume. Alinha-se ao [container de dependências](#padrão-do-container-de-dependências),
que distribui dependências via `Arc` a partir do `bootstrap`.

```rust
// Em vez de clonar o AppConfig inteiro por requisição:
let config: Arc<AppConfig> = Arc::clone(&state.config);   // apenas +1 refcount
```

#### 5. Iteradores preguiçosos em vez de `.collect()` intermediário

Cadeias de `.map()`/`.filter()` seguidas de `.collect()` criam vetores temporários
desnecessários. Prefira iteradores preguiçosos (*lazy*), materializando apenas no
ponto de consumo final.

```rust
// Evite: aloca um Vec intermediário só para iterar de novo.
let ativos: Vec<_> = usuarios.iter().filter(|u| u.ativo).collect();
for u in &ativos { /* ... */ }

// Prefira: consome diretamente, sem alocação intermediária.
for u in usuarios.iter().filter(|u| u.ativo) { /* ... */ }
```

#### 6. Alocação por arena (ex.: `bumpalo`) para objetos temporários

Para objetos que vivem apenas durante uma única requisição, o uso de arenas
agrupa e libera todos os objetos de uma vez ao final do escopo, eliminando a
sobrecarga de desalocar cada objeto individualmente. Restrito a `infrastructure`
ou a *hot paths* específicos; nunca deve vazar tempos de vida de arena para o
`domain` ou para contratos públicos da `application`.

```rust
use bumpalo::Bump;

let arena = Bump::new();
let no = arena.alloc(NoTemporario::novo());   // liberado em massa com a arena
```

#### 7. `#[repr(transparent)]` e tipos conscientes do layout de memória

Newtypes wrapper de campo único devem usar `#[repr(transparent)]` para garantir
layout idêntico ao tipo interno, evitando conversões e cópias desnecessárias na
serialização e melhorando a localidade de cache. Combina diretamente com a
regra de newtypes de [§8](#8-modelagem-de-tipos).

```rust
#[repr(transparent)]
pub struct UserId(Uuid);   // mesmo layout de Uuid: zero custo de representação
```

#### Regra de decisão para alocação

- **Otimize o caminho quente, não o frio.** Aplique estas técnicas onde há volume
  real; em código frio, prefira a forma mais simples e legível.
- **Meça antes de complicar.** Mudanças motivadas por performance devem ter base em
  *profiling* ou em característica conhecida do caminho; não troque clareza por
  micro-otimização especulativa.
- **Respeite as fronteiras de camada.** Tipos de transporte (`Bytes`) e arenas não
  pertencem ao `domain`; `Arc` de estado compartilhado é composto no `bootstrap`.
- **`Arc` é para imutável compartilhado.** Para mutação concorrente, use o tipo de
  sincronização adequado; `Arc<T>` por si só não habilita mutação.

---

## 9. Construção de objetos

### Regras de construção

- Use `new(...)` quando a construção for simples e validável diretamente.
- Use `builder()` quando houver:
  - muitos campos opcionais;
  - combinações inválidas;
  - configuração incremental.

### Proibição de estados inválidos

Não exponha structs públicas de domínio que possam ser instanciadas livremente em estado inválido quando isso puder ser evitado.

---

## 10. Erros

### 10.1 Domain

Deve usar erros tipados com `thiserror`.

### 10.2 Application

Pode definir erros próprios de caso de uso e orquestração.

### 10.3 Infrastructure

Deve usar erros técnicos tipados ou convertê-los adequadamente ao cruzar a fronteira.

### 10.4 Bootstrap e entrypoints

Podem usar `anyhow` para:

- startup;
- composição;
- configuração;
- falhas operacionais sem benefício de tipagem fina.

### Proibições de mapeamento de erros

- Não transformar erro técnico em erro semântico de domínio por conveniência.
- Não mapear falha de banco, timeout ou erro de rede para `NotFound`, `Validation` ou equivalente sem base semântica real.

---

## 11. Conversões

### Regras obrigatórias de conversão

- `From` apenas para conversões infalíveis.
- `TryFrom` para conversões falíveis.
- DTO externo para input interno deve falhar explicitamente quando necessário.
- Domain para response pode usar `From` quando não houver falha.

### Proibição de `From` falível

É proibido implementar `From` para conversão potencialmente falível.

---

## 12. Configuração

Toda configuração obrigatória deve ser validada no startup.

### Regras de configuração

- Falhas de configuração devem interromper o startup.
- Defaults silenciosos só podem existir para valores realmente opcionais.
- Nenhuma env var obrigatória pode ser lida tardiamente de forma espalhada no código.

### Local correto

- leitura em `bootstrap`;
- consumo tipado nas demais camadas.

---

## 13. Observabilidade

Todo projeto deve inicializar tracing de forma centralizada.

### Regras de observabilidade

- usar `tracing`;
- permitir filtro por `RUST_LOG`;
- suportar saída estruturada quando aplicável;
- propagar request ID ou correlation ID nas superfícies de rede.

---

## 14. Políticas de entrada

Toda superfície de entrada deve ter política explícita para:

- timeout;
- limite de payload;
- identificação de request;
- autenticação/autorização, quando aplicável.

### Regra de exceções de entrada

O default pode ser global, mas exceções devem ser deliberadas e documentadas.

---

## 15. Lints e política de qualidade

O workspace deve centralizar lints.

**Baseline recomendada para `Cargo.toml` do workspace:**

```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
edition = "2024"
rust-version = "1.94"

[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
thiserror = "2"
tracing = "0.1"
uuid = { version = "1", features = ["v4", "serde"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal", "time"] }
# Otimização de alocação (ver §8.1); adicione por crate apenas onde houver hot path real.
bytes = "1"
smallvec = { version = "1", features = ["serde"] }
bumpalo = "3"

[workspace.lints.rust]
unsafe_code = "deny"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
indexing_slicing = "deny"
fallible_impl_from = "deny"
fn_params_excessive_bools = "warn"
must_use_candidate = "warn"

[profile.release]
lto = "thin"
strip = true
codegen-units = 1
```

O workspace pode manter uma allowlist adicional de regras Clippy quando houver justificativa técnica explícita. Toda exceção deve ser documentada e revista em PR.

**Em cada crate:**

```toml
[lints]
workspace = true
```

---

## 16. Uso de `unsafe`

### Regra para `unsafe`

`unsafe` é negado por padrão no workspace.

### Exceção

Só pode ser permitido localmente quando:

- houver necessidade técnica real;
- estiver isolado;
- estiver documentado;
- tiver justificativa explícita de segurança.

---

## 17. Testes

### Organização mínima

- unitários no módulo correspondente;
- integração e contratos em `tests/`;
- doc-tests para APIs públicas relevantes.

### Regras de teste

- prefira mocks simples;
- prefira doubles in-memory antes de framework de mock;
- casos de uso devem ser testáveis sem subir infraestrutura real, salvo testes de integração específicos;
- cenários com banco devem usar doubles, banco efêmero ou ambiente controlado, conforme o nível de realismo necessário;
- mudanças em camadas, startup, contratos públicos ou documentação de API devem manter verdes os testes de contrato correspondentes.

---

## 18. Docker e build

### Regras de build

- use multi-stage build para produção;
- não imponha `musl + scratch` como padrão universal;
- use imagem mínima de runtime quando houver objetivo claro;
- binários devem ser construídos de forma reproduzível.

O build operacional pode passar por `Dockerfile`, tasks do workspace ou scripts auxiliares, desde que o processo seja reproduzível e documentado.

### Recomendação

`cargo-chef` é aceitável como padrão de cache de dependências em imagens de build.

---

## 19. `.cargo/config.toml`

Pode ser usado para otimizações locais ou do time, como linker alternativo.

### Regra para `.cargo/config.toml`

Se `.cargo/config.toml` for introduzido, não pode conter configuração não portátil sem documentação no repositório e validação do impacto em CI e nos ambientes de desenvolvimento suportados.

---

## 20. Checklist obrigatório de CI

Todo PR com impacto no código Rust deve passar por, a partir da raiz do workspace correspondente:

```bash
cargo check --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
cargo doc --workspace --no-deps
```

Quando a mudança afetar contratos arquiteturais, endpoints ou documentação de API, os testes de contrato existentes em `tests/` também devem continuar verdes, com destaque para validações de camadas e rotas públicas.

---

## 21. Proibições explícitas

É proibido:

- colocar regra de negócio em handler HTTP;
- colocar SQL na camada de apresentação;
- fazer `application` depender de `infrastructure`;
- criar dependências concretas dentro de caso de uso;
- usar `From` em conversão falível;
- mapear erro técnico para erro semântico sem critério;
- usar `unwrap()` ou `expect()` em fluxo normal de produção;
- usar `String`, `bool` e `Option` como modelagem frouxa quando houver tipo melhor;
- tratar `async fn` em trait e `dyn Trait` como compatíveis por padrão;
- duplicar wiring entre API, CLI e worker quando isso puder ser centralizado em bootstrap.

---

## 22. Regra de decisão arquitetural

Quando houver dúvida entre duas soluções, prefira a que:

- reduz acoplamento entre camadas;
- preserva invariantes no tipo;
- deixa a composição mais explícita;
- simplifica testes;
- reduz semântica implícita;
- evita abstração prematura.

---

## 23. Exceções permitidas

Exceções a este protocolo só são aceitáveis quando todas as condições abaixo forem atendidas:

- a exceção estiver documentada no repositório;
- houver justificativa técnica explícita;
- o impacto arquitetural estiver delimitado;
- não houver alternativa mais simples aderente ao protocolo;
- a exceção não comprometer invariantes centrais do domínio.

### Exemplos de exceção aceitável

- uso localizado de `unsafe` com justificativa técnica e documentação;
- uso de `dyn Trait` por necessidade real de seleção em runtime;
- uso de `musl + scratch` em binário específico com objetivo operacional claro;
- leitura excepcional de configuração fora do bootstrap em tooling muito pequeno, desde que documentada.

### Exemplos de exceção não aceitável

- colocar SQL em handler por conveniência;
- misturar regra de negócio com serialização HTTP;
- usar `anyhow` no domínio por preguiça de tipar erro;
- usar `Arc<dyn Trait>` com trait async não object-safe sem tratamento adequado.

---

## 24. Resumo normativo

- `domain` modela o negócio.
- `application` orquestra o negócio.
- `infrastructure` implementa IO.
- `bootstrap` monta o sistema.
- `presentation` adapta protocolo externo.
- dependências apontam para dentro.
- invariantes ficam nos tipos.
- erros técnicos e de negócio não se confundem.
- traits async exigem decisão arquitetural explícita.
- composição deve ser centralizada e visível.
- caminhos de alto volume minimizam alocações (`Cow`, `SmallVec`, `Bytes`, `Arc`, iteradores preguiçosos, arenas, `#[repr(transparent)]`), sem otimização prematura em caminhos frios.
