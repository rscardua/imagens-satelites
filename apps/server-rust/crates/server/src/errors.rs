use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use imagery::ImageryAppError;
use serde::Serialize;

/// Corpo de erro padronizado da API (contrato `ErrorBody`).
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_source: Option<String>,
}

/// Wrapper que adapta erros de aplicação para resposta HTTP (§10 — sem inverter semântica).
pub struct ApiError(pub ImageryAppError);

impl From<ImageryAppError> for ApiError {
    fn from(value: ImageryAppError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let suggested = self.0.suggested_source().map(|s| s.as_str().to_owned());
        let (status, code) = match &self.0 {
            ImageryAppError::AreaTooLarge { .. } => {
                (StatusCode::UNPROCESSABLE_ENTITY, "area_too_large")
            }
            ImageryAppError::SourceNotConfigured(_) => (StatusCode::BAD_REQUEST, "invalid_params"),
            ImageryAppError::ProviderUnavailable(_) | ImageryAppError::ProviderProtocol(_) => {
                (StatusCode::BAD_GATEWAY, "provider_unavailable")
            }
            ImageryAppError::ProviderTimeout(_) => {
                (StatusCode::GATEWAY_TIMEOUT, "provider_timeout")
            }
        };
        let body = ErrorBody {
            code,
            message: self.0.to_string(),
            suggested_source: suggested,
        };
        (status, Json(body)).into_response()
    }
}

/// Erro de validação de entrada (DTO malformado) → 422.
pub struct ValidationError(pub String);

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            code: "invalid_params",
            message: self.0,
            suggested_source: None,
        };
        (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
    }
}
