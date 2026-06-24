use std::collections::HashMap;
use std::sync::Arc;

use chrono::Duration;

use super::errors::ImageryAppError;
use super::models::{AreaQuery, ImagerySearchResult, SearchInput};
use super::ports::{
    AssetKind, AssetPayload, Clock, ImageryCache, ImageryProvider, ProviderError, ProviderResponse,
    TileProvider,
};
use crate::domain::models::{DateRange, SceneId, SourceId};

/// Parâmetros operacionais do caso de uso (lidos da config no bootstrap).
#[derive(Debug, Clone, Copy)]
pub struct SearchConfig {
    pub max_area_deg2: f64,
    pub default_window_days: i64,
    pub result_limit: u16,
}

/// Caso de uso: busca cenas/camada recentes priorizando recência.
pub struct SearchRecentImagery {
    providers: HashMap<SourceId, Arc<dyn ImageryProvider>>,
    cache: Arc<dyn ImageryCache>,
    clock: Arc<dyn Clock>,
    config: SearchConfig,
}

impl SearchRecentImagery {
    #[must_use]
    pub fn new(
        providers: HashMap<SourceId, Arc<dyn ImageryProvider>>,
        cache: Arc<dyn ImageryCache>,
        clock: Arc<dyn Clock>,
        config: SearchConfig,
    ) -> Self {
        Self {
            providers,
            cache,
            clock,
            config,
        }
    }

    fn provider(&self, source: SourceId) -> Result<&Arc<dyn ImageryProvider>, ImageryAppError> {
        self.providers
            .get(&source)
            .ok_or(ImageryAppError::SourceNotConfigured(source))
    }

    fn map_provider_error(source: SourceId, err: ProviderError) -> ImageryAppError {
        match err {
            ProviderError::Unavailable => ImageryAppError::ProviderUnavailable(source),
            ProviderError::Timeout => ImageryAppError::ProviderTimeout(source),
            ProviderError::Protocol(msg) => ImageryAppError::ProviderProtocol(msg),
        }
    }

    /// Executa a busca: valida área, resolve janela temporal, consulta cache e
    /// provider, prioriza por recência e aplica limite.
    ///
    /// # Errors
    /// Retorna [`ImageryAppError`] em área excessiva, fonte não configurada ou
    /// falha técnica do provider. Ausência de cobertura retorna resultado vazio.
    pub async fn execute(
        &self,
        input: SearchInput,
    ) -> Result<ImagerySearchResult, ImageryAppError> {
        let area = input.bbox.area_deg2();
        if area > self.config.max_area_deg2 {
            return Err(ImageryAppError::AreaTooLarge {
                area,
                limit: self.config.max_area_deg2,
            });
        }

        let range = match input.range {
            Some(r) => r,
            None => self.default_range(),
        };
        let limit = input.limit.unwrap_or(self.config.result_limit).max(1);
        let query = AreaQuery {
            bbox: input.bbox,
            range,
            source: input.source,
            limit,
        };

        let key = cache_key(&query);
        if let Some(hit) = self.cache.get(&key).await {
            return Ok(hit);
        }

        let provider = self.provider(input.source)?;
        let response = provider
            .search(&query)
            .await
            .map_err(|e| Self::map_provider_error(input.source, e))?;

        let result = Self::assemble(input.source, response, limit);
        self.cache.put(key, result.clone()).await;
        Ok(result)
    }

    /// Proxy de asset de uma cena pela fonte indicada.
    ///
    /// # Errors
    /// Falha se a fonte não estiver configurada ou o provider falhar.
    pub async fn fetch_asset(
        &self,
        source: SourceId,
        scene_id: &SceneId,
        kind: AssetKind,
    ) -> Result<AssetPayload, ImageryAppError> {
        let provider = self.provider(source)?;
        provider
            .fetch_asset(scene_id, kind)
            .await
            .map_err(|e| Self::map_provider_error(source, e))
    }

    /// Renderiza um overview PNG de maior resolução da cena (decodifica o COG).
    ///
    /// # Errors
    /// Falha se a fonte não estiver configurada ou o provider falhar.
    pub async fn fetch_overview(
        &self,
        source: SourceId,
        scene_id: &SceneId,
        max_size: u32,
    ) -> Result<AssetPayload, ImageryAppError> {
        let provider = self.provider(source)?;
        provider
            .fetch_overview(scene_id, max_size)
            .await
            .map_err(|e| Self::map_provider_error(source, e))
    }

    /// Renderiza uma janela geográfica da cena em resolução nativa (PNG).
    ///
    /// # Errors
    /// Falha se a fonte não estiver configurada ou o provider falhar.
    pub async fn fetch_window(
        &self,
        source: SourceId,
        scene_id: &SceneId,
        bbox: [f64; 4],
        target: u32,
    ) -> Result<AssetPayload, ImageryAppError> {
        let provider = self.provider(source)?;
        provider
            .fetch_window(scene_id, bbox, target)
            .await
            .map_err(|e| Self::map_provider_error(source, e))
    }

    fn default_range(&self) -> DateRange {
        let now = self.clock.now();
        let start = now - Duration::days(self.config.default_window_days);
        // start <= end por construção (janela positiva); fallback seguro sem unwrap.
        DateRange::new(start, now).unwrap_or_else(|_| {
            // janela degenerada só se default_window_days < 0; colapsa para [now,now].
            DateRange::new(now, now).expect("now <= now holds")
        })
    }

    fn assemble(source: SourceId, response: ProviderResponse, limit: u16) -> ImagerySearchResult {
        match response {
            ProviderResponse::Scenes(mut scenes) => {
                scenes.sort_by(|a, b| b.acquired_at.cmp(&a.acquired_at));
                let truncated = scenes.len() > usize::from(limit);
                scenes.truncate(usize::from(limit));
                let prioritized = scenes.first().map(|s| s.id.clone());
                ImagerySearchResult {
                    source,
                    scenes,
                    tile_layer: None,
                    prioritized,
                    truncated,
                }
            }
            ProviderResponse::TileLayer(descriptor) => ImagerySearchResult {
                source,
                scenes: Vec::new(),
                tile_layer: Some(descriptor),
                prioritized: None,
                truncated: false,
            },
        }
    }
}

/// Caso de uso de proxy de tiles (NASA GIBS).
pub struct ProxyTile {
    providers: HashMap<SourceId, Arc<dyn TileProvider>>,
}

impl ProxyTile {
    #[must_use]
    pub fn new(providers: HashMap<SourceId, Arc<dyn TileProvider>>) -> Self {
        Self { providers }
    }

    /// Busca um tile da fonte indicada.
    ///
    /// # Errors
    /// Falha se a fonte não tiver provider de tile configurado ou se o provider falhar.
    pub async fn fetch(
        &self,
        source: SourceId,
        z: u32,
        x: u32,
        y: u32,
        layer: &str,
        date: &str,
    ) -> Result<AssetPayload, ImageryAppError> {
        let provider = self
            .providers
            .get(&source)
            .ok_or(ImageryAppError::SourceNotConfigured(source))?;
        provider
            .fetch_tile(z, x, y, layer, date)
            .await
            .map_err(|e| SearchRecentImagery::map_provider_error(source, e))
    }
}

/// Chave de cache: fonte + bbox arredondada (3 casas) + janela (epoch segundos).
fn cache_key(query: &AreaQuery) -> String {
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    format!(
        "{}|{:.3},{:.3},{:.3},{:.3}|{}-{}|{}",
        query.source.as_str(),
        round(query.bbox.min_lon()),
        round(query.bbox.min_lat()),
        round(query.bbox.max_lon()),
        round(query.bbox.max_lat()),
        query.range.start().timestamp(),
        query.range.end().timestamp(),
        query.limit,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{DateTime, TimeZone, Utc};
    use serde_json::json;
    use shared::BBox;

    use super::*;
    use crate::domain::models::{Footprint, Scene, Sensor};

    struct FixedClock(DateTime<Utc>);
    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    #[derive(Default)]
    struct NoopCache;
    #[async_trait]
    impl ImageryCache for NoopCache {
        async fn get(&self, _key: &str) -> Option<ImagerySearchResult> {
            None
        }
        async fn put(&self, _key: String, _value: ImagerySearchResult) {}
    }

    /// Cache em memória que conta puts (para teste de cache hit).
    #[derive(Default)]
    struct CountingCache {
        store: Mutex<HashMap<String, ImagerySearchResult>>,
        puts: Mutex<u32>,
    }
    #[async_trait]
    impl ImageryCache for CountingCache {
        async fn get(&self, key: &str) -> Option<ImagerySearchResult> {
            self.store.lock().expect("lock").get(key).cloned()
        }
        async fn put(&self, key: String, value: ImagerySearchResult) {
            *self.puts.lock().expect("lock") += 1;
            self.store.lock().expect("lock").insert(key, value);
        }
    }

    struct StubProvider {
        scenes: Vec<Scene>,
        calls: Mutex<u32>,
    }
    #[async_trait]
    impl ImageryProvider for StubProvider {
        fn source(&self) -> SourceId {
            SourceId::Inpe
        }
        async fn search(&self, _q: &AreaQuery) -> Result<ProviderResponse, ProviderError> {
            *self.calls.lock().expect("lock") += 1;
            Ok(ProviderResponse::Scenes(self.scenes.clone()))
        }
        async fn fetch_asset(
            &self,
            _id: &SceneId,
            _k: AssetKind,
        ) -> Result<AssetPayload, ProviderError> {
            Err(ProviderError::Unavailable)
        }
    }

    fn scene(id: &str, day: u32) -> Scene {
        Scene {
            id: SceneId::new(id).expect("id"),
            acquired_at: Utc
                .with_ymd_and_hms(2026, 5, day, 0, 0, 0)
                .single()
                .expect("date"),
            source: SourceId::Inpe,
            sensor: Sensor::new("WFI").expect("sensor"),
            footprint: Footprint::new(json!({"type":"Polygon","coordinates":[]})),
            cloud_cover: None,
            has_preview: true,
        }
    }

    fn bbox() -> BBox {
        BBox::new(-48.0, -16.0, -47.9, -15.9).expect("bbox")
    }

    fn config() -> SearchConfig {
        SearchConfig {
            max_area_deg2: 4.0,
            default_window_days: 30,
            result_limit: 50,
        }
    }

    fn use_case(cache: Arc<dyn ImageryCache>, provider: Arc<StubProvider>) -> SearchRecentImagery {
        let mut providers: HashMap<SourceId, Arc<dyn ImageryProvider>> = HashMap::new();
        providers.insert(SourceId::Inpe, provider);
        let clock = Arc::new(FixedClock(
            Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0)
                .single()
                .expect("clock"),
        ));
        SearchRecentImagery::new(providers, cache, clock, config())
    }

    fn input() -> SearchInput {
        SearchInput {
            bbox: bbox(),
            range: None,
            source: SourceId::Inpe,
            limit: None,
        }
    }

    #[tokio::test]
    async fn prioritizes_most_recent() {
        let provider = Arc::new(StubProvider {
            scenes: vec![scene("a", 10), scene("c", 17), scene("b", 12)],
            calls: Mutex::new(0),
        });
        let uc = use_case(Arc::new(NoopCache), provider);
        let result = uc.execute(input()).await.expect("ok");
        assert_eq!(result.prioritized.as_ref().map(SceneId::as_str), Some("c"));
        assert_eq!(result.scenes.first().expect("scene").id.as_str(), "c");
        assert!(!result.truncated);
    }

    #[tokio::test]
    async fn empty_is_not_error() {
        let provider = Arc::new(StubProvider {
            scenes: vec![],
            calls: Mutex::new(0),
        });
        let uc = use_case(Arc::new(NoopCache), provider);
        let result = uc.execute(input()).await.expect("ok");
        assert!(result.scenes.is_empty());
        assert!(result.prioritized.is_none());
    }

    #[tokio::test]
    async fn area_too_large_rejected() {
        let provider = Arc::new(StubProvider {
            scenes: vec![],
            calls: Mutex::new(0),
        });
        let uc = use_case(Arc::new(NoopCache), provider);
        let big = SearchInput {
            bbox: BBox::new(-50.0, -20.0, -40.0, -10.0).expect("bbox"),
            range: None,
            source: SourceId::Inpe,
            limit: None,
        };
        let err = uc.execute(big).await.expect_err("should reject");
        assert!(matches!(err, ImageryAppError::AreaTooLarge { .. }));
    }

    #[tokio::test]
    async fn cache_hit_skips_second_provider_call() {
        let provider = Arc::new(StubProvider {
            scenes: vec![scene("a", 10)],
            calls: Mutex::new(0),
        });
        let cache = Arc::new(CountingCache::default());
        let uc = use_case(cache.clone(), provider.clone());
        uc.execute(input()).await.expect("first");
        uc.execute(input()).await.expect("second");
        assert_eq!(
            *provider.calls.lock().expect("lock"),
            1,
            "provider called once"
        );
        assert_eq!(*cache.puts.lock().expect("lock"), 1);
    }

    #[tokio::test]
    async fn unknown_source_not_configured() {
        let provider = Arc::new(StubProvider {
            scenes: vec![],
            calls: Mutex::new(0),
        });
        let uc = use_case(Arc::new(NoopCache), provider);
        let nasa = SearchInput {
            source: SourceId::Nasa,
            ..input()
        };
        let err = uc.execute(nasa).await.expect_err("not configured");
        assert!(matches!(
            err,
            ImageryAppError::SourceNotConfigured(SourceId::Nasa)
        ));
    }
}
