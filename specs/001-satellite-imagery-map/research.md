# Phase 0 — Research: Imagens Recentes CBERS-4A no Mapa

Resolve as decisões técnicas da feature. Sem NEEDS CLARIFICATION remanescentes (clarify fechou persistência, fontes, render e acesso).

## R1. Fonte primária — INPE STAC (CBERS-4A)

- **Decision**: Consumir o Brazil Data Cube STAC do INPE em `https://data.inpe.br/bdc/stac/v1` (STAC API OGC, `POST /search` com `bbox` + `datetime` + `collections`). Coleção CBERS-4A confirmada e usável (via `GET /collections`, verificada 2026-06-23): **`CBERS-WFI-8D-1`** (CBERS/WFI Level-4-SR, composto de 8 dias). Configurável por env (`IMAGERY_INPE_STAC_COLLECTIONS`, default `CBERS-WFI-8D-1`). Extrair de cada Item: `id`, `properties.datetime`, `geometry` (footprint), `properties` de nuvem (`eo:cloud_cover` quando presente) e `assets` (thumbnail/preview).
- **Rationale**: STAC é o padrão OGC para catálogos de imagem; expõe busca por bbox+datas nativamente, casando com FR-001/FR-006. Resposta GeoJSON-friendly (footprints prontos). Alinha com os adapters geoespaciais/OGC citados na ESTRUTURA_PROJETO. O composto WFI de 8 dias entrega cobertura recente e frequente com footprint+datetime — adequado ao mapa.
- **Alternatives considered**: Portal de visualização/WMS do INPE (sem busca por data estruturada, difícil de testar); scraping do catálogo web (frágil, contra políticas).
- **Limitação confirmada (2026-06-23)**: A STAC pública do BDC expõe apenas 8 coleções; **não há coleções de cena única WPM/MUX** (alta resolução). Cenas individuais CBERS-4A (WPM 2 m) ficam no catálogo DGI do INPE (`dgi.inpe.br`), fora desta STAC. Consequências: (1) `Scene.sensor` na v1 é predominantemente `WFI`; (2) suporte a WPM/MUX por cena fica fora de escopo da v1 (port `ImageryProvider` permite adicionar um adapter DGI no futuro sem mudar o núcleo — FR-014). `mosaic-cbers4a-paraiba-3m-1` é regional (Paraíba) e não serve busca geral.
- **Verificado ao vivo (2026-06-23)** via `GET /search?collections=CBERS-WFI-8D-1&bbox=...&limit=1`: Item expõe `properties.datetime`, `properties.eo:cloud_cover` (ex. `0.0`), `geometry` Polygon e asset **`thumbnail` `image/png`** → overlay via `L.imageOverlay` confirmado (R3 sem fallback). Demais assets são COG: bandas WFI `BAND13`–`BAND16`, `CMASK`, `NDVI`/`EVI`, `CLEAROB`/`TOTALOB` — base para composição RGB real em evolução futura (fora da v1).

## R2. Fonte alternativa — NASA GIBS (WMTS)

- **Decision**: Usar NASA GIBS como **camada de tiles** (WMTS/`REST` tile template, ex.: layers de "Corrected Reflectance" diários). Backend resolve o template/capabilities e expõe ao frontend um descritor de camada (URL template + matriz + data); frontend monta `L.tileLayer`. GIBS não fornece footprints por cena — é mosaico diário global; portanto entra como camada, não como lista de cenas (FR-003/FR-012).
  - Endpoint base (config `IMAGERY_NASA_GIBS_BASE_URL`): `https://gibs.earthdata.nasa.gov/wmts/epsg3857/best`
  - Layer default (config `IMAGERY_NASA_GIBS_LAYER`): `VIIRS_NOAA20_CorrectedReflectance_TrueColor` (alternativa `MODIS_Terra_CorrectedReflectance_TrueColor`), TileMatrixSet `GoogleMapsCompatible_Level9`, formato `jpg`, `date` = dia mais recente disponível.
  - Template REST: `{base}/{layer}/default/{date}/GoogleMapsCompatible_Level9/{z}/{y}/{x}.jpg`
- **Rationale**: GIBS é público, rápido (tiles pré-renderizados), bom para "imagery recente" como fallback de cobertura quando STAC não retorna cena. WMTS combina com `TileLayer` do Leaflet sem tiler próprio.
- **Alternatives considered**: NASA CMR/Earthdata STAC (exige credenciais/token Earthdata; mais atrito na v1 pública); Worldview Snapshots API (orientada a export de imagem única, não a tiles navegáveis).

## R3. Estratégia de exibição do raster da cena (sem tiler próprio)

- **Decision**: Para a cena priorizada (mais recente do STAC), fazer overlay do **asset de preview/thumbnail** (PNG/JPEG) via `L.imageOverlay` georreferenciado pelo bbox do footprint. Footprints de todas as cenas como `L.geoJSON` clicável. Não renderizar COG em escala real na v1.
- **Rationale**: Evita um serviço tiler (titiler/COG) — abstração prematura para v1 (regra §22 das guidelines). Atende "overlay raster da cena priorizada" (FR-003) com latência baixa. Backend faz **proxy** do asset (R5) para evitar CORS/exposição de URL externa.
- **Alternatives considered**: Servir COG via tiler dinâmico (custo operacional alto, fora de escopo v1); overlay direto da URL externa no browser (problemas de CORS/política e acoplamento do frontend ao provedor).

## R4. Biblioteca de mapa no frontend — Leaflet

- **Decision**: Leaflet (TS) encapsulado num composable `useSatelliteMap`. Camadas: base (OSM/raster claro), `geoJSON` (footprints), `imageOverlay` (cena priorizada), `tileLayer` (GIBS). Sem DevExtreme Map para esta feature.
- **Rationale**: Maduro, leve, suporte nativo a `imageOverlay`/`geoJSON`/WMTS-as-tilelayer, casa com os requisitos de render (R3). Vue só orquestra via composable, mantendo a apresentação fina.
- **Alternatives considered**: MapLibre GL (vetorial; overkill p/ overlay de imagem e curva maior p/ ImageOverlay equivalente); DevExtreme Map (menos flexível p/ camadas raster/STAC).

## R5. Proxy ao vivo + cache de curta TTL (FR-015)

- **Decision**: Backend não persiste. `SearchRecentImagery` chama o provider por busca; resultados de **metadados** vão a um cache em memória (`moka`) com TTL curto (default 60 s), chaveado por `(source, bbox arredondado, date_range)`. Assets de imagem são **proxiados** por endpoint dedicado (stream do provedor → cliente) com `Cache-Control` curto; sem armazenamento em disco.
- **Rationale**: Cumpre FR-015 (sem persistência durável) reduzindo chamadas redundantes durante pan/zoom e protegendo limites do provedor (FR-013). Proxy resolve CORS e centraliza timeout/erros (FR-009).
- **Alternatives considered**: Sem cache (estoura limites do provedor em navegação); cache persistente em Redis/DB (persistência durável proibida na v1).

## R6. Async traits / dispatch (§7 guidelines)

- **Decision**: **Estratégia B — dispatch dinâmico.** `ImageryProvider` é trait object-safe via `#[async_trait]`; bootstrap registra `HashMap<SourceId, Arc<dyn ImageryProvider>>` e o use case seleciona o provider por `SourceId` em runtime.
- **Rationale**: A fonte é escolhida em runtime pelo parâmetro do usuário (FR-012) e o conjunto deve ser extensível (FR-014) — caso explícito de necessidade de `Arc<dyn Trait>` (exceção aceitável §23). Garante object safety (não cai na combinação ingênua proibida §7).
- **Alternatives considered**: Dispatch estático com generics (não permite seleção dinâmica por request sem enum manual; mais rígido p/ adicionar fontes).

## R7. Tratamento de erros e resiliência (FR-008/FR-009)

- **Decision**: `ImageryDomainError` (thiserror) p/ invariantes (bbox inválido, range invertido); `ImageryAppError` p/ orquestração (`ProviderUnavailable`, `ProviderTimeout`, `NoImagery`, `AreaTooLarge`). Mapeamento HTTP no `server/errors`: `NoImagery`→200 com lista vazia + flag; `AreaTooLarge`→422; `ProviderUnavailable/Timeout`→502/504 com corpo que sugere fonte alternativa. Sem mapear erro técnico→semântico indevidamente (§10). `reqwest` com timeout por provider; 1 retry leve em erro transitório.
- **Rationale**: Distingue "sem dados" de "falha técnica" (FR-008 vs FR-009) e preserva semântica (§10). Permite ao frontend oferecer fallback de fonte (US3/AC2).
- **Alternatives considered**: Tratar tudo como 500 (perde UX de fallback e mensagem clara).

## R8. Limite de resultados / área ampla (FR-011)

- **Decision**: Validar área da bbox no use case; acima de `IMAGERY_MAX_AREA_DEG2` (default 4.0 — ~2°×2°) retornar `AreaTooLarge` orientando aproximar. Limitar resultados ao default 50 cenas mais recentes (config).
- **Rationale**: Protege payload, mapa e limites do provedor; mantém SC-001.
- **Alternatives considered**: Paginação completa (complexidade desnecessária p/ exploração visual v1).

## R9. Observabilidade & entrada (§13/§14)

- **Decision**: `tracing` + `tower-http` (TraceLayer, TimeoutLayer, RequestBodyLimitLayer), correlation id propagado, `RUST_LOG`. CORS liberado para o frontend. Config validada no bootstrap (URLs base dos provedores, timeouts, TTL, limites).
- **Rationale**: Cumpre §13/§14 e §12 (config validada no startup, consumo tipado).

## R10. Testes (§17)

- **Decision**: Unitários: priorização por data, limite, validação de bbox/range, parsing STAC→Scene (`TryFrom`). Contरato/integração: `wiremock` simulando STAC (com/sem cena, erro, timeout) e GIBS capabilities; verifica mapeamento HTTP. Frontend: Vitest p/ `SearchImageryUseCase` + Zod parsing; Cypress p/ render de footprints/overlay/troca de fonte. Doubles in-memory p/ `Clock`/`ImageryCache`.
- **Rationale**: Casos de uso testáveis sem infra real (§17); cobre cenários de spec (sem cobertura, provider fora).
