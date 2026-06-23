use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;

use super::errors::ImageryDomainError;

/// Fonte de imagens suportada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    /// INPE — CBERS-4A via catálogo STAC.
    Inpe,
    /// NASA — GIBS via tiles WMTS.
    Nasa,
}

impl SourceId {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SourceId::Inpe => "inpe",
            SourceId::Nasa => "nasa",
        }
    }
}

impl std::fmt::Display for SourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Converte uma string externa em [`SourceId`].
///
/// # Examples
/// ```
/// use imagery::SourceId;
/// assert_eq!(SourceId::try_from("inpe").map(|s| s.as_str()), Ok("inpe"));
/// assert!(SourceId::try_from("foo").is_err());
/// ```
impl TryFrom<&str> for SourceId {
    type Error = ImageryDomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "inpe" => Ok(SourceId::Inpe),
            "nasa" => Ok(SourceId::Nasa),
            other => Err(ImageryDomainError::UnknownSource(other.to_owned())),
        }
    }
}

/// Identificador não-vazio de uma cena (id do Item STAC).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SceneId(String);

impl SceneId {
    /// # Errors
    /// Falha se a string for vazia.
    pub fn new(value: impl Into<String>) -> Result<Self, ImageryDomainError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ImageryDomainError::EmptySceneId);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Sensor de origem (ex.: WFI, MUX, WPM).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sensor(String);

impl Sensor {
    /// # Errors
    /// Falha se a string for vazia.
    pub fn new(value: impl Into<String>) -> Result<Self, ImageryDomainError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ImageryDomainError::EmptySensor);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Cobertura de nuvens em porcentagem `[0,100]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloudCover(f32);

impl CloudCover {
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl TryFrom<f32> for CloudCover {
    type Error = ImageryDomainError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if (0.0..=100.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(ImageryDomainError::InvalidCloudCover(value))
        }
    }
}

/// Intervalo temporal fechado de aquisição.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DateRange {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl DateRange {
    /// # Errors
    /// Falha se `start > end`.
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Self, ImageryDomainError> {
        if start > end {
            return Err(ImageryDomainError::InvalidDateRange);
        }
        Ok(Self { start, end })
    }

    #[must_use]
    pub fn start(&self) -> DateTime<Utc> {
        self.start
    }
    #[must_use]
    pub fn end(&self) -> DateTime<Utc> {
        self.end
    }
}

/// Footprint geográfico da cena (geometria `GeoJSON` crua + nada além disso na v1).
#[derive(Debug, Clone, PartialEq)]
pub struct Footprint {
    geometry: JsonValue,
}

impl Footprint {
    #[must_use]
    pub fn new(geometry: JsonValue) -> Self {
        Self { geometry }
    }

    #[must_use]
    pub fn geometry(&self) -> &JsonValue {
        &self.geometry
    }
}

/// Cena de satélite disponível (apenas em memória; não persistida — FR-015).
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub id: SceneId,
    pub acquired_at: DateTime<Utc>,
    pub source: SourceId,
    pub sensor: Sensor,
    pub footprint: Footprint,
    pub cloud_cover: Option<CloudCover>,
    /// Indica se há asset de preview/thumbnail disponível para overlay (R3).
    pub has_preview: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_roundtrip() {
        assert_eq!(SourceId::try_from("inpe"), Ok(SourceId::Inpe));
        assert_eq!(SourceId::try_from("nasa"), Ok(SourceId::Nasa));
        assert!(SourceId::try_from("foo").is_err());
    }

    #[test]
    fn scene_id_rejects_empty() {
        assert!(SceneId::new("  ").is_err());
        assert!(SceneId::new("abc").is_ok());
    }

    #[test]
    fn cloud_cover_bounds() {
        assert!(CloudCover::try_from(0.0).is_ok());
        assert!(CloudCover::try_from(100.0).is_ok());
        assert!(CloudCover::try_from(101.0).is_err());
    }
}
