# Imagens de Satélite CBERS-4A no Mapa

Sistema web que busca imagens **recentes de satélite** e as exibe num mapa interativo.
A fonte primária é o **CBERS-4A do INPE** (catálogo STAC do Brazil Data Cube) e a
alternativa é a **NASA GIBS** (camada de tiles WMTS). O backend faz _proxy ao vivo_ das
fontes — sem persistência durável — e o frontend desenha footprints, sobrepõe o raster da
cena mais recente e mostra os metadados.

- **Backend**: Rust (edition 2024) com Clean Architecture em Cargo Workspace — Axum, Tokio, reqwest, moka.
- **Frontend**: Vue 3 + Vite + TypeScript + Leaflet.
- **Acesso**: público (sem autenticação) na v1.

> Especificação completa, plano e tarefas em [`specs/001-satellite-imagery-map/`](specs/001-satellite-imagery-map/).
> Diretrizes de arquitetura em [`docs/`](docs/).

---

## Arquitetura

```
Frontend Vue 3 (Leaflet)
  → POST /api/imagery/search        busca cenas/camada para a área visível
  → GET  /api/imagery/assets        proxy do thumbnail (overlay raster)
  → GET  /api/imagery/tiles/{z}/{x}/{y}   proxy de tile (NASA GIBS)

Backend Rust (Clean Architecture, dependências apontam para dentro)
  presentation → application → domain
  infrastructure → application/domain
  composição só no bootstrap (AppState via Arc)

Fontes externas
  INPE  → https://data.inpe.br/bdc/stac/v1  (coleção CBERS-WFI-8D-1)
  NASA  → https://gibs.earthdata.nasa.gov/wmts/epsg3857/best
```

Crates do workspace (`apps/server-rust/crates/`):

- `shared` — abstrações transversais (`BBox`, `AppError`).
- `imagery` — bounded context: `domain` (entidades/erros), `application` (caso de uso + ports), `infrastructure` (adapters INPE/NASA, cache, relógio).
- `server` — HTTP (Axum), bootstrap, config, telemetria, mapeamento de erros.

---

## Pré-requisitos

- **Rust** estável (edition 2024, `rust-version` 1.94+). A toolchain é fixada em `apps/server-rust/rust-toolchain.toml`.
- **Node.js 22+** e **pnpm**.
- Acesso de saída à internet (as fontes INPE/NASA são consultadas ao vivo).

Verifique:

```bash
cargo --version   # 1.94+
node --version    # v22+
pnpm --version
```

---

## Como rodar

### 1. Backend (Rust)

Forma mais simples — copie o exemplo e rode (o server carrega `.env` automaticamente no startup):

```bash
cd apps/server-rust
cp .env.example .env     # PowerShell: Copy-Item .env.example .env
cargo run -p server
```

A configuração é lida e validada no startup. Variáveis disponíveis (defina via `.env`
ou no ambiente — o ambiente tem precedência):

| Variável | Default | Descrição |
|---|---|---|
| `IMAGERY_INPE_STAC_BASE_URL` | _(obrigatória)_ | Base do STAC INPE |
| `IMAGERY_INPE_STAC_COLLECTIONS` | _(obrigatória)_ | Coleções CBERS-4A/WFI (~64 m, regional) |
| `IMAGERY_INPE_WPM_COLLECTIONS` | `CB4A-WPM-PCA-FUSED-1` | Coleções CBERS-4A/WPM (~2 m, urbano) |
| `IMAGERY_NASA_GIBS_BASE_URL` | `https://gibs.earthdata.nasa.gov/wmts/epsg3857/best` | Base do GIBS |
| `IMAGERY_NASA_GIBS_LAYER` | `VIIRS_NOAA20_CorrectedReflectance_TrueColor` | Camada GIBS |
| `IMAGERY_PROVIDER_TIMEOUT_SECS` | `8` | Timeout por provedor |
| `IMAGERY_CACHE_TTL_SECS` | `60` | TTL do cache de metadados |
| `IMAGERY_RESULT_LIMIT` | `50` | Máximo de cenas por busca |
| `IMAGERY_MAX_AREA_DEG2` | `4.0` | Limite de área (graus²) antes de exigir refinar |
| `IMAGERY_DEFAULT_WINDOW_DAYS` | `30` | Janela padrão de datas |
| `IMAGERY_VALIDATE_COLLECTIONS` | `true` | Valida as collections no startup |
| `HTTP_BIND_ADDR` | `0.0.0.0:8080` | Endereço de bind |
| `CORS_ALLOW_ORIGIN` | `*` | Origem permitida no CORS |
| `RUST_LOG` | `info` | Filtro de logs |

**Bash / Linux / macOS:**

```bash
cd apps/server-rust
export IMAGERY_INPE_STAC_BASE_URL=https://data.inpe.br/bdc/stac/v1
export IMAGERY_INPE_STAC_COLLECTIONS=CBERS-WFI-8D-1
export HTTP_BIND_ADDR=127.0.0.1:8080
cargo run -p server
```

**PowerShell (Windows):**

```powershell
cd apps/server-rust
$env:IMAGERY_INPE_STAC_BASE_URL = "https://data.inpe.br/bdc/stac/v1"
$env:IMAGERY_INPE_STAC_COLLECTIONS = "CBERS-WFI-8D-1"
$env:HTTP_BIND_ADDR = "127.0.0.1:8080"
cargo run -p server
```

O servidor sobe em `http://127.0.0.1:8080`. Health check: `GET /api/health`.

### 2. Frontend (Vue)

```bash
cd apps/client
pnpm install
pnpm dev
```

Abre em **`http://localhost:5180`** (porta fixa — `strictPort`). O Vite faz proxy de `/api`
para `http://localhost:8080` (configurável em `apps/client/vite.config.ts`), então rode o
backend antes.

---

## Smoke test (API)

```bash
# Busca CBERS-4A numa bbox pequena, com intervalo explícito
curl -s -X POST http://127.0.0.1:8080/api/imagery/search \
  -H 'content-type: application/json' \
  -d '{"bbox":[-48.0,-16.0,-47.8,-15.8],"source":"inpe","date_from":"2026-03-01T00:00:00Z","date_to":"2026-06-23T00:00:00Z"}'

# Camada de tiles da NASA
curl -s -X POST http://127.0.0.1:8080/api/imagery/search \
  -H 'content-type: application/json' \
  -d '{"bbox":[-48,-16,-47.8,-15.8],"source":"nasa"}'
```

Respostas esperadas:
- Sem cobertura → `200` com `"scenes": []` (não é erro).
- Área muito ampla → `422 area_too_large`.
- Fonte indisponível → `502 provider_unavailable` com `suggested_source`.

> Observação: a coleção `CBERS-WFI-8D-1` é um **composto WFI de 8 dias**; pela latência,
> a janela padrão de 30 dias pode não conter a cena mais recente — use um intervalo de
> datas mais amplo para visualizar cenas.

---

## Testes

**Backend** (a partir de `apps/server-rust`, espelha o checklist de CI):

```bash
cargo check --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace          # unitários + contrato (wiremock) + adapter + doc-tests
cargo doc --workspace --no-deps
```

**Frontend** (a partir de `apps/client`):

```bash
pnpm lint
pnpm exec vue-tsc --noEmit -p tsconfig.json
pnpm test          # Vitest
# E2E (requer backend + frontend no ar): pnpm exec cypress install && pnpm e2e
```

---

## Estrutura do projeto

```
apps/
  server-rust/                 # backend Rust (Cargo workspace)
    crates/
      shared/                  # BBox, AppError
      imagery/                 # domain / application / infrastructure
      server/                  # Axum, bootstrap, config, presentation
    tests/ (por crate)         # contrato (wiremock) e adapter
  client/                      # frontend Vue 3 + Vite + Leaflet
    src/
      @core/imagery/           # domain / application / infra (Clean Arch no front)
      composables/             # useSatelliteMap (Leaflet)
      components/imagery/       # SourceSelector, DateRangeFilter, SceneInfoPanel
      pages/Mapa/              # página do mapa
docs/                          # diretrizes de arquitetura e estrutura
specs/001-satellite-imagery-map/   # spec, plan, research, data-model, contracts, tasks
```

---

## Notas de escopo (v1)

- Sem persistência durável: o backend faz **proxy ao vivo** das fontes; só há cache em memória de curta duração (FR-015).
- CBERS-4A na v1 = produto **WFI 8 dias** (`CBERS-WFI-8D-1`). Cenas de alta resolução WPM/MUX ficam no catálogo DGI do INPE (fora desta STAC) e podem ser adicionadas via novo adapter sem mudar o núcleo.
- Acesso público; RBAC do projeto disponível mas não obrigatório no fluxo de busca/exibição.
