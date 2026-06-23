use thiserror::Error;

/// Erros de construção de tipos geográficos.
#[derive(Debug, Error, PartialEq)]
pub enum GeoError {
    #[error("longitude out of range [-180,180]: {0}")]
    LongitudeOutOfRange(f64),
    #[error("latitude out of range [-90,90]: {0}")]
    LatitudeOutOfRange(f64),
    #[error("degenerate bbox: min must be strictly less than max")]
    DegenerateBBox,
}

/// Bounding box em graus decimais (WGS84 / EPSG:4326).
///
/// Estados inválidos são irrepresentáveis: só construível via [`BBox::new`],
/// que valida limites e não-degenerescência.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox {
    min_lon: f64,
    min_lat: f64,
    max_lon: f64,
    max_lat: f64,
}

impl BBox {
    /// Constrói uma bbox validada.
    ///
    /// # Examples
    /// ```
    /// # fn main() -> Result<(), shared::GeoError> {
    /// use shared::BBox;
    /// let b = BBox::new(-48.0, -16.0, -46.0, -14.0)?;
    /// assert!((b.area_deg2() - 4.0).abs() < 1e-9);
    /// assert!(BBox::new(1.0, 1.0, 1.0, 2.0).is_err());
    /// # Ok(()) }
    /// ```
    ///
    /// # Errors
    /// Falha se lon/lat saírem dos limites ou se `min >= max` em qualquer eixo.
    pub fn new(min_lon: f64, min_lat: f64, max_lon: f64, max_lat: f64) -> Result<Self, GeoError> {
        for lon in [min_lon, max_lon] {
            if !(-180.0..=180.0).contains(&lon) {
                return Err(GeoError::LongitudeOutOfRange(lon));
            }
        }
        for lat in [min_lat, max_lat] {
            if !(-90.0..=90.0).contains(&lat) {
                return Err(GeoError::LatitudeOutOfRange(lat));
            }
        }
        if min_lon >= max_lon || min_lat >= max_lat {
            return Err(GeoError::DegenerateBBox);
        }
        Ok(Self {
            min_lon,
            min_lat,
            max_lon,
            max_lat,
        })
    }

    #[must_use]
    pub fn min_lon(&self) -> f64 {
        self.min_lon
    }
    #[must_use]
    pub fn min_lat(&self) -> f64 {
        self.min_lat
    }
    #[must_use]
    pub fn max_lon(&self) -> f64 {
        self.max_lon
    }
    #[must_use]
    pub fn max_lat(&self) -> f64 {
        self.max_lat
    }

    /// Área aproximada da bbox em graus quadrados (suficiente para guardas de tamanho).
    #[must_use]
    pub fn area_deg2(&self) -> f64 {
        (self.max_lon - self.min_lon) * (self.max_lat - self.min_lat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_out_of_range_longitude() {
        assert_eq!(
            BBox::new(-200.0, 0.0, 1.0, 1.0),
            Err(GeoError::LongitudeOutOfRange(-200.0))
        );
    }

    #[test]
    fn rejects_degenerate() {
        assert_eq!(BBox::new(1.0, 1.0, 1.0, 2.0), Err(GeoError::DegenerateBBox));
    }

    #[test]
    fn area_is_product_of_spans() {
        let b = BBox::new(-48.0, -16.0, -46.0, -14.0).expect("valid");
        assert!((b.area_deg2() - 4.0).abs() < 1e-9);
    }
}
