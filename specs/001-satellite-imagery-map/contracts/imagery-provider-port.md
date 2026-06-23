# Contrato — Port `ImageryProvider` (application)

Port da camada `application` (não do domínio — §6: integração externa). Dispatch dinâmico (Estratégia B, §7): trait object-safe, registrado como `Arc<dyn ImageryProvider>` por `SourceId` no bootstrap. Adapters concretos vivem em `infrastructure`.

## Trait

```rust
// crates/imagery/src/application/ports.rs
use async_trait::async_trait;

#[async_trait]
pub trait ImageryProvider: Send + Sync {
    /// Identifica a fonte que este provider atende.
    fn source(&self) -> SourceId;

    /// Busca cenas/camada para a consulta. Implementações NÃO priorizam nem
    /// aplicam limite de área — isso é responsabilidade do use case.
    /// Retorno vazio = sem cobertura (não é erro).
    async fn search(&self, query: &AreaQuery) -> Result<ProviderResponse, ProviderError>;

    /// Stream do asset visual de uma cena (proxy). Opcional por provider.
    async fn fetch_asset(&self, scene_id: &SceneId, kind: AssetKind)
        -> Result<AssetStream, ProviderError>;
}

pub enum ProviderResponse {
    Scenes(Vec<Scene>),                 // INPE STAC
    TileLayer(TileLayerDescriptor),     // NASA GIBS
}

#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    #[error("provider unavailable")]
    Unavailable,
    #[error("provider timeout")]
    Timeout,
    #[error("protocol error: {0}")]
    Protocol(String),
}
```

## Ports auxiliares (application)

```rust
#[async_trait]
pub trait ImageryCache: Send + Sync {
    async fn get(&self, key: &CacheKey) -> Option<ImagerySearchResult>;
    async fn put(&self, key: CacheKey, value: ImagerySearchResult); // TTL interno curto
}

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;   // default DateRange (30d) + chave de cache temporal
}
```

## Regras do contrato

1. **Sem regra de negócio no adapter** (§5.3/§21): o provider apenas traduz protocolo externo → tipos de domínio. Priorização por recência, limite de área e `truncated` são do `SearchRecentImagery` use case.
2. **Conversão falível** (§11): mapeamento DTO STAC → `Scene` via `TryFrom`, descartando itens inválidos com log (não derruba a busca).
3. **Erros** (§10): falha técnica → `ProviderError` (não confundir com "sem cobertura", que é `Scenes(vec![])`). O use case converte `ProviderError` → `ImageryAppError` preservando semântica.
4. **Object safety**: trait sem métodos genéricos, `Send + Sync`, retornos concretos/boxed — apta a `Arc<dyn ImageryProvider>`.
5. **Resiliência**: timeout e retry leve ficam no adapter (config injetada no bootstrap).

## Adapters previstos (infrastructure)

| Adapter | `SourceId` | Resposta | Notas |
|---|---|---|---|
| `InpeStacProvider` | `Inpe` | `Scenes` | `POST /search` STAC; bbox+datetime; assets thumbnail/preview |
| `NasaGibsProvider` | `Nasa` | `TileLayer` | resolve layer/date → `TileLayerDescriptor`; `fetch_asset` via proxy de tile |

## Cenários de teste de contrato (tests/)

- STAC retorna N itens válidos → `Scenes` com N cenas ordenáveis.
- STAC retorna item sem `datetime`/`geometry` → item descartado, busca prossegue.
- STAC 5xx → `ProviderError::Unavailable` → HTTP 502 + `suggested_source`.
- STAC lento além do timeout → `ProviderError::Timeout` → HTTP 504.
- GIBS → `TileLayer` com template/atribuição válidos.
- Busca repetida idêntica dentro do TTL → servida do `ImageryCache` (sem 2ª chamada externa).
