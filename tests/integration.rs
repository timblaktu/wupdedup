use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use wupdedup_rs::config::{Config, LocalConfig};
use wupdedup_rs::db::DB;
use wupdedup_rs::storage::local::LocalStrategy;
use wupdedup_rs::storage::{StorageStrategy, StorageStrategyContext};

fn create_test_files(dir: &Path) -> Result<()> {
    // Create test directory structure
    fs::create_dir_all(dir.join("photos"))?;
    fs::create_dir_all(dir.join("documents"))?;
    fs::create_dir_all(dir.join("duplicates"))?;
    
    // Create some test files
    fs::write(dir.join("photos/image1.jpg"), b"fake jpeg content 1")?;
    fs::write(dir.join("photos/image2.jpg"), b"fake jpeg content 2")?;
    fs::write(dir.join("documents/doc1.txt"), b"document content 1")?;
    fs::write(dir.join("documents/doc2.txt"), b"document content 2")?;
    
    // Create duplicate files
    fs::write(dir.join("duplicates/dup1.txt"), b"duplicate content")?;
    fs::write(dir.join("duplicates/dup2.txt"), b"duplicate content")?;
    
    Ok(())
}

#[tokio::test]
async fn test_full_scan_workflow() -> Result<()> {
    // Setup test environment
    let temp_dir = TempDir::new()?;
    let test_files_dir = temp_dir.path().join("test_files");
    create_test_files(&test_files_dir)?;
    
    // Create database
    let db_path = temp_dir.path().join("test.db");
    let db = DB::init(db_path.to_str().unwrap())?;
    
    // Create local storage strategy
    let local_config = LocalConfig {
        root_path: test_files_dir.clone(),
    };
    
    let strategy = std::sync::Arc::new(LocalStrategy::new(local_config));
    let mut context = StorageStrategyContext::new(strategy, "test_local".to_string());
    
    // Set up bucket
    let bucket = db.bucket("test_local")?;
    context.set_bucket(bucket);
    
    // Run scan
    context.scan_tree().await?;
    
    // Verify results
    assert!(context.file_count > 0, "Should have scanned some files");
    assert!(context.node_count > 0, "Should have scanned some nodes");
    
    // Verify files were stored in database
    let bucket = db.bucket("test_local")?;
    let count = bucket.count()?;
    assert!(count > 0, "Database should contain scanned files");
    
    Ok(())
}

#[test]
fn test_duplicate_detection() -> Result<()> {
    use blake3::Hasher;
    
    // Create identical content
    let content = b"duplicate content for testing";
    
    // Hash the content twice to simulate duplicate files
    let mut hasher1 = Hasher::new();
    hasher1.update(content);
    let hash1 = hasher1.finalize();
    
    let mut hasher2 = Hasher::new();
    hasher2.update(content);
    let hash2 = hasher2.finalize();
    
    // Verify hashes are identical
    assert_eq!(hash1, hash2, "Identical content should produce identical hashes");
    
    Ok(())
}

#[test]
fn test_different_content_different_hashes() -> Result<()> {
    use blake3::Hasher;
    
    // Create different content
    let content1 = b"content 1";
    let content2 = b"content 2";
    
    // Hash the different contents
    let mut hasher1 = Hasher::new();
    hasher1.update(content1);
    let hash1 = hasher1.finalize();
    
    let mut hasher2 = Hasher::new();
    hasher2.update(content2);
    let hash2 = hasher2.finalize();
    
    // Verify hashes are different
    assert_ne!(hash1, hash2, "Different content should produce different hashes");
    
    Ok(())
}

#[tokio::test]
async fn test_empty_directory_scan() -> Result<()> {
    // Setup test environment with empty directory
    let temp_dir = TempDir::new()?;
    let empty_dir = temp_dir.path().join("empty");
    fs::create_dir_all(&empty_dir)?;
    
    // Create database
    let db_path = temp_dir.path().join("test.db");
    let db = DB::init(db_path.to_str().unwrap())?;
    
    // Create local storage strategy
    let local_config = LocalConfig {
        root_path: empty_dir,
    };
    
    let strategy = std::sync::Arc::new(LocalStrategy::new(local_config));
    let mut context = StorageStrategyContext::new(strategy, "test_empty".to_string());
    
    // Set up bucket
    let bucket = db.bucket("test_empty")?;
    context.set_bucket(bucket);
    
    // Run scan on empty directory
    context.scan_tree().await?;
    
    // Verify no files were found
    assert_eq!(context.file_count, 0, "Empty directory should have no files");
    
    Ok(())
}

#[tokio::test]
async fn test_nested_directory_scan() -> Result<()> {
    // Setup test environment with nested directories
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path().join("nested");
    
    // Create deeply nested structure
    fs::create_dir_all(root.join("level1/level2/level3"))?;
    fs::write(root.join("level1/file1.txt"), b"level 1")?;
    fs::write(root.join("level1/level2/file2.txt"), b"level 2")?;
    fs::write(root.join("level1/level2/level3/file3.txt"), b"level 3")?;
    
    // Create database
    let db_path = temp_dir.path().join("test.db");
    let db = DB::init(db_path.to_str().unwrap())?;
    
    // Create local storage strategy
    let local_config = LocalConfig {
        root_path: root,
    };
    
    let strategy = std::sync::Arc::new(LocalStrategy::new(local_config));
    let mut context = StorageStrategyContext::new(strategy, "test_nested".to_string());
    
    // Set up bucket
    let bucket = db.bucket("test_nested")?;
    context.set_bucket(bucket);
    
    // Run scan
    context.scan_tree().await?;
    
    // Verify all files were found
    assert_eq!(context.file_count, 3, "Should find all 3 files in nested structure");
    
    Ok(())
}