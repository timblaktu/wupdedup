use anyhow::{Context, Result};
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

// Define index tables for common query patterns using regular tables with composite keys
const HASH_INDEX: TableDefinition<&str, &str> = TableDefinition::new("idx_hash");
const SIZE_INDEX: TableDefinition<&str, &str> = TableDefinition::new("idx_size");
const TYPE_INDEX: TableDefinition<&str, &str> = TableDefinition::new("idx_type");

// Metadata for indexing
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub hash: Option<String>,
    pub size: Option<u64>,
    pub file_type: Option<String>,
}

pub struct DB {
    database: Arc<Database>,
}

impl DB {
    pub fn init(dbfile: &str) -> Result<Self> {
        Self::open(dbfile)
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("Opening redb database at {:?}", path);

        let database = Database::create(path)
            .with_context(|| format!("Failed to open database at {:?}", path))?;

        // Initialize index tables
        let write_txn = database.begin_write()?;
        {
            // Create index tables if they don't exist
            let _hash_idx = write_txn.open_table(HASH_INDEX)?;
            let _size_idx = write_txn.open_table(SIZE_INDEX)?;
            let _type_idx = write_txn.open_table(TYPE_INDEX)?;
        }
        write_txn.commit()?;

        info!("Database opened successfully with indexes at {:?}", path);
        Ok(Self {
            database: Arc::new(database),
        })
    }

    pub fn bucket(&self, name: &str) -> Result<Bucket> {
        debug!("Creating bucket if it doesn't exist: {}", name);

        // Create a table definition for this bucket
        // Using &str for both key and value types for simplicity
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(name);

        // Ensure the table exists
        let write_txn = self.database.begin_write()?;
        {
            let _table = write_txn.open_table(table_def)?;
        }
        write_txn.commit()?;

        debug!("Bucket '{}' ready", name);
        Ok(Bucket {
            database: self.database.clone(),
            name: name.to_string(),
        })
    }

    pub fn close(self) -> Result<()> {
        // redb automatically handles closing when dropped
        debug!("Database closed");
        Ok(())
    }
}

pub struct Bucket {
    database: Arc<Database>,
    name: String,
}

impl Bucket {
    #[allow(dead_code)] // Public API - will be used by future storage backends
    pub fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        self.put_with_indexes(key, value, None)
    }

    // Enhanced put that updates indexes
    pub fn put_with_indexes(
        &self,
        key: &str,
        value: &[u8],
        metadata: Option<FileMetadata>,
    ) -> Result<()> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let write_txn = self.database.begin_write()?;
        {
            let mut table = write_txn.open_table(table_def)?;
            table.insert(key, value)?;

            // Update indexes if metadata is provided
            if let Some(meta) = metadata {
                // Update hash index with composite key: "hash:key"
                if let Some(hash) = meta.hash {
                    let mut hash_idx = write_txn.open_table(HASH_INDEX)?;
                    let composite_key = format!("{}:{}", hash, key);
                    hash_idx.insert(composite_key.as_str(), key)?;
                }

                // Update size index with composite key: "size:key"
                if let Some(size) = meta.size {
                    let mut size_idx = write_txn.open_table(SIZE_INDEX)?;
                    let composite_key = format!("{}:{}", size, key);
                    size_idx.insert(composite_key.as_str(), key)?;
                }

                // Update type index with composite key: "type:key"
                if let Some(file_type) = meta.file_type {
                    let mut type_idx = write_txn.open_table(TYPE_INDEX)?;
                    let composite_key = format!("{}:{}", file_type, key);
                    type_idx.insert(composite_key.as_str(), key)?;
                }
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let read_txn = self.database.begin_read()?;
        let table = read_txn.open_table(table_def)?;

        match table.get(key)? {
            Some(value) => Ok(Some(value.value().to_vec())),
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub fn delete(&self, key: &str) -> Result<()> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let write_txn = self.database.begin_write()?;
        {
            let mut table = write_txn.open_table(table_def)?;
            table.remove(key)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn exists(&self, key: &str) -> Result<bool> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let read_txn = self.database.begin_read()?;
        let table = read_txn.open_table(table_def)?;
        Ok(table.get(key)?.is_some())
    }

    #[allow(dead_code)]
    pub fn scan_prefix(&self, prefix: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let read_txn = self.database.begin_read()?;
        let table = read_txn.open_table(table_def)?;

        let mut results = Vec::new();
        for item in table.iter()? {
            let (key, value) = item?;
            let key_str = key.value();
            if key_str.starts_with(prefix) {
                results.push((key_str.to_string(), value.value().to_vec()));
            }
        }

        Ok(results)
    }

    pub fn count(&self) -> Result<usize> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let read_txn = self.database.begin_read()?;
        let table = read_txn.open_table(table_def)?;
        Ok(table.len()? as usize)
    }

    // Query by hash using index
    #[allow(dead_code)] // Advanced querying API - planned for future use
    pub fn find_by_hash(&self, hash: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let read_txn = self.database.begin_read()?;
        let table = read_txn.open_table(table_def)?;
        let hash_idx = read_txn.open_table(HASH_INDEX)?;

        let mut results = Vec::new();
        let prefix = format!("{}:", hash);

        // Scan index for all entries with this hash prefix
        for item in hash_idx.range(prefix.as_str()..)? {
            let (composite_key, key) = item?;
            if !composite_key.value().starts_with(&prefix) {
                break; // No more entries with this prefix
            }

            if let Some(value) = table.get(key.value())? {
                results.push((key.value().to_string(), value.value().to_vec()));
            }
        }

        Ok(results)
    }

    // Query by size using index
    #[allow(dead_code)] // Advanced querying API - planned for future use
    pub fn find_by_size(&self, size: u64) -> Result<Vec<(String, Vec<u8>)>> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let read_txn = self.database.begin_read()?;
        let table = read_txn.open_table(table_def)?;
        let size_idx = read_txn.open_table(SIZE_INDEX)?;

        let mut results = Vec::new();
        let prefix = format!("{}:", size);

        // Scan index for all entries with this size prefix
        for item in size_idx.range(prefix.as_str()..)? {
            let (composite_key, key) = item?;
            if !composite_key.value().starts_with(&prefix) {
                break; // No more entries with this prefix
            }

            if let Some(value) = table.get(key.value())? {
                results.push((key.value().to_string(), value.value().to_vec()));
            }
        }

        Ok(results)
    }

    // Query by file type using index
    #[allow(dead_code)] // Advanced querying API - planned for future use
    pub fn find_by_type(&self, file_type: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let read_txn = self.database.begin_read()?;
        let table = read_txn.open_table(table_def)?;
        let type_idx = read_txn.open_table(TYPE_INDEX)?;

        let mut results = Vec::new();
        let prefix = format!("{}:", file_type);

        // Scan index for all entries with this type prefix
        for item in type_idx.range(prefix.as_str()..)? {
            let (composite_key, key) = item?;
            if !composite_key.value().starts_with(&prefix) {
                break; // No more entries with this prefix
            }

            if let Some(value) = table.get(key.value())? {
                results.push((key.value().to_string(), value.value().to_vec()));
            }
        }

        Ok(results)
    }

    // Find duplicate files by hash
    pub fn find_duplicates(&self) -> Result<HashMap<String, Vec<String>>> {
        let read_txn = self.database.begin_read()?;
        let hash_idx = read_txn.open_table(HASH_INDEX)?;

        let mut hash_files: HashMap<String, Vec<String>> = HashMap::new();

        // Iterate through all hash entries and group by hash
        for item in hash_idx.iter()? {
            let (composite_key, file_key) = item?;
            let composite = composite_key.value();

            // Extract hash from composite key "hash:key"
            if let Some(colon_pos) = composite.find(':') {
                let hash = &composite[..colon_pos];
                hash_files
                    .entry(hash.to_string())
                    .or_default()
                    .push(file_key.value().to_string());
            }
        }

        // Filter to only include duplicates (more than 1 file)
        let duplicates: HashMap<String, Vec<String>> = hash_files
            .into_iter()
            .filter(|(_, files)| files.len() > 1)
            .collect();

        Ok(duplicates)
    }

    #[allow(dead_code)]
    pub fn all(&self) -> Result<Vec<(String, Vec<u8>)>> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let read_txn = self.database.begin_read()?;
        let table = read_txn.open_table(table_def)?;

        let mut results = Vec::new();
        for item in table.iter()? {
            let (key, value) = item?;
            results.push((key.value().to_string(), value.value().to_vec()));
        }

        Ok(results)
    }
}

// Scanner trait for iterating over database entries
#[allow(dead_code)]
pub trait Scanner {
    fn scan<F>(&self, f: F) -> Result<()>
    where
        F: FnMut(&str, &[u8]) -> Result<()>;
}

impl Scanner for Bucket {
    fn scan<F>(&self, mut f: F) -> Result<()>
    where
        F: FnMut(&str, &[u8]) -> Result<()>,
    {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let read_txn = self.database.begin_read()?;
        let table = read_txn.open_table(table_def)?;

        for item in table.iter()? {
            let (key, value) = item?;
            f(key.value(), value.value())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_db_init() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let _db = DB::init(db_path.to_str().unwrap()).unwrap();
    }

    #[test]
    fn test_bucket_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DB::init(db_path.to_str().unwrap()).unwrap();

        let bucket = db.bucket("test_bucket").unwrap();
        assert_eq!(bucket.name, "test_bucket");
    }

    #[test]
    fn test_put_get_delete() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("test_bucket").unwrap();

        // Test put
        bucket.put("key1", b"value1").unwrap();

        // Test get
        let value = bucket.get("key1").unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        // Test get non-existent key
        let missing = bucket.get("nonexistent").unwrap();
        assert_eq!(missing, None);

        // Test delete
        bucket.delete("key1").unwrap();
        let deleted = bucket.get("key1").unwrap();
        assert_eq!(deleted, None);
    }

    #[test]
    fn test_exists() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("test_bucket").unwrap();

        assert!(!bucket.exists("key1").unwrap());
        bucket.put("key1", b"value1").unwrap();
        assert!(bucket.exists("key1").unwrap());
    }

    #[test]
    fn test_scan_prefix() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("test_bucket").unwrap();

        // Add keys with different prefixes
        bucket.put("user:1", b"Alice").unwrap();
        bucket.put("user:2", b"Bob").unwrap();
        bucket.put("post:1", b"Hello").unwrap();

        // Scan for user prefix
        let users = bucket.scan_prefix("user:").unwrap();
        assert_eq!(users.len(), 2);

        // Scan for post prefix
        let posts = bucket.scan_prefix("post:").unwrap();
        assert_eq!(posts.len(), 1);
    }

    #[test]
    fn test_count() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("test_bucket").unwrap();

        assert_eq!(bucket.count().unwrap(), 0);

        bucket.put("key1", b"value1").unwrap();
        bucket.put("key2", b"value2").unwrap();
        assert_eq!(bucket.count().unwrap(), 2);

        bucket.delete("key1").unwrap();
        assert_eq!(bucket.count().unwrap(), 1);
    }

    #[test]
    fn test_scanner_trait() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("test_bucket").unwrap();

        // Add some data
        bucket.put("a", b"1").unwrap();
        bucket.put("b", b"2").unwrap();
        bucket.put("c", b"3").unwrap();

        // Use scanner to collect all keys
        let mut keys = Vec::new();
        bucket
            .scan(|key, _value| {
                keys.push(key.to_string());
                Ok(())
            })
            .unwrap();

        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"a".to_string()));
        assert!(keys.contains(&"b".to_string()));
        assert!(keys.contains(&"c".to_string()));
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Arc::new(DB::init(db_path.to_str().unwrap()).unwrap());

        let mut handles = vec![];

        // Spawn multiple threads writing to different buckets
        for i in 0..5 {
            let db_clone = Arc::clone(&db);
            let handle = thread::spawn(move || {
                let bucket = db_clone.bucket(&format!("bucket_{}", i)).unwrap();
                for j in 0..10 {
                    bucket
                        .put(
                            &format!("key_{}", j),
                            format!("value_{}_{}", i, j).as_bytes(),
                        )
                        .unwrap();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify data
        for i in 0..5 {
            let bucket = db.bucket(&format!("bucket_{}", i)).unwrap();
            for j in 0..10 {
                let value = bucket.get(&format!("key_{}", j)).unwrap();
                assert_eq!(value, Some(format!("value_{}_{}", i, j).into_bytes()));
            }
        }
    }

    #[test]
    fn test_indexed_operations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("test_bucket").unwrap();

        // Add files with metadata
        let metadata1 = FileMetadata {
            hash: Some("hash123".to_string()),
            size: Some(1024),
            file_type: Some("image/jpeg".to_string()),
        };
        bucket
            .put_with_indexes("file1.jpg", b"content1", Some(metadata1))
            .unwrap();

        let metadata2 = FileMetadata {
            hash: Some("hash456".to_string()),
            size: Some(2048),
            file_type: Some("image/png".to_string()),
        };
        bucket
            .put_with_indexes("file2.png", b"content2", Some(metadata2))
            .unwrap();

        let metadata3 = FileMetadata {
            hash: Some("hash123".to_string()), // Duplicate hash
            size: Some(1024),
            file_type: Some("image/jpeg".to_string()),
        };
        bucket
            .put_with_indexes("file3.jpg", b"content3", Some(metadata3))
            .unwrap();

        // Test find by hash
        let by_hash = bucket.find_by_hash("hash123").unwrap();
        assert_eq!(by_hash.len(), 2); // Should find 2 files with same hash

        // Test find by size
        let by_size = bucket.find_by_size(1024).unwrap();
        assert_eq!(by_size.len(), 2); // Should find 2 files with size 1024

        // Test find by type
        let by_type = bucket.find_by_type("image/jpeg").unwrap();
        assert_eq!(by_type.len(), 2); // Should find 2 JPEG files

        // Test find duplicates
        let duplicates = bucket.find_duplicates().unwrap();
        assert_eq!(duplicates.len(), 1); // Should find 1 duplicate hash
        assert_eq!(duplicates.get("hash123").unwrap().len(), 2); // With 2 files
    }

    #[test]
    fn test_index_performance() {
        use std::time::Instant;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("perf_bucket").unwrap();

        // Add many files
        for i in 0..1000 {
            let metadata = FileMetadata {
                hash: Some(format!("hash_{}", i % 100)), // 100 unique hashes
                size: Some((i % 10) as u64 * 1024),
                file_type: Some(
                    if i % 2 == 0 {
                        "image/jpeg"
                    } else {
                        "image/png"
                    }
                    .to_string(),
                ),
            };
            bucket
                .put_with_indexes(
                    &format!("file_{}.jpg", i),
                    format!("content_{}", i).as_bytes(),
                    Some(metadata),
                )
                .unwrap();
        }

        // Measure indexed query performance
        let start = Instant::now();
        let results = bucket.find_by_hash("hash_50").unwrap();
        let duration = start.elapsed();

        assert_eq!(results.len(), 10); // Should find 10 files with hash_50
        assert!(duration.as_millis() < 10); // Should be fast (< 10ms)

        // Measure duplicate finding performance
        let start = Instant::now();
        let duplicates = bucket.find_duplicates().unwrap();
        let duration = start.elapsed();

        assert_eq!(duplicates.len(), 100); // Should find 100 duplicate groups
        assert!(duration.as_millis() < 100); // Should be reasonably fast (< 100ms)
    }
}
