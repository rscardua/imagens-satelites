# Estrutura, Tecnologias e Padrões


## Referências Normativas

O repositório é governado por **`.specify/memory/constitution.md`**:
- Para **backend Rust**, as regras normativas de arquitetura, composição, tipos, erros, observabilidade e CI estão em `docs/rust_clean_arch_guidelines.md`.
- Para **frontend**, a referência estrutural é esta documentação (`docs/ESTRUTURA_PROJETO.md`).
- Para **permissões e segurança** (RBAC/RLS), ver `docs/seguranca-permissoes.md`.
- Em caso de conflito, a constituição prevalece.

---

## 1. Panorama do monorepo
- **Gerenciamento**: monorepo com duas aplicações principais em `apps/` e documentação em `docs/`; os comandos de desenvolvimento são executados por aplicação, não por scripts agregadores na raiz.
- **Requisitos**:
  - Node.js 22+ (exigido para o frontend e tooling via `engines` no package.json).
  - Rust estável (exigido para compilar o backend em `apps/server-rust`).
- **Estrutura de pastas**:
  - `apps/client`: Frontend Vue.js SPA com kernel compartilhado consolidado localmente em `src/@core/@shared`.
  - `apps/server-rust`: Backend Rust com Axum/SQLx (backend principal).
  - `docs/`: Documentação e artefatos auxiliares.

## 2. Tecnologias e Dependências

### 2.1 Front-end (`apps/client`)
- **Core**: Vue 3.5+, Vite 8+, TypeScript 6.x.
- **Estilização**: Tailwind CSS v4, plugins locais em `src/tailwind/`, Sass, `tw-animate-css`, Reka UI.
- **Estado**: Pinia 3.x com `pinia-plugin-persistedstate`.
- **Componentes**: DevExtreme Vue (Grids, Charts), TipTap (Editor), GridStack, Lucide Vue Next e `@iconify/vue`.
- **Comunicação**: Axios.
- **Validação**: Zod.
- **Infra complementar**: `vite-plugin-mkcert`, `vite-plugin-fs`, `workbox-window`, `markdown-it`, DOMPurify.
- **Testes**: Vitest (Unit/Integration), Cypress (Component/E2E).

### 2.2 Back-end Rust (`apps/server-rust`)
- **Core**: Rust stable edition 2024, Tokio 1, Axum 0.8.
- **Banco de Dados**: SQLx com PostgreSQL e migrations SQLx.
- **API Docs**: Utoipa + Scalar API Reference.
- **Observabilidade**: `tracing`, `tower-http`, OpenTelemetry OTLP.
- **Segurança**: JWT, OAuth2, middlewares explícitos de autenticação e RBAC.
- **Assincronismo**: Tokio runtime, scheduler com `tokio-cron-scheduler`, SSE com broadcast channel.
- **Exportação**: `rust_xlsxwriter`.
- **Integrações geoespaciais e dados**: adapters para ArcGIS, GeoServer, OGC, PostgreSQL/PostGIS e ClickHouse.
- **Testes**: `cargo test`, testes unitários e de integração do crate.

### 2.3 Núcleos Compartilhados
- **Frontend (`apps/client/src/@core/@shared`)**: concentra `BaseEntity`, Notification, contratos genéricos de repositório, validações Zod e a infraestrutura base de HTTP (`ApiRepository` e `HttpClient`).
- **Backend (`apps/server-rust/crates/shared/src`)**: concentra traits e DTOs transversais, `CrudUseCases`, `SearchParams`, `SqlxRepository<E>` e adaptadores reutilizáveis.
- **Diretriz**: novos contratos compartilhados DEVEM nascer nesses núcleos; não há pacote separado `packages/common` no estado atual do projeto.

## 3. Arquitetura e Padrões de Código

### 3.1 Padrão "Onion" / Clean Architecture (Implementação Detalhada)
O projeto aplica rigorosamente a Clean Architecture, distribuindo as camadas entre os pacotes do monorepo para maximizar o compartilhamento de código de domínio e garantir a separação de responsabilidades.

#### A Camada de Domínio do Client
Localizada em `apps/client/src/@core`, é o núcleo agnóstico consumido pelo frontend.
- **Responsabilidade**: Define as regras de negócio, entidades e contratos usados pelo client.
- **Estrutura Interna por Módulo** (ex: `acidente-trabalho/domain`):
  - `entities/`: Classes de Entidade (ex: `AcidenteTrabalhoEntity`) que estendem `BaseEntity`.
  - `rules/`: Regras de negócio puras e validadores customizados.
  - `repository/`: Interfaces de repositório (ex: `AcidenteTrabalhoRepositoryInterface`) que o `infra` deve implementar.
- **Compartilhamento**: Exportada localmente pelo próprio client.

#### A Camada de Aplicação e Infraestrutura (Client vs Server)
O client consome seus próprios contratos locais; o backend mantém suas próprias camadas de aplicação e infraestrutura.

| Camada | Frontend (`apps/client/src/@core`) | Backend Rust (`apps/server-rust/crates/<modulo>/src`) |
| :--- | :--- | :--- |
| **Application** | Contém Use Cases focados na UX (ex: validação de form, chamada de API). <br> `use-case/`: Classes executáveis `execute(input)`. | Contém Use Cases focados em orquestração (ex: transação de banco, disparo de e-mail). <br> `application/use_cases.rs`: Lógica de orquestração. |
| **Infra** | Implementações focadas em HTTP/Browser. <br> `api/`: Repositórios que estendem `ApiRepository` e chamam o backend via Axios. | Implementações focadas em Banco de Dados/Serviços. <br> `infrastructure/sqlx_repository.rs`: Repositórios SQLx que implementam os ports de aplicação. |
| **Domain** | **Localizado em `apps/client/src/@core`**. O cliente importa as Entidades e Interfaces do próprio workspace. | **Por crate**. Cada bounded context mantém seus contratos em `domain/models.rs`, `domain/errors.rs` e `domain/ports.rs`. |

### 3.2 Estrutura de Pastas do Cliente (`apps/client/src`)
- `@core/`: O "coração" da lógica do frontend.
  - `<modulo>/`: (ex: `usuario`, `acidente-trabalho`)
    - `application/use-case/`: Casos de uso específicos do front (ex: `FindByIdAcidenteTrabalhoUseCase`).
    - `infra/api/`: Implementação de repositórios que comunicam com a API (ex: `AcidenteTrabalhoApiRepository`).
- `components/`: componentes reutilizáveis da UI, incluindo navegação, layout e wrappers de componentes base.
- `layouts/`: layouts de composição de página e cascas de navegação.
- `helpers/`, `utils/`, `services/` e `composables/`: funções auxiliares, integrações transversais e serviços de apoio à UI.
- `pages/`: Camada de Apresentação. Cada página injeta Use Cases do `@core` para buscar dados, desconhecendo se vêm de uma API ou mock.
- `router.ts`: guards de autenticação/navegação.
- `routes.ts`: catálogo declarativo de rotas, metadados de menu, ícones e descrições.
- `store/`: Gerenciamento de estado global (Pinia) para dados transitórios (Sessão, UI state).
- `tailwind/`: plugins e extensões de utilitários do design system atual.
- `__tests__/`: testes de integração/infra da aplicação frontend.

### 3.3 Estrutura de Pastas do Servidor Rust (`apps/server-rust`)
O backend Rust é organizado como um Cargo workspace. Cada bounded context é um crate próprio em `crates/<modulo>/`. O crate `server` concentra toda a camada HTTP e o bootstrap da aplicação.

#### Crate `crates/server/src/`
- `main.rs`: bootstrap do processo HTTP.
  - Carrega config.
  - Inicializa telemetria.
  - Garante a existência do banco configurado na ENV antes de abrir o pool principal.
  - Abre pool PostgreSQL.
  - Executa migrations por padrão em todo startup; só deixa de executar quando `DATABASE_RUN_MIGRATIONS=false`.
  - Sincroniza recursos/operações no banco a partir do catálogo real de rotas.
  - Executa seeds iniciais de admin/perfis quando necessário.
  - Audita rotas esperadas versus tabela `recurso`.
  - Inicializa scheduler e sobe o `axum::serve`.
- `app_state.rs`: estado compartilhado da aplicação.
  - Contém todas as dependências da aplicação com injeção explícita via `Arc` para compartilhamento seguro entre handlers.
  - Expõe pool, config, repositórios, serviços JWT/OAuth/local auth, e-mail e event bus.
- `router.rs`: composição da árvore HTTP.
  - Agrupa rotas públicas, rotas do usuário autenticado e rotas protegidas por RBAC.
  - Publica a documentação OpenAPI em `/api/openapi.json` e a interface Scalar em `/api` (`/api/` redireciona para o endpoint oficial).
  - Aplica camadas globais de CORS, compressão, trace, métricas, rate limit e security headers.
- `presentation/`: camada HTTP.
  - Cada módulo expõe `router(api_prefix)` e `resource_routes(api_prefix)`.
  - `handler.rs` contém handlers Axum, DTOs `serde` e metadados OpenAPI.
  - `shared/crud.rs` contém a macro de CRUD que gera handlers, rotas e catálogo de recursos de forma padronizada.
  - `shared/routes.rs` centraliza a estrutura `ManagedRoute`, usada para sincronizar `operacao`, `recurso` e `operacao_recurso`.
- `middleware/`: middlewares explícitos do Axum.
  - `auth.rs`: resolve ou exige token e injeta `AuthenticatedUser` no request.
  - `rbac.rs`: consulta recursos/operações/permissões e injeta contexto de auditoria e filtro RLS.
- `security/`: auditoria de endpoints, rate limit e headers de segurança.
- `telemetry/`: tracing, OTLP e middleware de métricas HTTP.
- `jobs/`: scheduler e tarefas de background.
- `seeds/`: sincronização de recursos do backend e seed inicial de dados administrativos.

#### Crates de bounded context (`crates/<modulo>/src/`)
Cada bounded context segue o layout canônico obrigatório:
- `lib.rs`: re-exports públicos do crate.
- `domain/`: entidades (`models.rs`), erros tipados (`errors.rs`), traits de domínio puro (`ports.rs`).
- `application/`: orquestração de casos de uso (`use_cases.rs`) e traits de repositório/serviços externos (`ports.rs`).
- `infrastructure/`: implementações concretas dos ports, ex: `sqlx_repository.rs`.

#### Crate `crates/shared/src/`
Centraliza abstrações transversais reutilizadas por todos os bounded contexts:
- `domain/`: trait `Entity`, `RepositoryTrait<E>`, `UseCase<I,O>`, `AppError`.
- `application/`: `CrudUseCases`, DTOs compartilhados, `SearchParams`.
- `infrastructure/`: `SqlxRepository<E>`, email, events, xlsx.

### 3.4 Arquitetura do Backend Rust
O backend Rust implementa a arquitetura com uma modelagem explícita e previsível, evitando reflexão ou “mágica” em runtime.

#### Camadas principais
- **Presentation**: Axum handlers, routers, DTOs HTTP, OpenAPI e Scalar API Reference.
- **Application**: Casos de uso e orquestração entre domínio, repositórios e serviços.
- **Domain**: Entidades, contratos de repositório, regras e erros de negócio.
- **Infrastructure**: SQLx, providers externos, telemetria, exportação, scheduler.

#### Injeção de dependência
- No Rust, a composição acontece em `AppState`, que é montado no bootstrap e compartilhado por `clone` barato via `Arc`.
- O desenvolvedor consegue rastrear facilmente onde cada dependência nasce e onde é consumida.

#### Organização dos módulos
- Módulos CRUD padronizados podem ser declarados com a macro de `presentation/shared/crud.rs`.
- Essa macro gera automaticamente:
  - handlers CRUD,
  - rotas HTTP Axum,
  - catálogo de recursos para sincronização no banco.
- Módulos com rotas customizadas, como autenticação, histórico de aceite e logs, expõem manualmente suas `resource_routes`.

#### Contrato de auditoria e timestamps
- O backend Rust possui contrato para auditoria e timestamps, centralizando a regra na camada de repositório SQLx.
- Em toda criação de registro:
  - `created_at` deve ser preenchido automaticamente no server;
  - `updated_at` deve ser preenchido automaticamente no server;
  - `operacao_usuario_id` deve ser preenchido a partir do usuário autenticado no token.
- Em toda atualização de registro:
  - `created_at` deve permanecer com o valor original retornado do banco;
  - `updated_at` deve ser sobrescrito automaticamente no server com o horário corrente;
  - `operacao_usuario_id` deve ser sobrescrito com o usuário autenticado da requisição.
- Essas regras valem para todas as entidades persistidas, sem exceções.
- A aplicação dessas regras não deve ficar distribuída entre handlers ou casos de uso. Handlers apenas repassam o contexto autenticado, e o repositório garante o preenchimento final antes da persistência.
- Como o frontend opera em timezone local, o formato persistido e retornado pelo backend Rust deve permanecer compatível com essa expectativa, evitando divergência prática de interpretação entre client e server.

#### Responsabilidade por camada
- `middleware/auth.rs` e `middleware/rbac.rs` resolvem o usuário autenticado e o contexto de auditoria da requisição.
- `presentation/` traduz HTTP para DTOs e repassa o `audit_user_id`, sem decidir timestamps manualmente.
- `application/` orquestra casos de uso e regras de negócio, sem assumir responsabilidade técnica de auditoria.
- `crates/shared/src/infrastructure/sqlx_repository.rs` é o ponto único para aplicar `created_at`, `updated_at` e `operacao_usuario_id` antes do `insert`, `insert_many`, `update` e `update_many`.
- `domain/` continua definindo entidades e contratos, mas a garantia operacional de auditoria pertence à infraestrutura de persistência.

### 3.5 Sincronização Automática de Rotas no Backend Rust
O backend Rust agora possui um mecanismo de sincronização automática de recursos e operações.

#### Fonte única de verdade
- Cada módulo HTTP declara suas rotas em dois níveis:
  - `router(api_prefix)`: monta os endpoints reais no Axum.
  - `resource_routes(api_prefix)`: descreve os recursos que devem existir no banco.
- O agregador em `presentation/mod.rs` consolida todos os `resource_routes(api_prefix)` do sistema.

#### Como funciona no startup
- `main.rs` garante primeiro que a database configurada exista; se não existir, ela é criada antes da conexão principal.
- `main.rs` executa migrations por padrão antes do restante do bootstrap.
- Quando `DATABASE_RUN_MIGRATIONS=false`, o startup exige que o schema base já exista; caso contrário, falha com erro explícito.
- `main.rs` executa a sincronização de recursos antes dos seeds iniciais.
- O módulo `seeds/mod.rs` faz `upsert` em:
  - `operacao`
  - `recurso`
  - `operacao_recurso`
- Isso permite que uma rota nova seja cadastrada no banco ao subir a aplicação, sem depender de base vazia.

#### O que é automático e o que ainda é explícito
- **Automático**:
  - CRUDs declarados com a macro compartilham a mesma definição para HTTP e banco.
  - Seeds e auditoria passam a ler o catálogo real de rotas do backend Rust.
  - O cadastro de nova rota já entra no banco em startups subsequentes por meio de `upsert`.
- **Explícito**:
  - Rotas customizadas ainda precisam declarar seu `resource_routes(api_prefix)` no módulo correspondente.
  - Isso substitui reflexão implícita por uma declaração fortemente tipada e rastreável.

#### Vantagens para manutenção
- Elimina a lista estática paralela de recursos no seed.
- Reduz drift entre rota implementada e recurso cadastrado.
- Permite revisar o mapa de autorização diretamente no código do módulo.
- Mantém a sincronização idempotente e segura para rodar em todo startup.

### 3.6 O Núcleo Compartilhado (`@shared`)
O diretório `@core/@shared` atua como um "Kernel" que unifica o comportamento do sistema.

#### No Cliente (`apps/client/src/@core/@shared`)
- **Application**:
  - `dto/`, `presenter/`, `use-case/`: DTOs, presenters e casos de uso compartilhados entre módulos.
- **Domain**:
  - `entity/`: `BaseEntity` e tipos fundamentais das entidades do frontend.
  - `notification/`: Implementação do padrão Notification para validações acumuladas.
  - `repository/`: interfaces genéricas (`RepositoryInterface`, `SearchParams`, `SearchResult`) reutilizadas pelos módulos.
- **Validation**:
  - `validation/`: schemas, adapters e validators reutilizáveis (incluindo validações BR e helpers Zod).
- **Infra**:
  - `api/api.repository.ts`: Classe base abstrata `ApiRepository<E>`. Ela implementa `RepositoryInterface` e traduz chamadas de método (`insert`, `update`, `search`) para requisições HTTP Axios padronizadas.
  - `http/`: Wrapper do Axios (`HttpClient`) com interceptors para tratamento de tokens e erros globais.
  - `storage/`: abstrações de storage para integração com Pinia e browser.
  - `types.ts`: tipos compartilhados de infraestrutura para filtros, paginação e operações de busca.

#### No Servidor Rust (`apps/server-rust/crates/shared/src`)
- **Infra**:
  - `infrastructure/`: `SqlxRepository<E>` e outros adaptadores base que implementam os traits compartilhados usando SQLx, permitindo que o domínio de cada crate permaneça agnóstico à persistência.

## 4. Fluxos de Trabalho e Convenções

### 4.1 Desenvolvimento
- **Comandos do frontend**: executar a partir de `apps/client` (`pnpm dev`, `pnpm build`, `pnpm lint`, `pnpm test -- --run`).
- **Comandos do backend**: executar a partir de `apps/server-rust` (`pnpm dev`, `pnpm lint`, `pnpm test`, além dos comandos `cargo` diretos do workspace).
- **Linting**: ESLint 10+ configurado em `apps/client/eslint.config.js`; Clippy com `-D warnings` no backend.
- **Commits**: Seguir convenção de commits para versionamento automático (se aplicável).

### 4.2 Navegação e Menus
- A estrutura de menus é dinâmica e nasce em `apps/client/src/routes.ts`.
- `routes.ts` concentra paths, nomes, ícones, descrições e metadados como `subMenuName`, `menuHidden`, `footer`, `publico` e `isPublic`.
- `router.ts` aplica guards de autenticação, redirecionamento de rotas públicas e fallback para `not-found`.
- Sidebars e menus derivam a visibilidade das rotas combinando metadados de navegação com `store.checkPermission(...)`.

### 4.3 Integração Client-Server
- O Frontend consome a API REST do Backend.
- Tipagem compartilhada agora vive no próprio client, em `apps/client/src/@core/@shared`.
- A autenticação suporta múltiplas estratégias configuradas pelo backend, incluindo `local`, `keycloak` e `google` no estado atual.
- O frontend gerencia tokens e refresh tokens em interceptors do Axios e na store global, preservando a separação entre UI, casos de uso e infraestrutura.

---

## 5. Sistema de Permissões e Segurança

> Esta seção foi extraída para um documento próprio:
> **[seguranca-permissoes.md](seguranca-permissoes.md)** (modelo RBAC/RLS, lógica
> de autenticação/autorização no backend Rust e frontend, contrato de permissões
> e checklist de segurança).

---

**Observação**: Este arquivo foi revisado em Abril de 2026 para refletir a estrutura atual do monorepo, o frontend Vue 3 em `apps/client` e o backend Rust consolidado em `apps/server-rust`.
