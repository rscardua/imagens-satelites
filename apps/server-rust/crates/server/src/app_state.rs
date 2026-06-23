use std::sync::Arc;

use imagery::{ProxyTile, SearchRecentImagery};

use crate::config::AppConfig;

/// Estado compartilhado da aplicação, composto no bootstrap e clonado por `Arc`.
#[derive(Clone)]
pub struct AppState {
    pub search: Arc<SearchRecentImagery>,
    pub tiles: Arc<ProxyTile>,
    pub config: Arc<AppConfig>,
}
