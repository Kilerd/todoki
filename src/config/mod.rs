use config::{Config, ConfigError, Environment, File};
use gotcha::ConfigWrapper;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Settings {
    pub database_url: String,
    pub user_token: String,
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
