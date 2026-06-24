use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::Value;
use tracing::warn;

use crate::application::models::AreaQuery;
use crate::application::ports::{
    AssetKind, AssetPayload, ImageryProvider, ProviderError, ProviderResponse,
};
use crate::domain::models::{CloudCover, Footprint, Scene, SceneId, Sensor, SourceId};

/// Adapter do catálogo STAC do INPE (Brazil Data Cube) para CBERS-4A.
pub struct InpeStacProvider {
    base_url: String,
    collections: Vec<String>,
    http: Client,
    source: SourceId,
}

impl InpeStacProvider {
    /// Constrói o adapter para uma `source` específica (ex.: WFI ou WPM da CBERS-4A).
    ///
    /// # Errors
    /// Falha se o cliente HTTP não puder ser construído ou se `collections` for vazio.
    pub fn new(
        base_url: impl Into<String>,
        collections: Vec<String>,
        timeout: Duration,
        source: SourceId,
    ) -> Result<Self, ProviderError> {
        if collections.is_empty() {
            return Err(ProviderError::Protocol(
                "no STAC collections configured".to_owned(),
            ));
        }
        let http = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ProviderError::Protocol(e.to_string()))?;
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            collections,
            http,
            source,
        })
    }

    /// Valida que cada collection configurada existe no catálogo (`GET /collections`).
    ///
    /// Evita o "0 resultados silencioso" por ID inválido (§12 — falha explícita no startup).
    ///
    /// # Errors
    /// Falha se o catálogo não responder ou se algum ID configurado não existir.
    pub async fn validate_collections(&self) -> Result<(), ProviderError> {
        let url = format!("{}/collections", self.base_url);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| map_send_error(&e))?;
        if !resp.status().is_success() {
            return Err(ProviderError::Unavailable);
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::Protocol(e.to_string()))?;
        let available: Vec<String> = body
            .get("collections")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.get("id").and_then(Value::as_str).map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        for wanted in &self.collections {
            if !available.contains(wanted) {
                return Err(ProviderError::Protocol(format!(
                    "configured STAC collection not found: {wanted}"
                )));
            }
        }
        Ok(())
    }

    async fn get_with_retry(
        &self,
        url: &str,
        query: &[(&str, String)],
    ) -> Result<Value, ProviderError> {
        let mut last_err = ProviderError::Unavailable;
        for _ in 0..2 {
            let result = self.http.get(url).query(query).send().await;
            match result {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return resp
                            .json::<Value>()
                            .await
                            .map_err(|e| ProviderError::Protocol(e.to_string()));
                    }
                    if status.is_server_error() {
                        last_err = ProviderError::Unavailable;
                    } else {
                        return Err(ProviderError::Protocol(format!("STAC status {status}")));
                    }
                }
                Err(e) => last_err = map_send_error(&e),
            }
        }
        Err(last_err)
    }

    /// Resolve o `href` de um asset (ex.: `tci`, `thumbnail`) de uma cena por id.
    async fn resolve_href(
        &self,
        scene_id: &SceneId,
        asset_key: &str,
    ) -> Result<String, ProviderError> {
        let url = format!("{}/search", self.base_url);
        let params = vec![
            ("ids", scene_id.as_str().to_owned()),
            ("limit", "1".to_owned()),
        ];
        let body = self.get_with_retry(&url, &params).await?;
        body.get("features")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(|f| f.get("assets"))
            .and_then(|a| a.get(asset_key))
            .and_then(|a| a.get("href"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| ProviderError::Protocol(format!("asset '{asset_key}' href not found")))
    }
}

#[async_trait]
impl ImageryProvider for InpeStacProvider {
    fn source(&self) -> SourceId {
        self.source
    }

    async fn search(&self, query: &AreaQuery) -> Result<ProviderResponse, ProviderError> {
        let url = format!("{}/search", self.base_url);
        let bbox = format!(
            "{},{},{},{}",
            query.bbox.min_lon(),
            query.bbox.min_lat(),
            query.bbox.max_lon(),
            query.bbox.max_lat()
        );
        let datetime = format!(
            "{}/{}",
            query.range.start().to_rfc3339(),
            query.range.end().to_rfc3339()
        );
        let params = vec![
            ("collections", self.collections.join(",")),
            ("bbox", bbox),
            ("datetime", datetime),
            ("limit", query.limit.to_string()),
        ];
        let body = self.get_with_retry(&url, &params).await?;
        let features = body
            .get("features")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let scenes: Vec<Scene> = features
            .iter()
            .filter_map(|f| {
                build_scene(f, self.source).or_else(|| {
                    warn!("skipping invalid STAC item");
                    None
                })
            })
            .collect();
        Ok(ProviderResponse::Scenes(scenes))
    }

    async fn fetch_asset(
        &self,
        scene_id: &SceneId,
        kind: AssetKind,
    ) -> Result<AssetPayload, ProviderError> {
        let url = format!("{}/search", self.base_url);
        let params = vec![
            ("ids", scene_id.as_str().to_owned()),
            ("limit", "1".to_owned()),
        ];
        let body = self.get_with_retry(&url, &params).await?;
        let asset = body
            .get("features")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|f| f.get("assets"))
            .and_then(|a| a.get(asset_name(kind)));
        let href = asset
            .and_then(|a| a.get("href"))
            .and_then(Value::as_str)
            .ok_or_else(|| ProviderError::Protocol("asset href not found".to_owned()))?;
        let declared_type = asset
            .and_then(|a| a.get("type"))
            .and_then(Value::as_str)
            .map(str::to_owned);

        let resp = self
            .http
            .get(href)
            .send()
            .await
            .map_err(|e| map_send_error(&e))?;
        if !resp.status().is_success() {
            return Err(ProviderError::Unavailable);
        }
        let content_type = declared_type
            .or_else(|| {
                resp.headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "application/octet-stream".to_owned());
        let bytes: Bytes = resp
            .bytes()
            .await
            .map_err(|e| ProviderError::Protocol(e.to_string()))?;
        Ok(AssetPayload {
            content_type,
            bytes,
        })
    }

    async fn fetch_overview(
        &self,
        scene_id: &SceneId,
        max_size: u32,
    ) -> Result<AssetPayload, ProviderError> {
        let href = self.resolve_href(scene_id, "tci").await?;
        let png = tokio::task::spawn_blocking(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .map_err(|e| ProviderError::Protocol(e.to_string()))?;
            crate::infrastructure::cog_overview::render_overview_png(client, &href, max_size)
        })
        .await
        .map_err(|e| ProviderError::Protocol(e.to_string()))??;
        Ok(AssetPayload {
            content_type: "image/png".to_owned(),
            bytes: Bytes::from(png),
        })
    }

    async fn fetch_window(
        &self,
        scene_id: &SceneId,
        bbox: [f64; 4],
        target: u32,
    ) -> Result<AssetPayload, ProviderError> {
        let href = self.resolve_href(scene_id, "tci").await?;
        let png = tokio::task::spawn_blocking(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .map_err(|e| ProviderError::Protocol(e.to_string()))?;
            crate::infrastructure::cog_window::render_window_png(&client, &href, bbox, target)
        })
        .await
        .map_err(|e| ProviderError::Protocol(e.to_string()))??;
        Ok(AssetPayload {
            content_type: "image/png".to_owned(),
            bytes: Bytes::from(png),
        })
    }
}

fn asset_name(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Thumbnail => "thumbnail",
        AssetKind::Preview => "preview",
    }
}

fn map_send_error(e: &reqwest::Error) -> ProviderError {
    if e.is_timeout() {
        ProviderError::Timeout
    } else {
        ProviderError::Unavailable
    }
}

/// Constrói uma [`Scene`] a partir de um Item STAC; retorna `None` se inválido.
#[allow(clippy::cast_possible_truncation)]
fn build_scene(item: &Value, source: SourceId) -> Option<Scene> {
    let id = item.get("id").and_then(Value::as_str)?;
    let scene_id = SceneId::new(id).ok()?;
    let props = item.get("properties")?;
    let datetime_str = props.get("datetime").and_then(Value::as_str)?;
    let acquired_at: DateTime<Utc> = DateTime::parse_from_rfc3339(datetime_str)
        .ok()?
        .with_timezone(&Utc);
    let geometry = item.get("geometry").filter(|g| g.is_object())?.clone();

    let cloud_cover = props
        .get("eo:cloud_cover")
        .and_then(Value::as_f64)
        .and_then(|v| CloudCover::try_from(v as f32).ok());

    let default_sensor = match source {
        SourceId::InpeWpm => "WPM",
        _ => "WFI",
    };
    let sensor = extract_sensor(props, default_sensor);
    let has_preview = item
        .get("assets")
        .is_some_and(|a| a.get("thumbnail").is_some() || a.get("preview").is_some());

    Some(Scene {
        id: scene_id,
        acquired_at,
        source,
        sensor,
        footprint: Footprint::new(geometry),
        cloud_cover,
        has_preview,
    })
}

fn extract_sensor(props: &Value, default: &str) -> Sensor {
    let raw = props
        .get("instruments")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .and_then(Value::as_str)
        .or_else(|| props.get("platform").and_then(Value::as_str))
        .or_else(|| props.get("bdc:instrument").and_then(Value::as_str))
        .unwrap_or(default);
    Sensor::new(raw).unwrap_or_else(|_| Sensor::new("WFI").expect("non-empty literal"))
}
