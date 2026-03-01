use std::fs::File;

use tracing_subscriber::EnvFilter;

use todoki_relay::config::{DaemonArgs, RelayConfig};
use todoki_relay::relay::Relay;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse daemon args first (before full config load)
    let daemon_args = DaemonArgs::parse_daemon_args();

    // Daemonize if requested (must be done before logging init)
    if daemon_args.daemonize {
        let stdout = File::create(&daemon_args.log_file)?;
        let stderr = stdout.try_clone()?;

        daemonize::Daemonize::new()
            .pid_file(&daemon_args.pid_file)
            .working_directory(std::env::current_dir()?)
            .stdout(stdout)
            .stderr(stderr)
            .start()?;
    }

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
