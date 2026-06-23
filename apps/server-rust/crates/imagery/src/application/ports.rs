use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use thiserror::Error;

use super::models::{AreaQuery, ImagerySearchResult, TileLayerDescriptor};
use crate::domain::models::{Scene, SceneId, SourceId};

/// Tipo de asset visual de uma cena.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Thumbnail,
    Preview,
}

/// Payload binário de um asset (proxy ao cliente).
#[derive(Debug, Clone)]
pub struct AssetPayload {
    pub content_type: String,
    pub bytes: Bytes,
}

/// Resposta de um provider externo.
#[derive(Debug)]
pub enum ProviderResponse {
    /// Cenas individuais (ex.: INPE STAC).
    Scenes(Vec<Scene>),
    /// Camada de tiles (ex.: NASA GIBS).
    TileLayer(TileLayerDescriptor),
}

/// Erro técnico de um provider (não confundir com "sem cobertura").
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider unavailable")]
    Unavailable,
    #[error("provider timeout")]
    Timeout,
    #[error("protocol error: {0}")]
    Protocol(String),
}

/// Port de integração com uma fonte de imagens (Estratégia B — dispatch dinâmico).
///
/// Object-safe: registrado como `Arc<dyn ImageryProvider>` por `SourceId` no
/// bootstrap e selecionado em runtime pelo caso de uso.
#[async_trait]
pub trait ImageryProvider: Send + Sync {
    /// Fonte que este provider atende.
    fn source(&self) -> SourceId;

    /// Busca cenas/camada. Vazio = sem cobertura (não é erro). Não aplica
    /// priorização nem limite de área — isso é do caso de uso.
    async fn search(&self, query: &AreaQuery) -> Result<ProviderResponse, ProviderError>;

    /// Stream/bytes do asset visual de uma cena (proxy).
    async fn fetch_asset(
        &self,
        scene_id: &SceneId,
        kind: AssetKind,
    ) -> Result<AssetPayload, ProviderError>;
}

/// Port de proxy de tiles (ex.: NASA GIBS). Separado de [`ImageryProvider`] porque
/// nem toda fonte serve tiles (object-safe; `Arc<dyn TileProvider>` por `SourceId`).
#[async_trait]
pub trait TileProvider: Send + Sync {
    fn source(&self) -> SourceId;

    /// Busca um tile (proxy). `layer`/`date` vêm do descritor entregue ao cliente.
    async fn fetch_tile(
        &self,
        z: u32,
        x: u32,
        y: u32,
        layer: &str,
        date: &str,
    ) -> Result<AssetPayload, ProviderError>;
}

/// Cache de curta duração dos metadados de busca (sem persistência durável — FR-015).
#[async_trait]
pub trait ImageryCache: Send + Sync {
    async fn get(&self, key: &str) -> Option<ImagerySearchResult>;
    async fn put(&self, key: String, value: ImagerySearchResult);
}

/// Relógio injetável (default de janela temporal + chave de cache).
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}
