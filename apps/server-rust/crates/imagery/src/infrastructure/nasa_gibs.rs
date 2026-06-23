use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{Duration as ChronoDuration, NaiveDate, Utc};
use reqwest::Client;

use crate::application::models::{AreaQuery, TileLayerDescriptor};
use crate::application::ports::{
    AssetKind, AssetPayload, ImageryProvider, ProviderError, ProviderResponse, TileProvider,
};
use crate::domain::models::{SceneId, SourceId};

const TILE_MATRIX_SET: &str = "GoogleMapsCompatible_Level9";
const MAX_ZOOM: u8 = 9;

/// Adapter da NASA GIBS (camada de tiles WMTS/REST).
pub struct NasaGibsProvider {
    base_url: String,
    layer: String,
    http: Client,
}

impl NasaGibsProvider {
    /// Constrói o adapter.
    ///
    /// # Errors
    /// Falha se o cliente HTTP não puder ser construído.
    pub fn new(
        base_url: impl Into<String>,
        layer: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, ProviderError> {
        let http = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ProviderError::Protocol(e.to_string()))?;
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            layer: layer.into(),
            http,
        })
    }

    fn recent_date() -> NaiveDate {
        (Utc::now() - ChronoDuration::days(1)).date_naive()
    }
}

#[async_trait]
impl ImageryProvider for NasaGibsProvider {
    fn source(&self) -> SourceId {
        SourceId::Nasa
    }

    async fn search(&self, _query: &AreaQuery) -> Result<ProviderResponse, ProviderError> {
        let date = Self::recent_date().format("%Y-%m-%d").to_string();
        let url_template = format!(
            "/api/imagery/tiles/{{z}}/{{x}}/{{y}}?layer={}&date={}",
            self.layer, date
        );
        Ok(ProviderResponse::TileLayer(TileLayerDescriptor {
            source: SourceId::Nasa,
            url_template,
            layer: self.layer.clone(),
            date: Self::recent_date(),
            max_zoom: MAX_ZOOM,
            attribution: "Imagery courtesy of NASA EOSDIS GIBS".to_owned(),
        }))
    }

    async fn fetch_asset(
        &self,
        _scene_id: &SceneId,
        _kind: AssetKind,
    ) -> Result<AssetPayload, ProviderError> {
        Err(ProviderError::Protocol(
            "GIBS does not expose per-scene assets".to_owned(),
        ))
    }
}

#[async_trait]
impl TileProvider for NasaGibsProvider {
    fn source(&self) -> SourceId {
        SourceId::Nasa
    }

    async fn fetch_tile(
        &self,
        z: u32,
        x: u32,
        y: u32,
        layer: &str,
        date: &str,
    ) -> Result<AssetPayload, ProviderError> {
        // GIBS REST ordena {z}/{y}/{x}.
        let url = format!(
            "{}/{}/default/{}/{}/{}/{}/{}.jpg",
            self.base_url, layer, date, TILE_MATRIX_SET, z, y, x
        );
        let resp = self.http.get(&url).send().await.map_err(|e| {
            if e.is_timeout() {
                ProviderError::Timeout
            } else {
                ProviderError::Unavailable
            }
        })?;
        if !resp.status().is_success() {
            return Err(ProviderError::Unavailable);
        }
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_owned();
        let bytes: Bytes = resp
            .bytes()
            .await
            .map_err(|e| ProviderError::Protocol(e.to_string()))?;
        Ok(AssetPayload {
            content_type,
            bytes,
        })
    }
}
