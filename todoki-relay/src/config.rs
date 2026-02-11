use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Relay role for task routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum RelayRole {
    #[default]
    General,
    Business,
    Coding,
    Qa,
}

impl RelayRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelayRole::General => "general",
            RelayRole::Business => "business",
            RelayRole::Coding => "coding",
            RelayRole::Qa => "qa",
        }
    }
}

/// Todoki relay agent
#[derive(Debug, Clone, Parser)]
#[command(name = "todoki-relay", version, about = "Remote agent relay for todoki")]
pub struct Args {
    /// WebSocket URL to connect to (e.g., wss://example.com/ws/relays)
    #[arg(env = "TODOKI_SERVER_URL")]
    pub url: String,

    /// Authentication token
    #[arg(env = "TODOKI_RELAY_TOKEN")]
    pub token: String,

    /// Relay name (default: hostname)
    #[arg(short, long, env = "TODOKI_RELAY_NAME")]
    pub name: Option<String>,

    /// Relay role for task routing
    #[arg(short, long, env = "TODOKI_RELAY_ROLE", default_value = "general")]
    pub role: RelayRole,

    /// Allowed working directories (comma-separated)
    #[arg(short, long, env = "TODOKI_SAFE_PATHS", value_delimiter = ',')]
    pub safe_paths: Vec<String>,

    /// Project IDs this relay is bound to (comma-separated UUIDs, empty = accept all)
    #[arg(short, long, env = "TODOKI_RELAY_PROJECTS", value_delimiter = ',')]
    pub projects: Vec<Uuid>,

    /// Labels for relay selection (format: key=value, can be repeated)
    #[arg(short, long, env = "TODOKI_RELAY_LABELS", value_parser = parse_label)]
    pub labels: Vec<(String, String)>,

    /// Path to setup script file to run before each session
    #[arg(long, env = "TODOKI_SETUP_SCRIPT_FILE")]
    pub setup_script_file: Option<PathBuf>,

    /// Path to config file
    #[arg(short, long, env = "TODOKI_CONFIG", default_value = "~/.todoki-relay/config.toml")]
    pub config: PathBuf,
}

fn parse_label(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid label format: {s}, expected key=value"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

/// File-based configuration (lower priority than CLI/env)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub relay: RelaySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    pub url: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RelaySettings {
    pub name: Option<String>,
    #[serde(default)]
    pub role: RelayRole,
    #[serde(default)]
    pub safe_paths: Vec<String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub projects: Vec<Uuid>,
    /// Path to setup script file to run before each session
    pub setup_script_file: Option<PathBuf>,
}

/// Merged configuration from CLI, env, and file
#[derive(Debug, Clone)]
pub struct RelayConfig {
    pub url: String,
    pub token: String,
    pub name: Option<String>,
    pub role: RelayRole,
    pub safe_paths: Vec<String>,
    pub labels: HashMap<String, String>,
    pub projects: Vec<Uuid>,
    pub setup_script: Option<String>,
}

impl RelayConfig {
    /// Load config by merging CLI args, env vars, and config file
    /// Priority: CLI > env > file
    pub fn load() -> anyhow::Result<Self> {
        let args = Args::parse();

        // Expand ~ in config path
        let config_path = expand_tilde(&args.config);

        // Load file config (optional)
        let file_config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            toml::from_str(&content)?
        } else {
            FileConfig::default()
        };

        // Merge: CLI/env takes precedence over file
        let name = args.name.or(file_config.relay.name);

        // For role, CLI default is "general", so we only use file if CLI wasn't explicitly set
        // Since clap doesn't distinguish "default" vs "explicitly set", we use file as fallback only if role is General
        let role = if args.role != RelayRole::General {
            args.role
        } else if file_config.relay.role != RelayRole::General {
            file_config.relay.role
        } else {
            RelayRole::General
        };

        // For vec fields, use CLI if non-empty, otherwise file
        let safe_paths = if !args.safe_paths.is_empty() {
            args.safe_paths
        } else {
            file_config.relay.safe_paths
        };

        let projects = if !args.projects.is_empty() {
            args.projects
        } else {
            file_config.relay.projects
        };

        // Merge labels: file first, then CLI overwrites
        let mut labels = file_config.relay.labels;
        for (k, v) in args.labels {
            labels.insert(k, v);
        }

        // CLI setup_script_file takes precedence over file config
        let setup_script_file = args
            .setup_script_file
            .or(file_config.relay.setup_script_file);
        let setup_script = if let Some(path) = setup_script_file {
            let expanded = expand_tilde(&path);
            match std::fs::read_to_string(&expanded) {
                Ok(content) => Some(content),
                Err(e) => {
                    tracing::warn!(path = ?expanded, error = %e, "failed to read setup script file");
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            url: args.url,
            token: args.token,
            name,
            role,
            safe_paths,
            labels,
            projects,
            setup_script,
        })
    }

    /// Get server URL
    pub fn server_url(&self) -> &str {
        &self.url
    }

    /// Get relay name, with fallback to hostname
    pub fn relay_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| {
            hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "relay".to_string())
        })
    }

    /// Get safe paths
    pub fn safe_paths(&self) -> &[String] {
        &self.safe_paths
    }

    /// Get labels
    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }

    /// Get relay role
    pub fn role(&self) -> RelayRole {
        self.role
    }

    /// Get project IDs this relay is bound to (empty = accept all)
    pub fn projects(&self) -> &[Uuid] {
        &self.projects
    }

    /// Get setup script content (if configured)
    pub fn setup_script(&self) -> Option<&str> {
        self.setup_script.as_deref()
    }
}

fn expand_tilde(path: &PathBuf) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(&path_str[2..]);
        }
    }
    path.clone()
}
