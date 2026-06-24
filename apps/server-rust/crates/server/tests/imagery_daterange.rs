//! US2: filtro por intervalo de datas é encaminhado ao STAC; datas parciais são rejeitadas.
#![allow(clippy::indexing_slicing)]

use std::time::Duration;

use serde_json::{Value, json};
use server::AppConfig;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn config(inpe_base_url: String) -> AppConfig {
    AppConfig {
        bind_addr: "127.0.0.1:0".to_owned(),
        inpe_base_url,
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

async fn spawn(config: AppConfig) -> String {
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

#[tokio::test]
async fn forwards_datetime_range_to_stac() {
    let mock = MockServer::start().await;
    // Só responde se o `datetime` esperado (rfc3339 start/end) for encaminhado.
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param(
            "datetime",
            "2026-05-01T00:00:00+00:00/2026-05-10T00:00:00+00:00",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "type": "FeatureCollection", "features": [] })),
        )
        .mount(&mock)
        .await;

    let base = spawn(config(mock.uri())).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/api/imagery/search"))
        .json(&json!({
            "bbox": [-48.0,-16.0,-47.9,-15.9],
            "source": "inpe",
            "date_from": "2026-05-01T00:00:00Z",
            "date_to": "2026-05-10T00:00:00Z"
        }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200, "datetime range forwarded and matched");
}

#[tokio::test]
async fn partial_date_is_rejected() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "type": "FeatureCollection", "features": [] })),
        )
        .mount(&mock)
        .await;

    let base = spawn(config(mock.uri())).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/api/imagery/search"))
        .json(&json!({
            "bbox": [-48.0,-16.0,-47.9,-15.9],
            "source": "inpe",
            "date_from": "2026-05-01T00:00:00Z"
        }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 422);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["code"], json!("invalid_params"));
}
