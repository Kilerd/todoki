mod acp;
mod config;
mod protocol;
mod relay;
mod session;

use tracing_subscriber::EnvFilter;

use crate::config::RelayConfig;
use crate::relay::Relay;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Load config
    let config = RelayConfig::load()?;

    tracing::info!(
        name = %config.relay_name(),
        safe_paths = ?config.safe_paths(),
        "todoki-relay starting"
    );

    // Validate required config
    if config.server.url.is_none() {
        anyhow::bail!(
            "server URL not configured. Set TODOKI_SERVER_URL or configure in ~/.todoki-relay/config.toml"
        );
    }

    // Run relay with reconnection logic
    loop {
        let mut relay = Relay::new(config.clone());

        match relay.run().await {
            Ok(()) => {
                tracing::info!("relay disconnected, reconnecting...");
            }
            Err(e) => {
                tracing::error!(error = %e, "relay error, reconnecting...");
            }
        }

        // Wait before reconnecting
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
