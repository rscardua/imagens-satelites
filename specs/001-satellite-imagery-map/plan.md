# Implementation Plan: Visualização de Imagens Recentes de Satélite CBERS-4A no Mapa

**Branch**: `001-satellite-imagery-map` | **Date**: 2026-06-23 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-satellite-imagery-map/spec.md`

## Summary

Aplicação web do monorepo que busca cenas recentes de satélite e as exibe num mapa interativo. Backend Rust (Clean Architecture, Cargo Workspace) expõe um endpoint de busca que faz **proxy ao vivo** de dois provedores externos — INPE STAC (CBERS-4A, primário) e NASA GIBS (WMTS, alternativo) — sem persistência durável; só cache em memória de curta TTL dos metadados de busca. Frontend Vue 3 + Leaflet renderiza footprints clicáveis (GeoJSON), faz overlay do raster (preview) da cena priorizada e mostra a fonte NASA como camada de tiles. Acesso público na v1.

## Technical Context

**Language/Version**: Backend Rust stable, edition 2024, `rust-version = 1.94`. Frontend TypeScript 6.x / Vue 3.5+ / Vite 8+ (Node 22+).

**Primary Dependencies**:
- Backend: Axum 0.8, Tokio 1, `reqwest` (cliente HTTP dos provedores, rustls), `serde`/`serde_json`, `thiserror` 2, `tracing` + `tower-http`, `async-trait` (ports object-safe), `moka` (cache em memória TTL), `geo`/`geojson` (footprints/bbox), `anyhow` (somente bootstrap).
- Frontend: Leaflet (mapa, `GeoJSON`, `ImageOverlay`, `TileLayer` WMTS), Pinia 3, Axios, Zod, Tailwind v4.

**Storage**: N/A durável na v1 (FR-015). Estado transitório: cache em memória de curta TTL (metadados de busca). Sem PostgreSQL/PostGIS nesta feature.

**Testing**: Backend `cargo test` — unitários nos módulos, contratos/integração em `tests/` com servidor HTTP mock (`wiremock`) simulando STAC/GIBS; doubles in-memory para `Clock`/`ImageryCache`. Frontend Vitest (unit/integração de use-cases e parsing) + Cypress (component/E2E do mapa).

**Target Platform**: Backend Linux server (container). Frontend navegador moderno desktop (v1).

**Project Type**: Web application (monorepo `apps/server-rust` + `apps/client`).

**Performance Goals**: Primeira imagem visível em ≤10 s a partir da abertura (SC-001). Timeout por provedor configurável (default 8 s). Cache TTL curto (default 60 s) para reduzir chamadas redundantes durante navegação.

**Constraints**: Dependências apontam para dentro (`presentation`/`infrastructure` → `application` → `domain`); `application` nunca depende de `infrastructure`. Sem `unwrap()`/`expect()` em produção. `unsafe` negado. Erros de domínio/app com `thiserror`; `anyhow` só no bootstrap. Acesso público (sem auth) na v1. Limite de resultados por busca (default 50) e orientação para refinar área ampla (FR-011).

**Scale/Scope**: v1 de exploração, baixa concorrência, sem escrita concorrente. 1 bounded context (`imagery`), 1 endpoint público de busca + endpoints de proxy de assets/tiles, 1 página de mapa no frontend.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

> O `constitution.md` está como template não ratificado. As normas vinculantes da feature são `docs/rust_clean_arch_guidelines.md` (backend) e `docs/ESTRUTURA_PROJETO.md` (estrutura/frontend), conforme `CLAUDE.md`. Gates derivados delas:

| Gate (origem) | Status | Como o plano atende |
|---|---|---|
| Workspace com crate por bounded context (§3) | PASS | `crates/shared`, `crates/imagery`, `crates/server`. |
| Camadas `domain`/`application`/`infrastructure` por crate (§3, §5) | PASS | `imagery` com as três camadas; `server` faz presentation+bootstrap. |
| `application` não depende de `infrastructure` (§4, §21) | PASS | Port `ImageryProvider` na `application`; adapters STAC/GIBS na `infrastructure`. |
| Ports de integração externa na `application`, não no `domain` (§6) | PASS | `ImageryProvider`, `ImageryCache`, `Clock` em `application/ports.rs`. |
| Decisão explícita de async trait (§7) | PASS | Estratégia B (dispatch dinâmico): trait object-safe via `async-trait`, `Arc<dyn ImageryProvider>` — seleção de provedor em runtime. Justificada em research.md. |
| Newtypes/estados inválidos irrepresentáveis (§8, §9) | PASS | `BBox`, `DateRange`, `CloudCover`, `SourceId`, `SceneId` validados na construção. |
| Erros tipados; `anyhow` só bootstrap (§10) | PASS | `thiserror` em `imagery`; `anyhow` só em `server/main.rs` e composição. |
| `From` infalível / `TryFrom` falível (§11) | PASS | DTO externo→domínio via `TryFrom`; domínio→response via `From`. |
| Composição no bootstrap via `Arc` (§5.4, §3) | PASS | `AppState` monta providers, cache e use case. |
| `unsafe` negado; lints workspace (§15, §16) | PASS | `[workspace.lints]` herdado; `unsafe_code = "deny"`. |
| Políticas de entrada: timeout/limite/request-id (§14) | PASS | `tower-http` timeout/limit/trace; correlation id propagado. |
| Sem regra de negócio em handler / sem IO no domínio (§21) | PASS | Priorização/seleção de cena no use case; handler só adapta HTTP. |

**Resultado**: PASS — sem violações. Complexity Tracking vazio.

## Project Structure

### Documentation (this feature)

```text
specs/001-satellite-imagery-map/
├── plan.md              # Este arquivo
├── research.md          # Phase 0
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1
├── contracts/           # Phase 1 (HTTP API + contrato de port)
│   ├── imagery-api.openapi.yaml
│   └── imagery-provider-port.md
└── tasks.md             # Phase 2 (/speckit-tasks — não criado aqui)
```

### Source Code (repository root)

```text
apps/server-rust/
├── Cargo.toml                       # [workspace]; members = ["crates/*"]; lints centralizados
├── crates/
│   ├── shared/
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── domain/              # AppError transversal, tipos geo base (BBox)
│   │       ├── application/
│   │       └── infrastructure/      # http client base, telemetry helpers
│   ├── imagery/                     # bounded context único da feature
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs               # re-exports públicos
│   │       ├── domain/
│   │       │   ├── mod.rs
│   │       │   ├── models.rs        # Scene, Footprint, SourceId, CloudCover, DateRange
│   │       │   └── errors.rs        # ImageryDomainError (thiserror)
│   │       ├── application/
│   │       │   ├── mod.rs
│   │       │   ├── use_cases.rs     # SearchRecentImagery (priorização/limite)
│   │       │   ├── ports.rs         # ImageryProvider, ImageryCache, Clock
│   │       │   └── errors.rs        # ImageryAppError
│   │       └── infrastructure/
│   │           ├── mod.rs
│   │           ├── inpe_stac.rs     # adapter STAC CBERS-4A
│   │           ├── nasa_gibs.rs     # adapter GIBS (WMTS template + capabilities)
│   │           └── moka_cache.rs    # ImageryCache em memória TTL
│   └── server/
│       └── src/
│           ├── main.rs              # load_config → telemetry → bootstrap → serve
│           ├── app_state.rs
│           ├── bootstrap/
│           │   ├── mod.rs
│           │   └── dependencies.rs  # compõe providers (Arc<dyn>), cache, use case
│           ├── router.rs            # rotas + camadas globais (cors, timeout, trace)
│           ├── config/
│           ├── telemetry/
│           ├── errors/              # mapeamento ImageryAppError → HTTP
│           └── presentation/
│               └── imagery/
│                   └── handler.rs   # GET /api/imagery/search, proxy assets/tiles
└── tests/
    ├── imagery_search_contract.rs   # contrato do endpoint (wiremock STAC/GIBS)
    └── provider_adapters.rs         # integração dos adapters

apps/client/
└── src/
    ├── @core/
    │   └── imagery/
    │       ├── domain/
    │       │   ├── entities/        # SceneEntity, MapBounds, DateRange
    │       │   └── repository/      # ImageryRepositoryInterface
    │       ├── application/
    │       │   └── use-case/        # SearchImageryUseCase
    │       └── infra/
    │           └── api/             # ImageryApiRepository (Axios) + Zod schemas
    ├── composables/
    │   └── useSatelliteMap.ts       # init Leaflet, camadas, footprints, overlay
    ├── components/
    │   └── imagery/                 # SourceSelector, DateRangeFilter, SceneInfoPanel
    └── pages/
        └── Mapa/
            └── Index.vue            # página do mapa (injeta SearchImageryUseCase)
```

**Structure Decision**: Web application no monorepo. Backend em `apps/server-rust` como Cargo Workspace com um bounded context `imagery` (layout canônico §3 das guidelines) + `shared` + `server` (presentation/bootstrap). Frontend em `apps/client` seguindo o `@core` (domain/application/infra) da ESTRUTURA_PROJETO, com a página de mapa na camada de apresentação injetando o use case. Sem camada de persistência durável nesta feature.

## Complexity Tracking

> Sem violações de gate. Nada a justificar.
