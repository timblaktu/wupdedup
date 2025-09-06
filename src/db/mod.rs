use anyhow::{Context, Result};
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

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
        
        info!("Database opened successfully at {:?}", path);
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
    pub fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&self.name);
        let write_txn = self.database.begin_write()?;
        {
            let mut table = write_txn.open_table(table_def)?;
            table.insert(key, value)?;
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
        bucket.scan(|key, _value| {
            keys.push(key.to_string());
            Ok(())
        }).unwrap();
        
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
                    bucket.put(&format!("key_{}", j), format!("value_{}_{}", i, j).as_bytes()).unwrap();
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
}