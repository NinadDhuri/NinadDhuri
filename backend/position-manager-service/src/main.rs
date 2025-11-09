mod service;

use anyhow::Result;
use service::api;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (addr, server) = api::build_router().await?;
    tracing::info!(%addr, "starting position manager api");

    tokio::select! {
        result = server => {
            result?;
        }
        _ = signal::ctrl_c() => {
            tracing::warn!("ctrl_c received, shutting down");
        }
    }

    Ok(())
}
