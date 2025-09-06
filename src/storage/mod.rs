use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info};

use crate::config::Config;
use crate::db::Bucket;

pub mod local;
pub mod smugmug;

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
            let strategy = Arc::new(smugmug::SmugMugStrategy::new(smugmug_config.clone()));
            contexts.push(StorageStrategyContext::new(strategy, "smugmug".to_string()));
            debug!("Loaded SmugMugStrategy");
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