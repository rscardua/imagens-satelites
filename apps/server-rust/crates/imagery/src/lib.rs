//! Bounded context `imagery`: busca de cenas/camadas recentes de satélite.
//!
//! Camadas (dependências apontam para dentro):
//! `infrastructure` → `application` → `domain`.

pub mod application;
pub mod domain;
pub mod infrastructure;

pub use application::models::{AreaQuery, ImagerySearchResult, SearchInput, TileLayerDescriptor};
pub use application::ports::{
    AssetKind, AssetPayload, Clock, ImageryCache, ImageryProvider, ProviderError, ProviderResponse,
    TileProvider,
};
pub use application::use_cases::{ProxyTile, SearchConfig, SearchRecentImagery};
pub use domain::errors::ImageryDomainError;
pub use domain::models::{CloudCover, DateRange, Footprint, Scene, SceneId, Sensor, SourceId};

pub use application::errors::ImageryAppError;
pub use infrastructure::inpe_stac::InpeStacProvider;
pub use infrastructure::moka_cache::MokaImageryCache;
pub use infrastructure::nasa_gibs::NasaGibsProvider;
pub use infrastructure::system_clock::SystemClock;
