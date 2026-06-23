//! Crate de composição HTTP e bootstrap da aplicação de imagery.

pub mod app_state;
pub mod bootstrap;
pub mod config;
pub mod errors;
pub mod presentation;
pub mod router;
pub mod telemetry;

pub use app_state::AppState;
pub use config::AppConfig;
pub use router::build_router;
