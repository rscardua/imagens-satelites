use std::time::Duration;

use async_trait::async_trait;
use moka::future::Cache;

use crate::application::models::ImagerySearchResult;
use crate::application::ports::ImageryCache;

/// Cache em memória de curta duração (TTL) dos metadados de busca.
///
/// Sem persistência durável (FR-015): reduz chamadas redundantes às fontes
/// durante pan/zoom e protege limites do provedor (FR-013).
#[derive(Clone)]
pub struct MokaImageryCache {
    inner: Cache<String, ImagerySearchResult>,
}

impl MokaImageryCache {
    #[must_use]
    pub fn new(ttl: Duration, max_capacity: u64) -> Self {
        let inner = Cache::builder()
            .time_to_live(ttl)
            .max_capacity(max_capacity)
            .build();
        Self { inner }
    }
}

#[async_trait]
impl ImageryCache for MokaImageryCache {
    async fn get(&self, key: &str) -> Option<ImagerySearchResult> {
        self.inner.get(key).await
    }

    async fn put(&self, key: String, value: ImagerySearchResult) {
        self.inner.insert(key, value).await;
    }
}
