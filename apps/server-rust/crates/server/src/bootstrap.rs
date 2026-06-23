use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use imagery::{
    ImageryProvider, InpeStacProvider, MokaImageryCache, NasaGibsProvider, ProxyTile, SearchConfig,
    SearchRecentImagery, SourceId, SystemClock, TileProvider,
};

use crate::app_state::AppState;
use crate::config::AppConfig;

/// Compõe o container de dependências (§5.4) a partir da config validada.
///
/// # Errors
/// Falha se um provider não puder ser construído ou se a validação das
/// collections do STAC (quando habilitada) falhar.
pub async fn build_state(config: AppConfig) -> Result<AppState> {
    let inpe = InpeStacProvider::new(
        config.inpe_base_url.clone(),
        config.inpe_collections.clone(),
        config.provider_timeout,
    )
    .map_err(|e| anyhow::anyhow!("failed to build INPE STAC provider: {e}"))?;

    if config.validate_collections {
        inpe.validate_collections()
            .await
            .map_err(|e| anyhow::anyhow!("STAC collections validation failed: {e}"))?;
    }

    let gibs = Arc::new(
        NasaGibsProvider::new(
            config.gibs_base_url.clone(),
            config.gibs_layer.clone(),
            config.provider_timeout,
        )
        .map_err(|e| anyhow::anyhow!("failed to build NASA GIBS provider: {e}"))?,
    );

    let mut providers: HashMap<SourceId, Arc<dyn ImageryProvider>> = HashMap::new();
    providers.insert(SourceId::Inpe, Arc::new(inpe));
    providers.insert(SourceId::Nasa, gibs.clone());

    let mut tile_providers: HashMap<SourceId, Arc<dyn TileProvider>> = HashMap::new();
    tile_providers.insert(SourceId::Nasa, gibs);

    let cache = Arc::new(MokaImageryCache::new(
        config.cache_ttl,
        config.cache_capacity,
    ));
    let clock = Arc::new(SystemClock);
    let search_config = SearchConfig {
        max_area_deg2: config.max_area_deg2,
        default_window_days: config.default_window_days,
        result_limit: config.result_limit,
    };
    let search = Arc::new(SearchRecentImagery::new(
        providers,
        cache,
        clock,
        search_config,
    ));
    let tiles = Arc::new(ProxyTile::new(tile_providers));

    Ok(AppState {
        search,
        tiles,
        config: Arc::new(config),
    })
}
