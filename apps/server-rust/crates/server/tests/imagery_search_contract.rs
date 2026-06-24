//! Testes de contrato do endpoint de busca (indexação de `serde_json::Value`
//! é idiomática em testes; produção mantém `indexing_slicing = deny`).
#![allow(clippy::indexing_slicing)]

use std::time::Duration;

use serde_json::{Value, json};
use server::AppConfig;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_config(base_url: String) -> AppConfig {
    AppConfig {
        bind_addr: "127.0.0.1:0".to_owned(),
        inpe_base_url: base_url,
        inpe_collections: vec!["CBERS-WFI-8D-1".to_owned()],
        inpe_wpm_collections: vec!["CB4A-WPM-PCA-FUSED-1".to_owned()],
        gibs_base_url: "https://example.invalid/gibs".to_owned(),
        gibs_layer: "VIIRS_NOAA20_CorrectedReflectance_TrueColor".to_owned(),
        provider_timeout: Duration::from_secs(5),
        cache_ttl: Duration::from_secs(60),
        cache_capacity: 64,
        result_limit: 50,
        max_area_deg2: 4.0,
        default_window_days: 30,
        cors_allow_origin: "*".to_owned(),
        validate_collections: false,
    }
}

async fn spawn_app(config: AppConfig) -> String {
    let state = server::bootstrap::build_state(config).await.expect("state");
    let app = server::build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    format!("http://{addr}")
}

fn feature(id: &str, datetime: Option<&str>) -> Value {
    let mut props = json!({ "eo:cloud_cover": 0.0, "instruments": ["WFI"] });
    if let Some(dt) = datetime {
        props["datetime"] = json!(dt);
    }
    json!({
        "type": "Feature",
        "id": id,
        "properties": props,
        "geometry": { "type": "Polygon", "coordinates": [[[-48.0,-16.0],[-47.0,-16.0],[-47.0,-15.0],[-48.0,-15.0],[-48.0,-16.0]]] },
        "assets": { "thumbnail": { "href": "https://example.invalid/t.png", "type": "image/png" } }
    })
}

async fn search_body(base: &str) -> (reqwest::StatusCode, Value) {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/api/imagery/search"))
        .json(&json!({ "bbox": [-48.0,-16.0,-47.9,-15.9], "source": "inpe" }))
        .send()
        .await
        .expect("request");
    let status = resp.status();
    let body: Value = resp.json().await.expect("json");
    (status, body)
}

#[tokio::test]
async fn returns_scenes_skipping_invalid_item() {
    let mock = MockServer::start().await;
    let collection = json!({
        "type": "FeatureCollection",
        "features": [
            feature("scene-recent", Some("2026-05-17T00:00:00Z")),
            feature("scene-older", Some("2026-05-09T00:00:00Z")),
            feature("scene-invalid", None)
        ]
    });
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(collection))
        .mount(&mock)
        .await;

    let base = spawn_app(test_config(mock.uri())).await;
    let (status, body) = search_body(&base).await;

    assert_eq!(status, 200);
    assert_eq!(
        body["scenes"].as_array().expect("arr").len(),
        2,
        "invalid item skipped"
    );
    assert_eq!(body["prioritized_scene_id"], json!("scene-recent"));
    assert_eq!(body["source"], json!("inpe"));
}

#[tokio::test]
async fn empty_coverage_is_ok_not_error() {
    let mock = MockServer::start().await;
    let collection = json!({ "type": "FeatureCollection", "features": [] });
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(collection))
        .mount(&mock)
        .await;

    let base = spawn_app(test_config(mock.uri())).await;
    let (status, body) = search_body(&base).await;

    assert_eq!(status, 200);
    assert!(body["scenes"].as_array().expect("arr").is_empty());
    assert!(body.get("prioritized_scene_id").is_none() || body["prioritized_scene_id"].is_null());
}

#[tokio::test]
async fn provider_5xx_maps_to_502_with_suggested_source() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock)
        .await;

    let base = spawn_app(test_config(mock.uri())).await;
    let (status, body) = search_body(&base).await;

    assert_eq!(status, 502);
    assert_eq!(body["code"], json!("provider_unavailable"));
    assert_eq!(body["suggested_source"], json!("nasa"));
}

#[tokio::test]
async fn area_too_large_maps_to_422() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"type":"FeatureCollection","features":[]})),
        )
        .mount(&mock)
        .await;

    let base = spawn_app(test_config(mock.uri())).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/api/imagery/search"))
        .json(&json!({ "bbox": [-50.0,-20.0,-40.0,-10.0], "source": "inpe" }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 422);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["code"], json!("area_too_large"));
}
