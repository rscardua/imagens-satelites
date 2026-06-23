use thiserror::Error;

/// Erro transversal mínimo para abstrações compartilhadas.
///
/// Cada bounded context define seus próprios erros tipados; este tipo cobre
/// apenas falhas genéricas de invariantes compartilhadas.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AppError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
}
