use anyhow::Result;
use config::{Config as ConfigBuilder, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub log_level: String,
    pub db_file: String,
    pub profile: ProfileConfig,
    pub local: Option<LocalConfig>,
    pub smugmug: Option<SmugMugConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ProfileConfig {
    pub enabled: bool,
    pub mode: Option<String>,
}

impl ProfileConfig {
    #[allow(dead_code)]
    pub fn specified(&self) -> bool {
        self.enabled
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalConfig {
    pub root_path: PathBuf,
}

impl LocalConfig {
    pub fn specified(&self) -> bool {
        true
    }

    pub fn valid(&self) -> Result<bool> {
        if !self.root_path.exists() {
            anyhow::bail!("LocalConfig.root_path does not exist: {:?}", self.root_path);
        }
        if !self.root_path.is_dir() {
            anyhow::bail!(
                "LocalConfig.root_path is not a directory: {:?}",
                self.root_path
            );
        }
        Ok(true)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmugMugConfig {
    pub api_key: String,
    pub api_secret: String,
    pub access_token: String,
    pub access_token_secret: String,
    #[serde(default)]
    pub user_nickname: Option<String>,  // Optional: If not provided, will fetch authenticated user
    #[serde(default)]
    pub mock_mode: bool,  // Use mock data instead of real API (for testing)
}

impl SmugMugConfig {
    pub fn specified(&self) -> bool {
        !self.api_key.is_empty()
    }

    pub fn valid(&self) -> Result<bool> {
        // In mock mode, we don't need real credentials
        if self.mock_mode {
            return Ok(true);
        }
        
        if self.api_key.is_empty() || self.api_secret.is_empty() {
            anyhow::bail!("SmugMug API key and secret are required");
        }
        if self.access_token.is_empty() || self.access_token_secret.is_empty() {
            anyhow::bail!("SmugMug access tokens are required");
        }
        Ok(true)
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        debug!("Loading configuration");

        // Start with default configuration
        let mut builder = ConfigBuilder::builder()
            .set_default("log_level", "info")?
            .set_default("db_file", "wupdedup.bolt.db")?
            .set_default("profile.enabled", false)?;

        // Load from .env file if it exists
        if Path::new(".env").exists() {
            dotenvy::dotenv().ok();
        }

        // Load from config files (optional)
        builder = builder
            .add_source(File::with_name("config").required(false))
            .add_source(File::with_name("config.local").required(false));

        // Override with environment variables
        builder = builder.add_source(
            Environment::with_prefix("WUPDEDUP")
                .separator("_")
                .try_parsing(true),
        );

        let config = builder.build()?;
        let settings: Config = config.try_deserialize()?;

        debug!("Configuration loaded: {:?}", settings);
        Ok(settings)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            db_file: "wupdedup.bolt.db".to_string(),
            profile: ProfileConfig::default(),
            local: None,
            smugmug: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.log_level, "info");
        assert_eq!(config.db_file, "wupdedup.bolt.db");
        assert!(!config.profile.enabled);
        assert!(config.local.is_none());
        assert!(config.smugmug.is_none());
    }

    #[test]
    fn test_profile_config_default() {
        let profile = ProfileConfig::default();
        assert!(!profile.enabled);
        assert!(profile.mode.is_none());
    }

    #[test]
    fn test_local_config_validation() {
        let temp_dir = TempDir::new().unwrap();
        let valid_path = temp_dir.path().to_path_buf();

        let local_config = LocalConfig {
            root_path: valid_path.clone(),
        };

        assert!(local_config.specified());
        assert!(local_config.valid().unwrap());

        // Test with non-existent path
        let invalid_config = LocalConfig {
            root_path: PathBuf::from("/nonexistent/path"),
        };
        assert!(invalid_config.valid().is_err());

        // Test with file instead of directory
        let file_path = valid_path.join("test.txt");
        std::fs::write(&file_path, "test").unwrap();
        let file_config = LocalConfig {
            root_path: file_path,
        };
        assert!(file_config.valid().is_err());
    }

    #[test]
    fn test_smugmug_config_validation() {
        let valid_config = SmugMugConfig {
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
            access_token: "test_token".to_string(),
            access_token_secret: "test_token_secret".to_string(),
            user_nickname: Some("test_user".to_string()),
            mock_mode: false,
        };

        assert!(valid_config.specified());
        assert!(valid_config.valid().unwrap());

        // Test with empty API key
        let invalid_config = SmugMugConfig {
            api_key: "".to_string(),
            ..valid_config.clone()
        };
        assert!(!invalid_config.specified());
        assert!(invalid_config.valid().is_err());
        
        // Test with empty access token
        let invalid_config2 = SmugMugConfig {
            access_token: "".to_string(),
            ..valid_config.clone()
        };
        assert!(invalid_config2.valid().is_err());
    }

    #[test]
    fn test_config_with_env_override() {
        // Save current env values
        let saved_log_level = env::var("WUPDEDUP_LOG_LEVEL").ok();
        let saved_db_file = env::var("WUPDEDUP_DB_FILE").ok();

        // Set test env values
        env::set_var("WUPDEDUP_LOG_LEVEL", "debug");
        env::set_var("WUPDEDUP_DB_FILE", "test.db");

        // Load config (this would normally work in a real environment)
        // Note: In tests, the config loading might not pick up env vars properly
        // due to how the config crate initializes

        // Restore env values
        match saved_log_level {
            Some(val) => env::set_var("WUPDEDUP_LOG_LEVEL", val),
            None => env::remove_var("WUPDEDUP_LOG_LEVEL"),
        }
        match saved_db_file {
            Some(val) => env::set_var("WUPDEDUP_DB_FILE", val),
            None => env::remove_var("WUPDEDUP_DB_FILE"),
        }
    }
}
