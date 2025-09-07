use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info};

use crate::config::Config;
use crate::db::Bucket;

pub mod local;
pub mod smugmug;
pub mod smugmug_mock;

// Strategy pattern interface implemented by storage providers
#[async_trait]
pub trait StorageStrategy: Send + Sync {
    async fn scan_tree(&self, context: &mut StorageStrategyContext) -> Result<()>;
    #[allow(dead_code)]
    fn name(&self) -> &str;
}

// Context encapsulates a concrete strategy and enables calling implementations at runtime
pub struct StorageStrategyContext {
    pub storage_strategy: Arc<dyn StorageStrategy>,
    pub name: String,
    pub bucket: Option<Bucket>,
    pub file_count: usize,
    pub node_count: usize,
}

impl std::fmt::Debug for StorageStrategyContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageStrategyContext")
            .field("name", &self.name)
            .field("file_count", &self.file_count)
            .field("node_count", &self.node_count)
            .field("bucket", &self.bucket.is_some())
            .finish()
    }
}

impl StorageStrategyContext {
    pub fn new(storage_strategy: Arc<dyn StorageStrategy>, name: String) -> Self {
        Self {
            storage_strategy,
            name,
            bucket: None,
            file_count: 0,
            node_count: 0,
        }
    }

    pub fn set_bucket(&mut self, bucket: Bucket) {
        self.bucket = Some(bucket);
    }

    pub async fn scan_tree(&mut self) -> Result<()> {
        // Clone the Arc to avoid borrowing issues
        let strategy = self.storage_strategy.clone();
        strategy.scan_tree(self).await
    }
}

// Utility function to load concrete StorageStrategy instances from config
pub fn load_storage_strategy_contexts(config: &Config) -> Result<Vec<StorageStrategyContext>> {
    let mut contexts = Vec::new();

    if let Some(local_config) = &config.local {
        if local_config.specified() && local_config.valid()? {
            let strategy = Arc::new(local::LocalStrategy::new(local_config.clone()));
            contexts.push(StorageStrategyContext::new(strategy, "local".to_string()));
            debug!("Loaded LocalStrategy");
        }
    }

    if let Some(smugmug_config) = &config.smugmug {
        if smugmug_config.specified() && smugmug_config.valid()? {
            let strategy: Arc<dyn StorageStrategy> = if smugmug_config.mock_mode {
                info!("Using MOCK SmugMug strategy (no real API calls)");
                Arc::new(smugmug_mock::MockSmugMugStrategy::new(smugmug_config.clone())?)
            } else {
                Arc::new(smugmug::SmugMugStrategy::new(smugmug_config.clone())?)
            };
            contexts.push(StorageStrategyContext::new(strategy, "smugmug".to_string()));
            debug!("Loaded SmugMugStrategy (mock: {})", smugmug_config.mock_mode);
        }
    }

    if contexts.is_empty() {
        anyhow::bail!("No storage strategies specified in config");
    }

    info!("Loaded {} storage strategy contexts", contexts.len());
    for context in &contexts {
        info!("  - {}", context.name);
    }

    Ok(contexts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LocalConfig, SmugMugConfig};
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Mock implementation for testing
    struct MockStrategy {
        name: String,
        scan_called: std::sync::Mutex<bool>,
    }

    impl MockStrategy {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                scan_called: std::sync::Mutex::new(false),
            }
        }
    }

    #[async_trait]
    impl StorageStrategy for MockStrategy {
        async fn scan_tree(&self, context: &mut StorageStrategyContext) -> Result<()> {
            *self.scan_called.lock().unwrap() = true;
            context.file_count = 42;
            context.node_count = 10;
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn test_storage_context_creation() {
        let strategy = Arc::new(MockStrategy::new("test"));
        let context = StorageStrategyContext::new(strategy.clone(), "test_context".to_string());

        assert_eq!(context.name, "test_context");
        assert_eq!(context.file_count, 0);
        assert_eq!(context.node_count, 0);
        assert!(context.bucket.is_none());
    }

    #[tokio::test]
    async fn test_storage_context_scan() {
        let strategy = Arc::new(MockStrategy::new("test"));
        let mut context = StorageStrategyContext::new(strategy.clone(), "test_context".to_string());

        context.scan_tree().await.unwrap();

        assert_eq!(context.file_count, 42);
        assert_eq!(context.node_count, 10);
        assert!(*strategy.scan_called.lock().unwrap());
    }

    #[test]
    fn test_storage_context_set_bucket() {
        use crate::db::DB;
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("test").unwrap();

        let strategy = Arc::new(MockStrategy::new("test"));
        let mut context = StorageStrategyContext::new(strategy, "test_context".to_string());

        assert!(context.bucket.is_none());
        context.set_bucket(bucket);
        assert!(context.bucket.is_some());
    }

    #[test]
    fn test_load_storage_contexts_with_local() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            log_level: "info".to_string(),
            db_file: "test.db".to_string(),
            profile: Default::default(),
            local: Some(LocalConfig {
                root_path: temp_dir.path().to_path_buf(),
            }),
            smugmug: None,
        };

        let contexts = load_storage_strategy_contexts(&config).unwrap();
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].name, "local");
    }

    #[test]
    fn test_load_storage_contexts_with_smugmug() {
        let config = Config {
            log_level: "info".to_string(),
            db_file: "test.db".to_string(),
            profile: Default::default(),
            local: None,
            smugmug: Some(SmugMugConfig {
                api_key: "test_key".to_string(),
                api_secret: "test_secret".to_string(),
                access_token: "test_token".to_string(),
                access_token_secret: "test_token_secret".to_string(),
                user_nickname: Some("test_user".to_string()),
                mock_mode: false,
            }),
        };

        let contexts = load_storage_strategy_contexts(&config).unwrap();
        assert_eq!(contexts.len(), 1);
        assert_eq!(contexts[0].name, "smugmug");
    }

    #[test]
    fn test_load_storage_contexts_with_both() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config {
            log_level: "info".to_string(),
            db_file: "test.db".to_string(),
            profile: Default::default(),
            local: Some(LocalConfig {
                root_path: temp_dir.path().to_path_buf(),
            }),
            smugmug: Some(SmugMugConfig {
                api_key: "test_key".to_string(),
                api_secret: "test_secret".to_string(),
                access_token: "test_token".to_string(),
                access_token_secret: "test_token_secret".to_string(),
                user_nickname: Some("test_user".to_string()),
                mock_mode: false,
            }),
        };

        let contexts = load_storage_strategy_contexts(&config).unwrap();
        assert_eq!(contexts.len(), 2);
        assert!(contexts.iter().any(|c| c.name == "local"));
        assert!(contexts.iter().any(|c| c.name == "smugmug"));
    }

    #[test]
    fn test_load_storage_contexts_with_none_fails() {
        let config = Config {
            log_level: "info".to_string(),
            db_file: "test.db".to_string(),
            profile: Default::default(),
            local: None,
            smugmug: None,
        };

        let result = load_storage_strategy_contexts(&config);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No storage strategies"));
    }

    #[test]
    fn test_load_storage_contexts_with_invalid_local() {
        let config = Config {
            log_level: "info".to_string(),
            db_file: "test.db".to_string(),
            profile: Default::default(),
            local: Some(LocalConfig {
                root_path: PathBuf::from("/nonexistent/path"),
            }),
            smugmug: None,
        };

        let result = load_storage_strategy_contexts(&config);
        assert!(result.is_err());
    }
}
