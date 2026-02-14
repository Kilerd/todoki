use config::{Config, ConfigError, Environment, File};
use gotcha::ConfigWrapper;
use serde::{Deserialize, Serialize};
use std::env;

/// Configuration for AI-based permission auto-review
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AutoReviewConfig {
    /// Enable auto-review (default: false)
    #[serde(default)]
    pub enabled: bool,
    /// OpenAI API key
    #[serde(default)]
    pub openai_api_key: String,
    /// Model to use (default: gpt-4o-mini)
    #[serde(default = "default_model")]
    pub model: String,
    /// Optional OpenAI base URL (for proxies or compatible APIs)
    #[serde(default)]
    pub openai_base_url: Option<String>,
    /// Timeout in seconds (default: 30)
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_timeout() -> u64 {
    30
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Settings {
    pub database_url: String,
    pub user_token: String,
    /// Token for relay authentication (can be same as user_token or separate)
    #[serde(default)]
    pub relay_token: String,
    /// Auto-review configuration
    #[serde(default)]
    pub auto_review: AutoReviewConfig,
}

impl Settings {
    pub fn new() -> Result<ConfigWrapper<Self>, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            // Start with defaults
            .add_source(File::with_name("config/default").required(false))
            // Add environment-specific file
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            // Add local configuration file (not tracked by git)
            .add_source(File::with_name("config/local").required(false))
            // Add in settings from environment variables (with prefix TODOKI)
            .add_source(Environment::with_prefix("TODOKI").separator("_"))
            .build()?;

        s.try_deserialize()
    }
}
