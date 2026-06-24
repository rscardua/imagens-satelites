use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use imagery::{
    AssetKind, DateRange, ImagerySearchResult, Scene, SearchInput, SourceId, TileLayerDescriptor,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use shared::BBox;

use crate::app_state::AppState;
use crate::errors::{ApiError, ValidationError};

#[derive(Debug, Deserialize)]
pub struct SearchRequestDto {
    pub bbox: [f64; 4],
    pub source: String,
    #[serde(default)]
    pub date_from: Option<String>,
    #[serde(default)]
    pub date_to: Option<String>,
    #[serde(default)]
    pub limit: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct SceneDto {
    pub id: String,
    pub acquired_at: String,
    pub source: String,
    pub sensor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_cover: Option<f32>,
    pub footprint: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_asset: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TileLayerDto {
    pub source: String,
    pub url_template: String,
    pub layer: String,
    pub date: String,
    pub max_zoom: u8,
    pub attribution: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResultDto {
    pub source: String,
    pub scenes: Vec<SceneDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prioritized_scene_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_layer: Option<TileLayerDto>,
    pub truncated: bool,
}

#[derive(Debug, Deserialize)]
pub struct AssetQuery {
    pub source: String,
    pub scene_id: String,
    pub asset: String,
}

#[derive(Debug, Deserialize)]
pub struct TileQuery {
    pub layer: String,
    pub date: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OverviewQuery {
    pub source: String,
    pub scene_id: String,
    #[serde(default)]
    pub size: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct WindowQuery {
    pub source: String,
    pub scene_id: String,
    /// `min_lon,min_lat,max_lon,max_lat`
    pub bbox: String,
    #[serde(default)]
    pub size: Option<u32>,
}

/// `POST /api/imagery/search` — busca cenas/camada recentes.
pub async fn search(
    State(state): State<AppState>,
    payload: Result<Json<SearchRequestDto>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Json(req) = match payload {
        Ok(json) => json,
        Err(rej) => return ValidationError(rej.body_text()).into_response(),
    };

    let input = match to_search_input(&req) {
        Ok(input) => input,
        Err(err) => return err.into_response(),
    };

    match state.search.execute(input).await {
        Ok(result) => Json(to_result_dto(&result)).into_response(),
        Err(app_err) => ApiError(app_err).into_response(),
    }
}

/// `GET /api/imagery/assets` — proxy do asset visual de uma cena (R5).
pub async fn proxy_asset(State(state): State<AppState>, Query(q): Query<AssetQuery>) -> Response {
    let source = match SourceId::try_from(q.source.as_str()) {
        Ok(s) => s,
        Err(e) => return ValidationError(e.to_string()).into_response(),
    };
    let kind = match q.asset.as_str() {
        "thumbnail" => AssetKind::Thumbnail,
        "preview" => AssetKind::Preview,
        other => return ValidationError(format!("unknown asset: {other}")).into_response(),
    };
    let scene_id = match imagery::SceneId::new(q.scene_id) {
        Ok(id) => id,
        Err(e) => return ValidationError(e.to_string()).into_response(),
    };

    match state.search.fetch_asset(source, &scene_id, kind).await {
        Ok(payload) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, payload.content_type),
                (header::CACHE_CONTROL, "public, max-age=300".to_owned()),
            ],
            payload.bytes,
        )
            .into_response(),
        Err(app_err) => ApiError(app_err).into_response(),
    }
}

/// `GET /api/imagery/overview` — renderiza um overview PNG nítido do COG da cena.
pub async fn proxy_overview(
    State(state): State<AppState>,
    Query(q): Query<OverviewQuery>,
) -> Response {
    let source = match SourceId::try_from(q.source.as_str()) {
        Ok(s) => s,
        Err(e) => return ValidationError(e.to_string()).into_response(),
    };
    let scene_id = match imagery::SceneId::new(q.scene_id) {
        Ok(id) => id,
        Err(e) => return ValidationError(e.to_string()).into_response(),
    };
    let size = q.size.unwrap_or(2048).clamp(256, 4096);

    match state.search.fetch_overview(source, &scene_id, size).await {
        Ok(payload) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, payload.content_type),
                (header::CACHE_CONTROL, "public, max-age=600".to_owned()),
            ],
            payload.bytes,
        )
            .into_response(),
        Err(app_err) => ApiError(app_err).into_response(),
    }
}

/// `GET /api/imagery/window` — renderiza uma janela geográfica em resolução nativa (2 m).
pub async fn proxy_window(State(state): State<AppState>, Query(q): Query<WindowQuery>) -> Response {
    let source = match SourceId::try_from(q.source.as_str()) {
        Ok(s) => s,
        Err(e) => return ValidationError(e.to_string()).into_response(),
    };
    let scene_id = match imagery::SceneId::new(q.scene_id) {
        Ok(id) => id,
        Err(e) => return ValidationError(e.to_string()).into_response(),
    };
    let parts: Vec<f64> = q
        .bbox
        .split(',')
        .filter_map(|p| p.trim().parse::<f64>().ok())
        .collect();
    let [min_lon, min_lat, max_lon, max_lat] = match parts.as_slice() {
        [lon0, lat0, lon1, lat1] => [*lon0, *lat0, *lon1, *lat1],
        _ => {
            return ValidationError("bbox must be 'minLon,minLat,maxLon,maxLat'".to_owned())
                .into_response();
        }
    };
    let size = q.size.unwrap_or(1024).clamp(256, 4096);

    match state
        .search
        .fetch_window(
            source,
            &scene_id,
            [min_lon, min_lat, max_lon, max_lat],
            size,
        )
        .await
    {
        Ok(payload) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, payload.content_type),
                (header::CACHE_CONTROL, "public, max-age=600".to_owned()),
            ],
            payload.bytes,
        )
            .into_response(),
        Err(app_err) => ApiError(app_err).into_response(),
    }
}

/// `GET /api/imagery/tiles/{z}/{x}/{y}` — proxy de tile (NASA GIBS).
pub async fn proxy_tile(
    State(state): State<AppState>,
    Path((z, x, y)): Path<(u32, u32, u32)>,
    Query(tq): Query<TileQuery>,
) -> Response {
    let source = match SourceId::try_from(tq.source.as_deref().unwrap_or("nasa")) {
        Ok(src) => src,
        Err(err) => return ValidationError(err.to_string()).into_response(),
    };
    match state
        .tiles
        .fetch(source, z, x, y, &tq.layer, &tq.date)
        .await
    {
        Ok(payload) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, payload.content_type),
                (header::CACHE_CONTROL, "public, max-age=3600".to_owned()),
            ],
            payload.bytes,
        )
            .into_response(),
        Err(app_err) => ApiError(app_err).into_response(),
    }
}

fn to_search_input(req: &SearchRequestDto) -> Result<SearchInput, ValidationError> {
    let [min_lon, min_lat, max_lon, max_lat] = req.bbox;
    let bbox = BBox::new(min_lon, min_lat, max_lon, max_lat)
        .map_err(|e| ValidationError(e.to_string()))?;
    let source =
        SourceId::try_from(req.source.as_str()).map_err(|e| ValidationError(e.to_string()))?;

    let range = match (req.date_from.as_deref(), req.date_to.as_deref()) {
        (None, None) => None,
        (Some(from), Some(to)) => {
            let start = parse_dt(from)?;
            let end = parse_dt(to)?;
            Some(DateRange::new(start, end).map_err(|e| ValidationError(e.to_string()))?)
        }
        _ => {
            return Err(ValidationError(
                "date_from and date_to must be provided together".to_owned(),
            ));
        }
    };

    Ok(SearchInput {
        bbox,
        range,
        source,
        limit: req.limit,
    })
}

fn parse_dt(value: &str) -> Result<DateTime<Utc>, ValidationError> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| ValidationError(format!("invalid datetime '{value}': {e}")))
}

fn to_result_dto(result: &ImagerySearchResult) -> SearchResultDto {
    SearchResultDto {
        source: result.source.as_str().to_owned(),
        scenes: result.scenes.iter().map(to_scene_dto).collect(),
        prioritized_scene_id: result.prioritized.as_ref().map(|s| s.as_str().to_owned()),
        tile_layer: result.tile_layer.as_ref().map(to_tile_dto),
        truncated: result.truncated,
    }
}

fn to_scene_dto(scene: &Scene) -> SceneDto {
    SceneDto {
        id: scene.id.as_str().to_owned(),
        acquired_at: scene.acquired_at.to_rfc3339(),
        source: scene.source.as_str().to_owned(),
        sensor: scene.sensor.as_str().to_owned(),
        cloud_cover: scene.cloud_cover.map(imagery::CloudCover::value),
        footprint: scene.footprint.geometry().clone(),
        preview_asset: if scene.has_preview {
            Some("thumbnail".to_owned())
        } else {
            None
        },
    }
}

fn to_tile_dto(tile: &TileLayerDescriptor) -> TileLayerDto {
    TileLayerDto {
        source: tile.source.as_str().to_owned(),
        url_template: tile.url_template.clone(),
        layer: tile.layer.clone(),
        date: tile.date.format("%Y-%m-%d").to_string(),
        max_zoom: tile.max_zoom,
        attribution: tile.attribution.clone(),
    }
}
