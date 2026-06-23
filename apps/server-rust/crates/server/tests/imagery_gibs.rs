//! Contrato da fonte NASA GIBS: busca retorna camada de tiles; proxy de tile.
#![allow(clippy::indexing_slicing)]

use std::time::Duration;

use serde_json::{Value, json};
use server::AppConfig;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn config(gibs_base_url: String) -> AppConfig {
    AppConfig {
        bind_addr: "127.0.0.1:0".to_owned(),
        inpe_base_url: "https://example.invalid/stac".to_owned(),
        inpe_collections: vec!["CBERS-WFI-8D-1".to_owned()],
        gibs_base_url,
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
async fn nasa_search_returns_tile_layer() {
    let base = spawn(config("https://example.invalid/gibs".to_owned())).await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/api/imagery/search"))
        .json(&json!({ "bbox": [-48.0,-16.0,-47.9,-15.9], "source": "nasa" }))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["source"], json!("nasa"));
    assert!(body["scenes"].as_array().expect("arr").is_empty());
    assert_eq!(body["tile_layer"]["max_zoom"], json!(9));
    assert!(
        body["tile_layer"]["url_template"]
            .as_str()
            .expect("tpl")
            .contains("{z}")
    );
}

#[tokio::test]
async fn tile_proxy_streams_gibs_tile() {
    let gibs = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/VIIRS_NOAA20_CorrectedReflectance_TrueColor/default/2026-05-01/GoogleMapsCompatible_Level9/3/2/1.jpg",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "image/jpeg")
                .set_body_bytes(vec![0xFF, 0xD8, 0xFF]),
        )
        .mount(&gibs)
        .await;

    let base = spawn(config(gibs.uri())).await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{base}/api/imagery/tiles/3/1/2?layer=VIIRS_NOAA20_CorrectedReflectance_TrueColor&date=2026-05-01&source=nasa"
        ))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()["content-type"], "image/jpeg");
    let bytes = resp.bytes().await.expect("bytes");
    assert_eq!(&bytes[..3], &[0xFF, 0xD8, 0xFF]);
}
