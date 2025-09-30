use anyhow::Result;
use tempfile::tempdir;
use wupdedup::config::{Config, SmugMugConfig};
use wupdedup::db::DB;
use wupdedup::storage::load_storage_strategy_contexts;
use std::sync::Arc;

#[tokio::test]
async fn test_smugmug_mock_scan_integration() -> Result<()> {
    // Create test configuration with mock mode enabled
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");
    
    let config = Config {
        log_level: "info".to_string(),
        db_file: db_path.to_str().unwrap().to_string(),
        profile: Default::default(),
        local: None,
        smugmug: Some(SmugMugConfig {
            api_key: "mock".to_string(),
            api_secret: "mock".to_string(),
            access_token: "mock".to_string(),
            access_token_secret: "mock".to_string(),
            user_nickname: Some("test_user".to_string()),
            mock_mode: true,  // Enable mock mode
        }),
    };
    
    // Load storage strategy (should use mock)
    let mut contexts = load_storage_strategy_contexts(&config)?;
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].name, "smugmug");
    
    // Initialize database
    let db = DB::init(&config.db_file)?;
    let bucket = db.bucket("smugmug_mock_test")?;
    contexts[0].set_bucket(bucket);
    
    // Run the mock scan
    contexts[0].scan_tree().await?;
    
    // Verify results
    assert_eq!(contexts[0].node_count, 3);  // 3 mock albums
    assert_eq!(contexts[0].file_count, 16); // Total mock images
    
    // Verify data was stored
    let bucket = db.bucket("smugmug_mock_test")?;
    let count = bucket.count()?;
    assert_eq!(count, 16);
    
    // Verify we can retrieve a specific mock image
    let all_items = bucket.all()?;
    assert!(!all_items.is_empty());
    
    // Check that paths follow smugmug:// format
    for (_key, value) in all_items.iter().take(1) {
        let file_info: serde_json::Value = serde_json::from_slice(value)?;
        assert!(file_info["path"].as_str().unwrap().starts_with("smugmug://"));
        assert!(file_info["hash"].as_str().is_some());
        assert!(file_info["size"].as_u64().unwrap() > 0);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_smugmug_mock_produces_consistent_data() -> Result<()> {
    // Create two separate mock instances
    let config = SmugMugConfig {
        api_key: "test".to_string(),
        api_secret: "test".to_string(),
        access_token: "test".to_string(),
        access_token_secret: "test".to_string(),
        user_nickname: None,
        mock_mode: true,
    };
    
    let temp_dir1 = tempdir()?;
    let temp_dir2 = tempdir()?;
    
    // Run first scan
    let strategy1 = Arc::new(wupdedup::storage::smugmug_mock::MockSmugMugStrategy::new(config.clone())?);
    let mut context1 = wupdedup::storage::StorageStrategyContext::new(strategy1, "mock1".to_string());
    
    let db1 = DB::init(temp_dir1.path().join("db1.db").to_str().unwrap())?;
    let bucket1 = db1.bucket("test1")?;
    context1.set_bucket(bucket1);
    context1.scan_tree().await?;
    
    // Run second scan
    let strategy2 = Arc::new(wupdedup::storage::smugmug_mock::MockSmugMugStrategy::new(config)?);
    let mut context2 = wupdedup::storage::StorageStrategyContext::new(strategy2, "mock2".to_string());
    
    let db2 = DB::init(temp_dir2.path().join("db2.db").to_str().unwrap())?;
    let bucket2 = db2.bucket("test2")?;
    context2.set_bucket(bucket2);
    context2.scan_tree().await?;
    
    // Both should produce identical results
    assert_eq!(context1.file_count, context2.file_count);
    assert_eq!(context1.node_count, context2.node_count);
    
    Ok(())
}

#[test]
fn test_smugmug_mock_mode_config_validation() {
    // Mock mode should pass validation even with empty credentials
    let config = SmugMugConfig {
        api_key: "".to_string(),
        api_secret: "".to_string(),
        access_token: "".to_string(),
        access_token_secret: "".to_string(),
        user_nickname: None,
        mock_mode: true,
    };
    
    assert!(config.valid().is_ok());
    
    // Without mock mode, empty credentials should fail
    let config_no_mock = SmugMugConfig {
        mock_mode: false,
        ..config
    };
    
    assert!(config_no_mock.valid().is_err());
}