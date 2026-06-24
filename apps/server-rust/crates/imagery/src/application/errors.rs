use thiserror::Error;

use crate::domain::models::SourceId;

/// Erros de orquestração do caso de uso.
///
/// `NoImagery` NÃO existe aqui: ausência de cobertura é resultado vazio, não erro.
#[derive(Debug, Error)]
pub enum ImageryAppError {
    #[error("area too large: {area:.3} deg2 exceeds limit {limit:.3}")]
    AreaTooLarge { area: f64, limit: f64 },
    #[error("source not configured: {0}")]
    SourceNotConfigured(SourceId),
    #[error("provider unavailable: {0}")]
    ProviderUnavailable(SourceId),
    #[error("provider timeout: {0}")]
    ProviderTimeout(SourceId),
    #[error("provider protocol error: {0}")]
    ProviderProtocol(String),
}

impl ImageryAppError {
    /// Fonte alternativa sugerida ao cliente em caso de falha (US3/AC2).
    #[must_use]
    pub fn suggested_source(&self) -> Option<SourceId> {
        match self {
            ImageryAppError::ProviderUnavailable(SourceId::Inpe)
            | ImageryAppError::ProviderTimeout(SourceId::Inpe) => Some(SourceId::Nasa),
            ImageryAppError::ProviderUnavailable(SourceId::Nasa | SourceId::InpeWpm)
            | ImageryAppError::ProviderTimeout(SourceId::Nasa | SourceId::InpeWpm) => {
                Some(SourceId::Inpe)
            }
            _ => None,
        }
    }
}
