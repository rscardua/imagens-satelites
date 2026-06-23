---

description: "Task list — Visualização de Imagens Recentes de Satélite CBERS-4A no Mapa"
---

# Tasks: Visualização de Imagens Recentes de Satélite CBERS-4A no Mapa

**Input**: Design documents from `/specs/001-satellite-imagery-map/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: INCLUÍDOS — exigidos pelos cenários de aceitação da spec e pela §17/§20 das guidelines (contratos verdes na CI).

**Organization**: Tarefas agrupadas por user story (P1→P3) para entrega incremental independente.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência pendente)
- **[Story]**: US1/US2/US3 (fases de story)

## Path Conventions

- Backend: `apps/server-rust/` (Cargo workspace; crates `shared`, `imagery`, `server`; testes em `apps/server-rust/tests/`)
- Frontend: `apps/client/src/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Inicialização do workspace e do projeto frontend.

- [x] T001 Criar Cargo workspace em `apps/server-rust/Cargo.toml` (`members = ["crates/*"]`, `resolver=2`, `[workspace.package]` edition 2024 / rust-version 1.94, `[workspace.dependencies]` e `[workspace.lints]` com `unsafe_code="deny"` e clippy pedantic conforme §15)
- [x] T002 [P] Adicionar `apps/server-rust/rust-toolchain.toml` (toolchain stable) e `apps/server-rust/.cargo/config.toml` se necessário (documentado)
- [x] T003 [P] Inicializar projeto frontend `apps/client/package.json` (Vue 3.5, Vite 8, TS 6, `engines` node 22+) + deps Leaflet, Pinia, Axios, Zod, Tailwind v4
- [x] T004 [P] Configurar tooling frontend: `apps/client/eslint.config.js`, Vitest e Cypress em `apps/client/` (scripts `dev`/`build`/`lint`/`test`)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Infraestrutura central que TODAS as stories dependem (camadas, bootstrap, ports, scaffolding front).

**⚠️ CRITICAL**: Nenhuma story começa antes desta fase.

- [x] T005 [P] Criar crate `shared` em `apps/server-rust/crates/shared/src/` (`lib.rs`, `domain/` com tipo geo base `BBox` e `AppError` transversal, `[lints] workspace=true`)
- [x] T006 Criar esqueleto do crate `imagery` em `apps/server-rust/crates/imagery/` (`Cargo.toml` com `serde`, `thiserror`, `async-trait`, `chrono`, `geojson`, `tracing`; `src/lib.rs` re-exports; `domain/`, `application/`, `infrastructure/` com `mod.rs` vazios)
- [x] T007 Criar esqueleto do crate `server` em `apps/server-rust/crates/server/src/` (`main.rs` fluxo load_config→telemetry→bootstrap→serve; `config/` com leitura+validação de env no startup conforme §12 e quickstart.md; `anyhow` só aqui)
- [x] T008 [P] Implementar telemetria em `apps/server-rust/crates/server/src/telemetry/` (`tracing` + `RUST_LOG`, saída estruturada, correlation id)
- [x] T009 [P] Implementar `apps/server-rust/crates/server/src/router.rs` com camadas globais (`tower-http` CORS, TimeoutLayer, RequestBodyLimitLayer, TraceLayer) e montagem do prefixo `/api`
- [x] T010 Implementar mapeamento de erros em `apps/server-rust/crates/server/src/errors/` (`ImageryAppError`→HTTP: NoImagery→200 vazio, AreaTooLarge→422, Unavailable→502, Timeout→504, com `ErrorBody`/`suggested_source` por contrato)
- [x] T011 Definir ports em `apps/server-rust/crates/imagery/src/application/ports.rs` (`ImageryProvider` object-safe via `#[async_trait]`, `ImageryCache`, `Clock`, `ProviderError`, `ProviderResponse`) conforme contracts/imagery-provider-port.md
- [x] T012 [P] Implementar `ImageryCache` (moka, TTL curto) em `apps/server-rust/crates/imagery/src/infrastructure/moka_cache.rs`
- [x] T013 [P] Implementar `Clock` de sistema em `apps/server-rust/crates/imagery/src/infrastructure/system_clock.rs`
- [x] T014 Implementar container de dependências em `apps/server-rust/crates/server/src/bootstrap/dependencies.rs` e `app_state.rs` (registry `HashMap<SourceId, Arc<dyn ImageryProvider>>`, cache, use case via `Arc`)
- [x] T015 [P] Scaffolding domínio frontend em `apps/client/src/@core/imagery/domain/` (`entities/SceneEntity.ts`, `entities/MapBounds.ts`, `entities/DateRange.ts`, `repository/ImageryRepositoryInterface.ts`)
- [x] T016 [P] Implementar `apps/client/src/@core/imagery/infra/api/ImageryApiRepository.ts` (base Axios/HttpClient) + schemas Zod de `SearchResult`/`Scene`
- [x] T017 [P] Inicializar mapa base em `apps/client/src/composables/useSatelliteMap.ts` (Leaflet + camada base) e shell `apps/client/src/pages/Mapa/Index.vue` + rota em `routes.ts`

**Checkpoint**: Fundação pronta — stories podem começar.

---

## Phase 3: User Story 1 - Visualizar imagens recentes no mapa (Priority: P1) 🎯 MVP

**Goal**: Abrir o mapa e ver footprints + overlay da cena CBERS-4A mais recente da área, com metadados.

**Independent Test**: Abrir numa área com cobertura → ≥1 cena recente sobreposta com data; área sem cobertura → mensagem clara.

### Tests for User Story 1 ⚠️ (escrever primeiro, devem FALHAR)

- [x] T018 [P] [US1] Teste de contrato do endpoint em `apps/server-rust/tests/imagery_search_contract.rs` (wiremock STAC: cenas, vazio→200, 5xx→502, timeout→504)
- [x] T019 [P] [US1] Teste de integração do adapter em `apps/server-rust/tests/provider_adapters.rs` (`InpeStacProvider` parsing STAC→Scene, item inválido descartado)
- [x] T020 [P] [US1] Teste Vitest em `apps/client/src/@core/imagery/__tests__/SearchImageryUseCase.spec.ts` (use case + parsing Zod do payload)

### Implementation for User Story 1

- [x] T021 [P] [US1] Newtypes/entidades em `apps/server-rust/crates/imagery/src/domain/models.rs` (`BBox` com `area_deg2`, `DateRange`, `CloudCover`, `SourceId`, `SceneId`, `Sensor`, `Footprint`, `Scene`) conforme data-model.md
- [x] T022 [P] [US1] Erros de domínio em `apps/server-rust/crates/imagery/src/domain/errors.rs` (`ImageryDomainError` thiserror)
- [x] T023 [US1] Use case `SearchRecentImagery` em `apps/server-rust/crates/imagery/src/application/use_cases.rs` (priorização por recência FR-004, limite/`truncated` FR-011, `AreaTooLarge`, default 30d via `Clock`; usa cache) + `ImageryAppError` em `application/errors.rs` (depende T011, T021, T022)
- [x] T024 [US1] Adapter `InpeStacProvider` em `apps/server-rust/crates/imagery/src/infrastructure/inpe_stac.rs` (reqwest timeout+1 retry, `StacItemDto`, `TryFrom<StacItemDto> for Scene`, descarte de itens inválidos). Inclui validação de `IMAGERY_INPE_STAC_COLLECTIONS` contra `GET /collections` no startup/bootstrap (config) — falhar startup se vazio ou se algum ID não existir no catálogo (§12, evita 0 resultados silencioso)
- [x] T025 [US1] Handler `POST /api/imagery/search` em `apps/server-rust/crates/server/src/presentation/imagery/handler.rs` (DTOs serde, `TryFrom` request→`AreaQuery`, `From` result→response, sem regra de negócio)
- [x] T026 [US1] Endpoint de proxy de asset `GET /api/imagery/assets` em `presentation/imagery/handler.rs` (stream do thumbnail/preview, `Cache-Control` curto)
- [x] T027 [US1] Registrar `InpeStacProvider` no `bootstrap/dependencies.rs` e montar rotas no `router.rs` (depende T014, T024, T025, T026)
- [x] T028 [P] [US1] `SearchImageryUseCase` em `apps/client/src/@core/imagery/application/use-case/SearchImageryUseCase.ts` + `ImageryApiRepository.search()` (depende T016)
- [x] T029 [US1] Render no `useSatelliteMap.ts`: footprints `L.geoJSON` clicáveis + `L.imageOverlay` da cena priorizada (via `/api/imagery/assets`) (FR-003)
- [x] T030 [US1] Componente `apps/client/src/components/imagery/SceneInfoPanel.vue` (data de aquisição, sensor, fonte) + handler de clique no footprint (FR-007)
- [x] T031 [US1] `Mapa/Index.vue`: busca inicial da área visível (source=inpe), estado vazio "sem imagens" e estado de carregando (FR-008)

**Checkpoint**: US1 funcional e testável de ponta a ponta (MVP).

---

## Phase 4: User Story 2 - Navegar e filtrar por área e período (Priority: P2)

**Goal**: Pan/zoom refazem a busca; filtro de datas restringe as cenas; percorrer cenas por data.

**Independent Test**: Mover o mapa atualiza as cenas; alterar intervalo de datas filtra resultados.

### Tests for User Story 2 ⚠️

- [x] T032 [P] [US2] Teste de integração em `apps/server-rust/tests/imagery_daterange.rs` (filtro por `date_from`/`date_to` honrado; default 30d quando omitido via `Clock` fake)

### Implementation for User Story 2

- [x] T033 [US2] Aplicar `date_from`/`date_to` no DTO do handler e propagar a `DateRange` no use case; default e `truncated` explícitos (`handler.rs`, `use_cases.rs`)
- [x] T034 [P] [US2] Componente `apps/client/src/components/imagery/DateRangeFilter.vue` (intervalo de datas, default últimos 30d)
- [x] T035 [US2] `useSatelliteMap.ts`: refazer busca em `moveend`/`zoomend` (debounce) para a nova bbox (FR-005)
- [x] T036 [US2] `Mapa/Index.vue`: navegação entre cenas sobrepostas por data (alternar overlay/precedência da mais recente) (US2/AC3)
- [x] T037 [US2] Surface do `truncated` no frontend: aviso "refine a área" quando atingiu o limite (FR-011)

**Checkpoint**: US1 e US2 funcionam independentemente.

---

## Phase 5: User Story 3 - Comparar fontes INPE e NASA (Priority: P3)

**Goal**: Selecionar/alternar fonte; NASA GIBS como camada de tiles; fallback quando INPE indisponível.

**Independent Test**: Selecionar NASA mostra camada GIBS; INPE fora → app oferece NASA.

### Tests for User Story 3 ⚠️

- [x] T038 [P] [US3] Teste de contrato em `apps/server-rust/tests/imagery_gibs.rs` (GIBS→`TileLayer`; INPE 5xx→502 com `suggested_source:"nasa"`)

### Implementation for User Story 3

- [x] T039 [P] [US3] Adapter `NasaGibsProvider` em `apps/server-rust/crates/imagery/src/infrastructure/nasa_gibs.rs` (resolve layer/date→`TileLayerDescriptor`)
- [x] T040 [US3] Endpoint proxy de tile `GET /api/imagery/tiles/{z}/{x}/{y}` em `presentation/imagery/handler.rs`
- [x] T041 [US3] Registrar `NasaGibsProvider` no bootstrap; `search` retorna `tile_layer` quando `source=nasa` (depende T014, T039, T040)
- [x] T042 [P] [US3] Componente `apps/client/src/components/imagery/SourceSelector.vue` (INPE/NASA)
- [x] T043 [US3] `useSatelliteMap.ts`: adicionar/alternar `L.tileLayer` GIBS via `/api/imagery/tiles` quando fonte=NASA (FR-012)
- [x] T044 [US3] `Mapa/Index.vue`: UX de fallback — em `provider_unavailable` oferecer `suggested_source` (US3/AC2)

**Checkpoint**: Todas as stories independentemente funcionais.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [x] T045 [P] Testes unitários de validação em `apps/server-rust/crates/imagery/src/domain/models.rs` (`BBox::area_deg2`, `DateRange`, `CloudCover` limites)
- [x] T046 [P] Teste de cache em `apps/server-rust/tests/imagery_cache.rs` (busca repetida dentro do TTL não chama o provedor 2x)
- [x] T047 [US1] Sinalização de cobertura de nuvens (FR-010) no `SceneInfoPanel.vue` e estilo do footprint quando `cloud_cover` presente
- [x] T048 [P] Atualizar `docs/`/`quickstart.md` se rotas/contratos mudarem; doc-tests de APIs públicas (§17)
- [x] T049 Rodar checklist CI (§20): `cargo check/fmt/clippy/test/doc --workspace` em `apps/server-rust` e `pnpm lint`/`pnpm test --run` em `apps/client`
- [x] T050 Executar validação do `quickstart.md` (smoke curl + cenários de aceitação US1–US3)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (P1)**: sem dependências.
- **Foundational (P2)**: depende do Setup — BLOQUEIA todas as stories.
- **User Stories (P3+)**: dependem da Foundational. Podem então seguir em paralelo ou em ordem P1→P2→P3.
- **Polish (P6)**: depende das stories desejadas concluídas.

### User Story Dependencies

- **US1 (P1)**: após Foundational. Sem dependência de outras stories (MVP).
- **US2 (P2)**: após Foundational. Estende US1 (busca/render), mas testável de forma independente.
- **US3 (P3)**: após Foundational. Adiciona segunda fonte; independente das demais.

### Within Each Story

- Testes escritos e FALHANDO antes da implementação (§17).
- Domínio (models/errors) → application (use case/ports) → infrastructure (adapters) → presentation (handlers/router) → frontend.

### Parallel Opportunities

- Setup: T002, T003, T004 em paralelo.
- Foundational: T005, T008, T009, T012, T013, T015, T016, T017 em paralelo (após T006/T007/T011 quando aplicável).
- US1: testes T018/T019/T020 em paralelo; T021/T022 em paralelo; backend (T024–T027) e frontend (T028–T031) por trilhas separadas após T023.
- Stories US1/US2/US3 paralelizáveis entre devs após a Foundational.

---

## Parallel Example: User Story 1

```bash
# Testes US1 juntos (devem falhar primeiro):
Task: "Contrato do search em apps/server-rust/tests/imagery_search_contract.rs"   # T018
Task: "Integração do adapter em apps/server-rust/tests/provider_adapters.rs"      # T019
Task: "Vitest do use case em apps/client/src/@core/imagery/__tests__/..."         # T020

# Modelos de domínio US1 juntos:
Task: "models.rs (Scene/BBox/DateRange/...) "                                     # T021
Task: "errors.rs (ImageryDomainError)"                                            # T022
```

---

## Implementation Strategy

### MVP First (US1)

1. Phase 1 Setup → 2. Phase 2 Foundational (crítico) → 3. Phase 3 US1 → **validar US1 isolada** (mapa + cena CBERS-4A recente + estado vazio) → demo.

### Incremental Delivery

1. Setup + Foundational → fundação pronta.
2. US1 → testar → demo (MVP: imagens recentes no mapa).
3. US2 → navegação/filtro de datas → demo.
4. US3 → fonte NASA + fallback → demo.

### Parallel Team Strategy

Após a Foundational: Dev A→US1, Dev B→US2, Dev C→US3, integrando independentemente.

---

## Notes

- [P] = arquivos diferentes, sem dependência pendente.
- `application` nunca depende de `infrastructure`; composição só no bootstrap (§4/§21).
- Sem `unwrap()`/`expect()` em produção; `anyhow` só no `server` (§10).
- Commit por tarefa ou grupo lógico; parar nos checkpoints para validar a story.
