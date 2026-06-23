# Quickstart — Imagens Recentes CBERS-4A no Mapa

Como subir e validar a feature localmente. Backend Rust (`apps/server-rust`) + frontend Vue (`apps/client`). Sem banco na v1.

## Pré-requisitos

- Rust stable, edition 2024 (`rust-version` 1.94+). `rustup show` deve refletir a toolchain do projeto.
- Node.js 22+ e pnpm (frontend).
- Acesso de saída à internet (provedores INPE STAC e NASA GIBS).

## Configuração (env — validada no bootstrap, §12)

```bash
# apps/server-rust
IMAGERY_INPE_STAC_BASE_URL=https://data.inpe.br/bdc/stac/v1
IMAGERY_INPE_STAC_COLLECTIONS=CBERS-WFI-8D-1
IMAGERY_NASA_GIBS_BASE_URL=https://gibs.earthdata.nasa.gov/wmts/epsg3857/best
IMAGERY_NASA_GIBS_LAYER=VIIRS_NOAA20_CorrectedReflectance_TrueColor
IMAGERY_PROVIDER_TIMEOUT_SECS=8
IMAGERY_CACHE_TTL_SECS=60
IMAGERY_RESULT_LIMIT=50
IMAGERY_MAX_AREA_DEG2=4.0
HTTP_BIND_ADDR=0.0.0.0:8080
RUST_LOG=info,imagery=debug
CORS_ALLOW_ORIGIN=http://localhost:5173
```

Falha de qualquer obrigatória → startup aborta (não há default silencioso para URLs/limites).

## Subir backend

```bash
cd apps/server-rust
cargo run -p server
# logs: load_config → init_telemetry → bootstrap (providers + cache + use case) → axum::serve
```

## Subir frontend

```bash
cd apps/client
pnpm install
pnpm dev   # http://localhost:5173 ; configurar base da API para o backend
```

## Validação manual (mapeia para os Acceptance Scenarios)

1. **US1/AC1** — abrir o mapa numa área com cobertura conhecida → footprints aparecem e a cena mais recente é sobreposta (overlay), com data visível. (SC-001: ≤10 s.)
2. **US1/AC2** — navegar a uma área sem cobertura → mensagem clara "sem imagens", sem estado de erro.
3. **US1/AC3** — clicar num footprint → painel com data de aquisição, sensor e fonte.
4. **US2/AC1** — arrastar/zoom → busca refeita para a nova área.
5. **US2/AC2** — ajustar intervalo de datas → só cenas no intervalo.
6. **US3/AC1** — selecionar NASA → camada de tiles GIBS substitui a visão por cenas.
7. **US3/AC2** — simular INPE fora → app sinaliza indisponibilidade e oferece NASA.

## Smoke via API

```bash
# Busca INPE (CBERS-4A) numa bbox pequena, últimos 30d (default)
curl -s -X POST http://localhost:8080/api/imagery/search \
  -H 'content-type: application/json' \
  -d '{"bbox":[-47.95,-15.85,-47.80,-15.70],"source":"inpe"}' | jq

# Esperado: { source:"inpe", scenes:[...], prioritized_scene_id:"...", truncated:false }
# Sem cobertura  → scenes:[] (HTTP 200)
# Área ampla     → HTTP 422 code=area_too_large
# Provider fora  → HTTP 502 code=provider_unavailable, suggested_source:"nasa"
```

## Verificação (CI — §20 guidelines)

```bash
cd apps/server-rust
cargo check --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
cargo doc --workspace --no-deps
```

```bash
cd apps/client
pnpm lint
pnpm test -- --run
```
