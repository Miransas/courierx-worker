mod config;
mod db;
mod error;
mod models;
mod provider;
mod worker;

use std::sync::Arc;

use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::provider::EmailProvider;
use crate::provider::stdout::StdoutProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env()?;
    let pool = db::connect(&cfg.database_url).await?;
    db::check_schema_version(&pool, cfg.min_schema_version).await?;
    let provider: Arc<dyn EmailProvider> = Arc::new(StdoutProvider);

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let worker_handle = {
        let pool = pool.clone();
        let provider = provider.clone();
        let cfg = cfg.clone();
        tokio::spawn(async move { worker::run(pool, provider, cfg, shutdown_rx).await })
    };

    shutdown_signal().await;
    tracing::info!("shutdown signal received");
    let _ = shutdown_tx.send(true);

    match worker_handle.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::error!(error = %e, "worker returned error"),
        Err(e) => tracing::error!(error = %e, "worker task panicked"),
    }

    pool.close().await;
    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("install ctrl-c handler");
}
