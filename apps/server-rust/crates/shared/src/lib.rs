//! Abstrações transversais reutilizadas pelos bounded contexts do workspace.

pub mod domain;

pub use domain::error::AppError;
pub use domain::geo::{BBox, GeoError};
