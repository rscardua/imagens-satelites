use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Carrega .env (do diretório atual ou ancestrais) se existir; não sobrescreve env já definido.
    let _ = dotenvy::dotenv();

    server::telemetry::init();

    let config = server::AppConfig::from_env()?;
    let bind_addr = config.bind_addr.clone();

    let state = server::bootstrap::build_state(config).await?;
    let app = server::build_router(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("imagery server listening on {bind_addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
