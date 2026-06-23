# Phase 1 — Data Model

Modelo de domínio da feature. Nenhuma entidade é persistida (FR-015); são tipos de domínio em memória, construídos por busca. Newtypes para invariantes (§8 guidelines), estados inválidos irrepresentáveis.

## Tipos de valor (Value Objects)

### `SourceId`
- Enum fechado: `Inpe` (CBERS-4A via STAC) | `Nasa` (GIBS WMTS).
- Serialização externa: `"inpe"` | `"nasa"`.
- Regra: parsing de string externa via `TryFrom<&str>` → falha `UnknownSource`.

### `BBox`
- Campos: `min_lon`, `min_lat`, `max_lon`, `max_lat` (graus, WGS84/EPSG:4326).
- Invariantes (construção `BBox::new` → `Result`): `min < max` em ambos os eixos; lon ∈ [-180,180]; lat ∈ [-90,90].
- Deriva: `area_deg2()` p/ regra de área ampla (FR-011).

### `DateRange`
- Campos: `start: DateTime<Utc>`, `end: DateTime<Utc>`.
- Invariante: `start <= end`. Default quando ausente: `[now-30d, now]` (assumption da spec) — resolvido na `application` usando `Clock`, não no domínio.
- Construção falível `DateRange::new` → `InvalidDateRange`.

### `CloudCover`
- Newtype `f32` ∈ [0,100], opcional por cena (`Option<CloudCover>`) — só quando a fonte fornece (FR-010).
- Construção `TryFrom<f32>` → `InvalidCloudCover` fora do intervalo.

### `SceneId`
- Newtype `String` não-vazio (id do Item STAC). Construção valida não-vazio.

### `Footprint`
- Geometria do polígono de cobertura (GeoJSON Polygon/MultiPolygon) + `bbox: BBox` derivada.
- Origem: `geometry` do Item STAC.

### `AssetRef`
- Referência ao recurso visual da cena: `kind` (`Thumbnail` | `Preview`), `media_type`, `href` (URL externa — nunca exposta crua ao cliente; acessada via proxy).

## Entidades

### `Scene` (Cena)
Representa uma captura disponível (Key Entity "Imagem de Satélite" da spec).

| Campo | Tipo | Origem | Notas |
|---|---|---|---|
| `id` | `SceneId` | STAC `id` | |
| `acquired_at` | `DateTime<Utc>` | STAC `properties.datetime` | base da priorização por recência (FR-004) |
| `source` | `SourceId` | provider | `Inpe` nesta entidade (GIBS não gera Scene) |
| `sensor` | `String` (newtype `Sensor`) | STAC `properties` | metadado exibido (FR-007); na v1 predominantemente `WFI` (coleção `CBERS-WFI-8D-1`; WPM/MUX por cena fora de escopo — ver research R1) |
| `footprint` | `Footprint` | STAC `geometry` | render como GeoJSON (FR-003) |
| `cloud_cover` | `Option<CloudCover>` | STAC `eo:cloud_cover` | sinalizado quando presente (FR-010) |
| `preview` | `Option<AssetRef>` | STAC `assets` | overlay raster da cena priorizada (R3) |

- **Regras**: construída via `TryFrom<StacItemDto>` (falível, §11). Cena sem `datetime` ou `geometry` válidos é descartada com log (não quebra a busca).
- **Sem ciclo de vida persistido**: existe apenas no escopo da resposta da busca.

### `TileLayerDescriptor` (camada NASA GIBS)
Descritor de camada de tiles (não é uma cena).

| Campo | Tipo | Notas |
|---|---|---|
| `source` | `SourceId` | `Nasa` |
| `url_template` | `String` | template WMTS/REST (`{z}/{x}/{y}`) servido/proxiado |
| `layer` | `String` | identificador da camada GIBS (default `VIIRS_NOAA20_CorrectedReflectance_TrueColor`) |
| `date` | `NaiveDate` | dia do mosaico (mais recente disponível) |
| `max_zoom` | `u8` | limite da matriz (default 9 — `GoogleMapsCompatible_Level9`) |
| `attribution` | `String` | crédito NASA GIBS |

### `AreaQuery` (Consulta de Área)
Parâmetros de uma busca (Key Entity "Consulta de Área").

| Campo | Tipo | Notas |
|---|---|---|
| `bbox` | `BBox` | área visível do mapa |
| `range` | `DateRange` | default 30d se ausente (via `Clock`) |
| `source` | `SourceId` | fonte selecionada (FR-012) |
| `limit` | `u16` | default 50, máx configurável (FR-011) |

- Ordenação implícita: por `acquired_at` desc (recência).

## Resultado da busca

### `ImagerySearchResult`
| Campo | Tipo | Notas |
|---|---|---|
| `source` | `SourceId` | fonte efetivamente consultada |
| `scenes` | `Vec<Scene>` | vazio quando sem cobertura (FR-008) — não é erro |
| `tile_layer` | `Option<TileLayerDescriptor>` | presente quando `source = Nasa` |
| `prioritized` | `Option<SceneId>` | cena mais recente p/ overlay (FR-004) |
| `truncated` | `bool` | `true` se atingiu `limit` (sinaliza refinar) |

## Erros (tipados — §10)

- **Domínio** (`ImageryDomainError`, thiserror): `InvalidBBox`, `InvalidDateRange`, `InvalidCloudCover`, `UnknownSource`, `EmptySceneId`.
- **Aplicação** (`ImageryAppError`): `AreaTooLarge { area, limit }`, `ProviderUnavailable(SourceId)`, `ProviderTimeout(SourceId)`, `ProviderProtocol(String)`. `NoImagery` é representado como resultado vazio, não erro.

## Mapeamento de validação ↔ requisitos

| Requisito | Onde é garantido |
|---|---|
| FR-004 (recência) | `ImagerySearchResult.prioritized` + ordenação no use case |
| FR-006 (intervalo de datas) | `DateRange` + default via `Clock` |
| FR-007 (metadados) | campos `acquired_at`/`sensor`/`source` em `Scene` |
| FR-008 (sem cobertura) | `scenes` vazio (não erro) |
| FR-010 (nuvens) | `Scene.cloud_cover` opcional |
| FR-011 (área ampla/limite) | `BBox::area_deg2` → `AreaTooLarge`; `limit`/`truncated` |
| FR-012 (seleção de fonte) | `AreaQuery.source` |
