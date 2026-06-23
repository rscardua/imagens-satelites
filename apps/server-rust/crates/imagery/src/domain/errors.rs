use thiserror::Error;

/// Erros de invariantes do domínio de imagery.
#[derive(Debug, Error, PartialEq)]
pub enum ImageryDomainError {
    #[error("unknown imagery source: {0}")]
    UnknownSource(String),
    #[error("empty scene id")]
    EmptySceneId,
    #[error("empty sensor")]
    EmptySensor,
    #[error("cloud cover out of range [0,100]: {0}")]
    InvalidCloudCover(f32),
    #[error("invalid date range: start must be <= end")]
    InvalidDateRange,
}
