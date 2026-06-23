use std::time::Duration;

use anyhow::{Context, Result};

/// Configuração da aplicação, validada no startup (§12).
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: String,
    pub inpe_base_url: String,
    pub inpe_collections: Vec<String>,
    pub gibs_base_url: String,
    pub gibs_layer: String,
    pub provider_timeout: Duration,
    pub cache_ttl: Duration,
    pub cache_capacity: u64,
    pub result_limit: u16,
    pub max_area_deg2: f64,
    pub default_window_days: i64,
    pub cors_allow_origin: String,
    pub validate_collections: bool,
}

impl AppConfig {
    /// Lê e valida a configuração a partir do ambiente.
    ///
    /// # Errors
    /// Falha se uma variável obrigatória estiver ausente ou malformada.
    pub fn from_env() -> Result<Self> {
        let inpe_collections: Vec<String> = required("IMAGERY_INPE_STAC_COLLECTIONS")?
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        anyhow::ensure!(
            !inpe_collections.is_empty(),
            "IMAGERY_INPE_STAC_COLLECTIONS must list at least one collection id"
        );

        Ok(Self {
            bind_addr: optional("HTTP_BIND_ADDR", "0.0.0.0:8080"),
            inpe_base_url: required("IMAGERY_INPE_STAC_BASE_URL")?,
            inpe_collections,
            gibs_base_url: optional(
                "IMAGERY_NASA_GIBS_BASE_URL",
                "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best",
            ),
            gibs_layer: optional(
                "IMAGERY_NASA_GIBS_LAYER",
                "VIIRS_NOAA20_CorrectedReflectance_TrueColor",
            ),
            provider_timeout: Duration::from_secs(parse("IMAGERY_PROVIDER_TIMEOUT_SECS", 8)?),
            cache_ttl: Duration::from_secs(parse("IMAGERY_CACHE_TTL_SECS", 60)?),
            cache_capacity: parse("IMAGERY_CACHE_CAPACITY", 1024)?,
            result_limit: parse("IMAGERY_RESULT_LIMIT", 50)?,
            max_area_deg2: parse_f64("IMAGERY_MAX_AREA_DEG2", 4.0)?,
            default_window_days: parse("IMAGERY_DEFAULT_WINDOW_DAYS", 30)?,
            cors_allow_origin: optional("CORS_ALLOW_ORIGIN", "*"),
            validate_collections: optional("IMAGERY_VALIDATE_COLLECTIONS", "true") != "false",
        })
    }
}

fn required(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("missing required env var: {key}"))
}

fn optional(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_owned())
}

fn parse<T: std::str::FromStr>(key: &str, default: T) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(v) => v
            .parse::<T>()
            .map_err(|e| anyhow::anyhow!("invalid {key}: {e}")),
        Err(_) => Ok(default),
    }
}

fn parse_f64(key: &str, default: f64) -> Result<f64> {
    parse::<f64>(key, default)
}
