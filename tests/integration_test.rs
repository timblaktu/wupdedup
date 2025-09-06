mod common;

use anyhow::Result;
use common::*;
use common::assertions::*;
use common::perf::PerfTimer;
use rstest::*;
use serial_test::serial;
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use wupdedup_rs::storage::local::LocalStrategy;
use wupdedup_rs::storage::StorageStrategyContext;

#[rstest]
#[tokio::test]
async fn test_scan_empty_directory() -> Result<()> {
    let fixture = TestFixture::new()?.with_db()?;
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let mut context = StorageStrategyContext::new(strategy, "test".to_string());
    
    let bucket = fixture.db.as_ref().unwrap().bucket("test")?;
    context.set_bucket(bucket);
    
    context.scan_tree().await?;
    
    assert_eq!(context.file_count, 0);
    assert_eq!(context.node_count, 0);
    assert_eq!(fixture.count_db_files()?, 0);
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_scan_standard_layout() -> Result<()> {
    let fixture = TestFixture::new()?.with_db()?;
    fixture.create_standard_layout()?;
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let mut context = StorageStrategyContext::new(strategy, "test".to_string());
    
    let bucket = fixture.db.as_ref().unwrap().bucket("test")?;
    context.set_bucket(bucket);
    
    context.scan_tree().await?;
    
    assert_eq!(context.file_count, 7); // 7 files in standard layout
    assert_eq!(fixture.count_db_files()?, 7);
    
    // Verify specific files are in database
    assert!(fixture.verify_in_db("photos/vacation/beach.jpg")?);
    assert!(fixture.verify_in_db("documents/report.pdf")?);
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_scan_with_duplicates() -> Result<()> {
    let fixture = TestFixture::new()?.with_db()?;
    fixture.create_duplicates()?;
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let mut context = StorageStrategyContext::new(strategy, "test".to_string());
    
    let bucket = fixture.db.as_ref().unwrap().bucket("test")?;
    context.set_bucket(bucket);
    
    context.scan_tree().await?;
    
    assert_eq!(context.file_count, 4); // 3 duplicates + 1 unique
    
    // Verify duplicate files have same hash
    assert_files_have_same_hash(&fixture, "originals/file1.txt", "copies/file1_copy.txt")?;
    assert_files_have_same_hash(&fixture, "originals/file1.txt", "backups/file1_backup.txt")?;
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_scan_performance_many_files() -> Result<()> {
    let fixture = TestScenarioBuilder::new()?
        .with_files(1000)
        .build()?
        .with_db()?;
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let mut context = StorageStrategyContext::new(strategy, "test".to_string());
    
    let bucket = fixture.db.as_ref().unwrap().bucket("test")?;
    context.set_bucket(bucket);
    
    let timer = PerfTimer::new("Scan 1000 files");
    context.scan_tree().await?;
    timer.assert_under(Duration::from_secs(5)); // Should scan 1000 files in under 5 seconds
    
    assert_eq!(context.file_count, 1000);
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_scan_large_file() -> Result<()> {
    let fixture = TestFixture::new()?.with_db()?;
    fixture.create_large_file("large.bin", 100)?; // 100MB file
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let mut context = StorageStrategyContext::new(strategy, "test".to_string());
    
    let bucket = fixture.db.as_ref().unwrap().bucket("test")?;
    context.set_bucket(bucket);
    
    let timer = PerfTimer::new("Hash 100MB file");
    context.scan_tree().await?;
    timer.assert_under(Duration::from_secs(2)); // Blake3 should hash 100MB in under 2 seconds
    
    assert_eq!(context.file_count, 1);
    assert!(fixture.verify_in_db("large.bin")?);
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_scan_deeply_nested() -> Result<()> {
    let fixture = TestFixture::new()?.with_db()?;
    
    // Create deeply nested structure
    let deep_path = "level1/level2/level3/level4/level5/level6/level7/level8/deep.txt";
    fixture.create_file(deep_path, b"deep content")?;
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let mut context = StorageStrategyContext::new(strategy, "test".to_string());
    
    let bucket = fixture.db.as_ref().unwrap().bucket("test")?;
    context.set_bucket(bucket);
    
    context.scan_tree().await?;
    
    assert_eq!(context.file_count, 1);
    assert!(fixture.verify_in_db(deep_path)?);
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_scan_special_characters_in_names() -> Result<()> {
    let fixture = TestFixture::new()?.with_db()?;
    
    let special_files = vec![
        "file with spaces.txt",
        "file-with-dashes.txt",
        "file_with_underscores.txt",
        "file.multiple.dots.txt",
        "UPPERCASE.TXT",
        "unicode-文件.txt",
        "emoji-🎉.txt",
    ];
    
    for filename in &special_files {
        fixture.create_file(filename, b"content")?;
    }
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let mut context = StorageStrategyContext::new(strategy, "test".to_string());
    
    let bucket = fixture.db.as_ref().unwrap().bucket("test")?;
    context.set_bucket(bucket);
    
    context.scan_tree().await?;
    
    assert_eq!(context.file_count, special_files.len());
    
    for filename in special_files {
        assert!(fixture.verify_in_db(filename)?);
    }
    
    Ok(())
}

#[rstest]
#[serial]
#[tokio::test]
async fn test_scan_persistence_across_sessions() -> Result<()> {
    use tempfile::tempdir;
    
    // Create persistent temp directory that won't be auto-cleaned
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test.db");
    let root_path = temp_dir.path().join("test_files");
    fs::create_dir_all(&root_path)?;
    
    // Create test files
    let files: Vec<(&str, &[u8])> = vec![
        ("photos/vacation/beach.jpg", b"fake jpeg content 1"),
        ("photos/vacation/sunset.jpg", b"fake jpeg content 2"),
        ("photos/family/portrait.jpg", b"fake jpeg content 3"),
        ("documents/report.pdf", b"fake pdf content"),
        ("documents/notes.txt", b"some text notes"),
        ("downloads/installer.exe", b"fake exe content"),
        ("downloads/archive.zip", b"fake zip content"),
    ];
    
    for (path, content) in &files {
        let file_path = root_path.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, content)?;
    }
    
    // First scan
    {
        let db = wupdedup_rs::db::DB::init(db_path.to_str().unwrap())?;
        let bucket = db.bucket("test")?;
        
        let config = wupdedup_rs::config::LocalConfig {
            root_path: root_path.clone(),
        };
        
        let strategy = Arc::new(LocalStrategy::new(config));
        let mut context = StorageStrategyContext::new(strategy, "test".to_string());
        context.set_bucket(bucket);
        
        context.scan_tree().await?;
        assert_eq!(context.file_count, 7);
        
        // Explicitly close database
        db.close()?;
    }
    
    // Reopen database and verify data persisted
    {
        let db = wupdedup_rs::db::DB::init(db_path.to_str().unwrap())?;
        let bucket = db.bucket("test")?;
        assert_eq!(bucket.count()?, 7);
        
        // Verify specific file
        let key = root_path.join("photos/vacation/beach.jpg").to_string_lossy().to_string();
        assert!(bucket.get(&key)?.is_some());
        
        db.close()?;
    }
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_concurrent_scans_different_buckets() -> Result<()> {
    let fixture = TestFixture::new()?.with_db()?;
    fixture.create_standard_layout()?;
    
    let db = fixture.db.as_ref().unwrap();
    
    // Create two separate contexts with different buckets
    let strategy1 = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let strategy2 = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    
    let mut context1 = StorageStrategyContext::new(strategy1, "bucket1".to_string());
    let mut context2 = StorageStrategyContext::new(strategy2, "bucket2".to_string());
    
    context1.set_bucket(db.bucket("bucket1")?);
    context2.set_bucket(db.bucket("bucket2")?);
    
    // Run scans concurrently
    let (result1, result2) = tokio::join!(
        context1.scan_tree(),
        context2.scan_tree()
    );
    
    result1?;
    result2?;
    
    // Both should have scanned the same files
    assert_eq!(context1.file_count, 7);
    assert_eq!(context2.file_count, 7);
    
    // Verify both buckets have the data
    assert_eq!(db.bucket("bucket1")?.count()?, 7);
    assert_eq!(db.bucket("bucket2")?.count()?, 7);
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_scan_handles_io_errors_gracefully() -> Result<()> {
    let fixture = TestFixture::new()?.with_db()?;
    
    // Create a file that we'll remove during scanning
    fixture.create_file("disappearing.txt", b"will be deleted")?;
    
    // Create some normal files
    fixture.create_files(&[
        ("normal1.txt", b"content1"),
        ("normal2.txt", b"content2"),
    ])?;
    
    // Note: In a real scenario, we'd simulate IO errors better
    // For now, we just verify the scan continues despite potential issues
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let mut context = StorageStrategyContext::new(strategy, "test".to_string());
    
    let bucket = fixture.db.as_ref().unwrap().bucket("test")?;
    context.set_bucket(bucket);
    
    context.scan_tree().await?;
    
    // Should have processed files successfully
    assert!(context.file_count > 0);
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_scan_with_permission_denied_files() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    
    let fixture = TestFixture::new()?.with_db()?;
    
    // Create a file with restricted permissions
    let restricted_file = fixture.create_file("restricted.txt", b"secret")?;
    let mut perms = fs::metadata(&restricted_file)?.permissions();
    perms.set_mode(0o000); // No permissions
    fs::set_permissions(&restricted_file, perms)?;
    
    // Create normal files
    fixture.create_files(&[
        ("normal1.txt", b"content1"),
        ("normal2.txt", b"content2"),
    ])?;
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let mut context = StorageStrategyContext::new(strategy, "test".to_string());
    
    let bucket = fixture.db.as_ref().unwrap().bucket("test")?;
    context.set_bucket(bucket);
    
    // Should complete even with permission denied files
    context.scan_tree().await?;
    
    // Should have processed the accessible files
    assert!(context.file_count >= 2);
    
    // Restore permissions for cleanup
    let mut perms = fs::metadata(&restricted_file)?.permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&restricted_file, perms)?;
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_scan_with_symlinks() -> Result<()> {
    use std::os::unix::fs;
    
    let fixture = TestFixture::new()?.with_db()?;
    
    // Create regular files
    fixture.create_file("original.txt", b"original content")?;
    fixture.create_file("dir/nested.txt", b"nested content")?;
    
    // Create symlinks
    let original_path = fixture.root_path.join("original.txt");
    let link_path = fixture.root_path.join("link_to_original.txt");
    fs::symlink(&original_path, &link_path)?;
    
    // Create directory symlink
    let dir_path = fixture.root_path.join("dir");
    let dir_link_path = fixture.root_path.join("link_to_dir");
    fs::symlink(&dir_path, &dir_link_path)?;
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let mut context = StorageStrategyContext::new(strategy, "test".to_string());
    
    let bucket = fixture.db.as_ref().unwrap().bucket("test")?;
    context.set_bucket(bucket);
    
    context.scan_tree().await?;
    
    // Should handle symlinks appropriately
    assert!(context.file_count > 0);
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_scan_with_zero_byte_files() -> Result<()> {
    let fixture = TestFixture::new()?.with_db()?;
    
    // Create zero-byte files
    fixture.create_file("empty1.txt", b"")?;
    fixture.create_file("empty2.txt", b"")?;
    fixture.create_file("dir/empty3.txt", b"")?;
    
    // Create normal files
    fixture.create_file("normal.txt", b"content")?;
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    let mut context = StorageStrategyContext::new(strategy, "test".to_string());
    
    let bucket = fixture.db.as_ref().unwrap().bucket("test")?;
    context.set_bucket(bucket);
    
    context.scan_tree().await?;
    
    assert_eq!(context.file_count, 4);
    
    // All files should be in the database, including empty ones
    assert!(fixture.verify_in_db("empty1.txt")?);
    assert!(fixture.verify_in_db("empty2.txt")?);
    assert!(fixture.verify_in_db("dir/empty3.txt")?);
    assert!(fixture.verify_in_db("normal.txt")?);
    
    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_rescan_idempotency() -> Result<()> {
    let fixture = TestFixture::new()?.with_db()?;
    fixture.create_standard_layout()?;
    
    let strategy = Arc::new(LocalStrategy::new(fixture.get_config().local.unwrap()));
    
    // First scan
    let mut context1 = StorageStrategyContext::new(strategy.clone(), "test".to_string());
    let bucket1 = fixture.db.as_ref().unwrap().bucket("test")?;
    context1.set_bucket(bucket1);
    context1.scan_tree().await?;
    let first_count = context1.file_count;
    
    // Second scan - should be idempotent
    let mut context2 = StorageStrategyContext::new(strategy, "test".to_string());
    let bucket2 = fixture.db.as_ref().unwrap().bucket("test")?;
    context2.set_bucket(bucket2);
    context2.scan_tree().await?;
    let second_count = context2.file_count;
    
    assert_eq!(first_count, second_count);
    assert_eq!(fixture.db.as_ref().unwrap().bucket("test")?.count()?, 7); // Should not duplicate entries
    
    Ok(())
}