mod common;

use anyhow::Result;
use proptest::prelude::*;
use std::collections::HashMap;
use tempfile::tempdir;
use wupdedup_rs::config::LocalConfig;
use wupdedup_rs::storage::local::LocalStrategy;

// Property: Hash function is deterministic
proptest! {
    #[test]
    fn prop_hash_deterministic(
        content in prop::collection::vec(any::<u8>(), 0..10000)
    ) {
        use blake3::Hasher;
        
        let mut hasher1 = Hasher::new();
        hasher1.update(&content);
        let hash1 = hasher1.finalize();
        
        let mut hasher2 = Hasher::new();
        hasher2.update(&content);
        let hash2 = hasher2.finalize();
        
        prop_assert_eq!(hash1, hash2, "Same content should produce same hash");
    }
}

// Property: Different content produces different hashes (with high probability)
proptest! {
    #[test]
    fn prop_hash_collision_resistance(
        content1 in prop::collection::vec(any::<u8>(), 1..1000),
        content2 in prop::collection::vec(any::<u8>(), 1..1000)
    ) {
        prop_assume!(content1 != content2);
        
        use blake3::Hasher;
        
        let mut hasher1 = Hasher::new();
        hasher1.update(&content1);
        let hash1 = hasher1.finalize();
        
        let mut hasher2 = Hasher::new();
        hasher2.update(&content2);
        let hash2 = hasher2.finalize();
        
        prop_assert_ne!(hash1, hash2, "Different content should produce different hashes");
    }
}

// Property: Database operations are consistent
proptest! {
    #[test]
    fn prop_db_consistency(
        operations in prop::collection::vec(
            (
                "[a-z]{1,20}",  // key
                prop::collection::vec(any::<u8>(), 0..100)  // value
            ),
            1..50
        )
    ) {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let db = wupdedup_rs::db::DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("test").unwrap();
        
        let mut expected = HashMap::new();
        
        // Apply all operations
        for (key, value) in &operations {
            bucket.put(key, value).unwrap();
            expected.insert(key.clone(), value.clone());
        }
        
        // Verify all values match expected
        for (key, expected_value) in &expected {
            let actual = bucket.get(key).unwrap();
            prop_assert_eq!(actual.as_ref(), Some(expected_value));
        }
        
        // Verify count matches
        prop_assert_eq!(bucket.count().unwrap(), expected.len());
    }
}

// Property: File path handling is robust
proptest! {
    #[test]
    fn prop_path_handling(
        path_components in prop::collection::vec(
            "[a-zA-Z0-9_-]{1,20}",
            1..5
        )
    ) {
        let temp_dir = tempdir().unwrap();
        let file_path = path_components.iter()
            .fold(temp_dir.path().to_path_buf(), |acc, comp| acc.join(comp));
        
        // Ensure parent directories exist
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        
        // Write test file
        std::fs::write(&file_path, b"test content").unwrap();
        
        // Verify file can be read back
        let content = std::fs::read(&file_path).unwrap();
        prop_assert_eq!(content, b"test content");
    }
}

// Property: Concurrent operations maintain consistency
proptest! {
    #[test]
    fn prop_concurrent_consistency(
        num_threads in 2..10usize,
        operations_per_thread in 5..20usize
    ) {
        use std::sync::Arc;
        use std::thread;
        
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Arc::new(wupdedup_rs::db::DB::init(db_path.to_str().unwrap()).unwrap());
        
        let mut handles = vec![];
        
        for thread_id in 0..num_threads {
            let db_clone = Arc::clone(&db);
            let handle = thread::spawn(move || {
                let bucket = db_clone.bucket(&format!("bucket_{}", thread_id)).unwrap();
                
                for op_id in 0..operations_per_thread {
                    let key = format!("key_{}", op_id);
                    let value = format!("thread_{}_op_{}", thread_id, op_id);
                    bucket.put(&key, value.as_bytes()).unwrap();
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify each bucket has the expected data
        for thread_id in 0..num_threads {
            let bucket = db.bucket(&format!("bucket_{}", thread_id)).unwrap();
            prop_assert_eq!(bucket.count().unwrap(), operations_per_thread);
            
            for op_id in 0..operations_per_thread {
                let key = format!("key_{}", op_id);
                let expected = format!("thread_{}_op_{}", thread_id, op_id);
                let actual = bucket.get(&key).unwrap().unwrap();
                prop_assert_eq!(actual, expected.as_bytes());
            }
        }
    }
}

// Property: File size calculations are accurate
proptest! {
    #[test]
    fn prop_file_size_accuracy(
        content in prop::collection::vec(any::<u8>(), 0..100000)
    ) {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.bin");
        
        std::fs::write(&file_path, &content).unwrap();
        
        let metadata = std::fs::metadata(&file_path).unwrap();
        prop_assert_eq!(metadata.len() as usize, content.len());
    }
}

// Property: MIME type detection never panics
proptest! {
    #[test]
    fn prop_mime_type_safety(
        content in prop::collection::vec(any::<u8>(), 0..1000),
        extension in "[a-z]{0,5}"
    ) {
        let temp_dir = tempdir().unwrap();
        let filename = if extension.is_empty() {
            "test".to_string()
        } else {
            format!("test.{}", extension)
        };
        let file_path = temp_dir.path().join(filename);
        
        std::fs::write(&file_path, &content).unwrap();
        
        // Should never panic
        let mime_type = wupdedup_rs::content::get_type(&file_path).unwrap();
        
        // Should always return a valid MIME type
        prop_assert!(!mime_type.is_empty());
        prop_assert!(mime_type.contains('/'), "MIME type should contain '/'");
    }
}

// Property: Scan results are reproducible
#[test]
#[ignore] // This test is slow and async, run with --ignored flag
fn prop_scan_reproducible() {
    use std::sync::Arc;
    use wupdedup_rs::storage::{StorageStrategy, StorageStrategyContext};
    
    // Note: This is a simplified version that doesn't use proptest due to async complexity
    // For production, consider using proptest-tokio or similar
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    rt.block_on(async {
        let temp_dir = tempdir().unwrap();
        
        // Create test files
        for i in 0..10 {
            let file_path = temp_dir.path().join(format!("file_{}.dat", i));
            std::fs::write(&file_path, format!("content {}", i).as_bytes()).unwrap();
        }
        
        // First scan
        let db_path1 = temp_dir.path().join("db1.db");
        let db1 = wupdedup_rs::db::DB::init(db_path1.to_str().unwrap()).unwrap();
        let bucket1 = db1.bucket("test").unwrap();
        
        let config = LocalConfig {
            root_path: temp_dir.path().to_path_buf(),
        };
        
        let strategy1 = Arc::new(LocalStrategy::new(config.clone()));
        let mut context1 = StorageStrategyContext::new(strategy1, "test".to_string());
        context1.set_bucket(bucket1);
        context1.scan_tree().await.unwrap();
        
        // Second scan
        let db_path2 = temp_dir.path().join("db2.db");
        let db2 = wupdedup_rs::db::DB::init(db_path2.to_str().unwrap()).unwrap();
        let bucket2 = db2.bucket("test").unwrap();
        
        let strategy2 = Arc::new(LocalStrategy::new(config));
        let mut context2 = StorageStrategyContext::new(strategy2, "test".to_string());
        context2.set_bucket(bucket2);
        context2.scan_tree().await.unwrap();
        
        // Results should be identical
        assert_eq!(context1.file_count, context2.file_count);
        assert_eq!(context1.node_count, context2.node_count);
    });
}

// Property: Hash uniqueness for different files
proptest! {
    #[test]
    fn prop_unique_files_unique_hashes(
        files in prop::collection::vec(
            (
                "[a-z]{1,10}",  // filename
                prop::collection::vec(any::<u8>(), 1..100)  // content
            ),
            2..20
        )
    ) {
        use blake3::Hasher;
        use std::collections::HashSet;
        
        // Create unique file contents
        let mut unique_contents = HashSet::new();
        for (_, content) in &files {
            unique_contents.insert(content.clone());
        }
        
        // Hash all unique contents
        let mut hashes = HashSet::new();
        for content in unique_contents.iter() {
            let mut hasher = Hasher::new();
            hasher.update(content);
            let hash = hasher.finalize().to_hex().to_string();
            hashes.insert(hash);
        }
        
        // Number of unique hashes should equal number of unique contents
        prop_assert_eq!(hashes.len(), unique_contents.len());
    }
}

// Property: Database operations are atomic
proptest! {
    #[test]
    fn prop_db_atomicity(
        operations in prop::collection::vec(
            (
                prop::bool::ANY,
                "[a-z]{1,20}",
                prop::collection::vec(any::<u8>(), 0..100)
            ),
            1..30
        )
    ) {
        use std::collections::HashSet;
        
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = wupdedup_rs::db::DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("test").unwrap();
        
        let mut expected_keys = HashSet::new();
        
        for (is_put, key, value) in operations {
            if is_put {
                bucket.put(&key, &value).unwrap();
                expected_keys.insert(key.clone());
            } else {
                bucket.delete(&key).unwrap();
                expected_keys.remove(&key);
            }
            
            // Verify consistency after each operation
            for expected_key in &expected_keys {
                prop_assert!(bucket.exists(expected_key).unwrap());
            }
        }
        
        prop_assert_eq!(bucket.count().unwrap(), expected_keys.len());
    }
}

// Property: Path normalization is consistent
proptest! {
    #[test]
    fn prop_path_normalization(
        components in prop::collection::vec(
            prop::string::string_regex("[a-zA-Z0-9._-]+").unwrap(),
            1..5
        )
    ) {
        use std::path::PathBuf;
        
        // Build path in different ways
        let path1 = components.iter()
            .fold(PathBuf::new(), |p, c| p.join(c));
        
        let path2 = PathBuf::from(components.join("/"));
        
        // On Unix systems, these should be equivalent
        #[cfg(unix)]
        prop_assert_eq!(path1.to_string_lossy(), path2.to_string_lossy());
    }
}

// Property: File operations preserve content
proptest! {
    #[test]
    fn prop_file_content_preservation(
        content in prop::collection::vec(any::<u8>(), 0..10000),
        filename in "[a-z]{1,20}\\.[a-z]{2,4}"
    ) {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join(&filename);
        
        // Write content
        std::fs::write(&file_path, &content).unwrap();
        
        // Read back
        let read_content = std::fs::read(&file_path).unwrap();
        
        // Content should be identical
        prop_assert_eq!(&content, &read_content);
        
        // Metadata should be consistent
        let metadata = std::fs::metadata(&file_path).unwrap();
        prop_assert_eq!(metadata.len() as usize, content.len());
        prop_assert!(metadata.is_file());
    }
}