use tracing_subscriber::EnvFilter;

use todoki_relay::config::RelayConfig;
use todoki_relay::relay::Relay;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Load config
    let config = RelayConfig::load()?;

    tracing::info!(
        name = %config.relay_name(),
        role = %config.role().as_str(),
        safe_paths = ?config.safe_paths(),
        projects = ?config.projects(),
        "todoki-relay starting"
    );

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
