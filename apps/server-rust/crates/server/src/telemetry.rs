use tracing_subscriber::EnvFilter;

/// Inicializa o tracing global com filtro por `RUST_LOG` (§13).
pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,imagery=debug,server=debug"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
