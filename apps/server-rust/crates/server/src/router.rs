use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::{get, post};
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::app_state::AppState;
use crate::presentation::imagery;

/// Monta a árvore HTTP com as camadas globais (§14: timeout, limite, trace, CORS).
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/health", get(health))
        .route("/api/imagery/search", post(imagery::search))
        .route("/api/imagery/assets", get(imagery::proxy_asset))
        .route("/api/imagery/overview", get(imagery::proxy_overview))
        .route("/api/imagery/window", get(imagery::proxy_window))
        .route("/api/imagery/tiles/{z}/{x}/{y}", get(imagery::proxy_tile))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(RequestBodyLimitLayer::new(64 * 1024))
        .layer(cors)
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
