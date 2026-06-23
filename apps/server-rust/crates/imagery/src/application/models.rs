use chrono::NaiveDate;
use shared::BBox;

use crate::domain::models::{DateRange, Scene, SceneId, SourceId};

/// Entrada de uma busca vinda da camada de apresentação.
#[derive(Debug, Clone)]
pub struct SearchInput {
    pub bbox: BBox,
    pub range: Option<DateRange>,
    pub source: SourceId,
    pub limit: Option<u16>,
}

/// Consulta normalizada repassada ao provider (range já resolvido).
#[derive(Debug, Clone)]
pub struct AreaQuery {
    pub bbox: BBox,
    pub range: DateRange,
    pub source: SourceId,
    pub limit: u16,
}

/// Descritor de camada de tiles (NASA GIBS) — não é uma cena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileLayerDescriptor {
    pub source: SourceId,
    pub url_template: String,
    pub layer: String,
    pub date: NaiveDate,
    pub max_zoom: u8,
    pub attribution: String,
}

/// Resultado da busca (apenas em memória).
#[derive(Debug, Clone)]
pub struct ImagerySearchResult {
    pub source: SourceId,
    pub scenes: Vec<Scene>,
    pub tile_layer: Option<TileLayerDescriptor>,
    pub prioritized: Option<SceneId>,
    pub truncated: bool,
}
