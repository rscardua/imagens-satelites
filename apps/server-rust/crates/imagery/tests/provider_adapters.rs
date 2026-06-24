//! Testes de integração do `InpeStacProvider` contra um STAC simulado (wiremock).

use std::time::Duration;

use chrono::{TimeZone, Utc};
use imagery::{
    AreaQuery, DateRange, ImageryProvider, InpeStacProvider, ProviderError, ProviderResponse,
    SourceId,
};
use serde_json::json;
use shared::BBox;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn query() -> AreaQuery {
    let start = Utc
        .with_ymd_and_hms(2026, 5, 1, 0, 0, 0)
        .single()
        .expect("start");
    let end = Utc
        .with_ymd_and_hms(2026, 5, 31, 0, 0, 0)
        .single()
        .expect("end");
    AreaQuery {
        bbox: BBox::new(-48.0, -16.0, -47.9, -15.9).expect("bbox"),
        range: DateRange::new(start, end).expect("range"),
        source: SourceId::Inpe,
        limit: 50,
    }
}

fn provider(base_url: String, timeout_secs: u64) -> InpeStacProvider {
    InpeStacProvider::new(
        base_url,
        vec!["CBERS-WFI-8D-1".to_owned()],
        Duration::from_secs(timeout_secs),
        SourceId::Inpe,
    )
    .expect("provider")
}

#[tokio::test]
async fn parses_valid_and_skips_invalid_items() {
    let mock = MockServer::start().await;
    let body = json!({
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "id": "scene-ok",
                "properties": { "datetime": "2026-05-17T00:00:00Z", "eo:cloud_cover": 1.5, "instruments": ["WFI"] },
                "geometry": { "type": "Polygon", "coordinates": [[[-48,-16],[-47,-16],[-47,-15],[-48,-15],[-48,-16]]] },
                "assets": { "thumbnail": { "href": "https://x.invalid/t.png", "type": "image/png" } }
            },
            { "type": "Feature", "id": "scene-no-datetime", "properties": {}, "geometry": { "type": "Polygon", "coordinates": [] } }
        ]
    });
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&mock)
        .await;

    let result = provider(mock.uri(), 5).search(&query()).await.expect("ok");
    match result {
        ProviderResponse::Scenes(scenes) => {
            assert_eq!(scenes.len(), 1, "invalid item without datetime is skipped");
            let scene = scenes.first().expect("scene");
            assert_eq!(scene.id.as_str(), "scene-ok");
            assert_eq!(scene.sensor.as_str(), "WFI");
            assert!(scene.has_preview);
            assert!(scene.cloud_cover.is_some());
        }
        ProviderResponse::TileLayer(_) => panic!("expected scenes"),
    }
}

#[tokio::test]
async fn server_error_maps_to_unavailable() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock)
        .await;

    let err = provider(mock.uri(), 5)
        .search(&query())
        .await
        .expect_err("should fail");
    assert!(matches!(err, ProviderError::Unavailable));
}

#[tokio::test]
async fn slow_response_maps_to_timeout() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(3)))
        .mount(&mock)
        .await;

    let err = provider(mock.uri(), 1)
        .search(&query())
        .await
        .expect_err("should time out");
    assert!(matches!(err, ProviderError::Timeout));
}
