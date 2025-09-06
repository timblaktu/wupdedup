use anyhow::Result;
use async_trait::async_trait;
use blake3::Hasher;
use rayon::prelude::*;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::{debug, error, info};
use walkdir::{DirEntry, WalkDir};

use crate::config::LocalConfig;
use crate::content;
use crate::db::FileMetadata;
use crate::storage::{StorageStrategy, StorageStrategyContext};

pub struct LocalStrategy {
    config: LocalConfig,
}

impl LocalStrategy {
    pub fn new(config: LocalConfig) -> Self {
        Self { config }
    }
    
    fn process_file(&self, entry: &DirEntry) -> Result<FileInfo> {
        let path = entry.path();
        let metadata = entry.metadata()?;
        
        // Get file type
        let file_type = content::get_type(path)?;
        
        // Calculate Blake3 hash
        let hash = self.calculate_hash(path)?;
        
        Ok(FileInfo {
            path: path.to_path_buf(),
            size: metadata.len(),
            file_type,
            hash,
            modified: metadata.modified()?.into(),
        })
    }
    
    fn calculate_hash(&self, path: &Path) -> Result<String> {
        let mut file = fs::File::open(path)?;
        let mut hasher = Hasher::new();
        
        // Read file in chunks for better performance
        let mut buffer = vec![0u8; 65536]; // 64KB chunks
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(hasher.finalize().to_hex().to_string())
    }
}

#[async_trait]
impl StorageStrategy for LocalStrategy {
    async fn scan_tree(&self, context: &mut StorageStrategyContext) -> Result<()> {
        info!("Scanning local tree at: {:?}", self.config.root_path);
        
        // Collect all file entries first
        let entries: Vec<DirEntry> = WalkDir::new(&self.config.root_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_type().is_dir())
            .collect();
        
        let total_files = entries.len();
        info!("Found {} files to process", total_files);
        
        // Process files in parallel using rayon
        let file_infos: Vec<FileInfo> = entries
            .par_iter()
            .filter_map(|entry| {
                match self.process_file(entry) {
                    Ok(info) => {
                        debug!("Processed: {:?}", info.path);
                        Some(info)
                    }
                    Err(e) => {
                        error!("Failed to process {:?}: {}", entry.path(), e);
                        None
                    }
                }
            })
            .collect();
        
        // Store results in database if bucket is available
        if let Some(bucket) = &context.bucket {
            for info in &file_infos {
                let key = info.path.to_string_lossy();
                let value = serde_json::to_vec(&info)?;
                
                // Create metadata for indexing
                let metadata = FileMetadata {
                    hash: Some(info.hash.clone()),
                    size: Some(info.size),
                    file_type: Some(info.file_type.clone()),
                };
                
                // Use indexed put operation
                bucket.put_with_indexes(&key, &value, Some(metadata))?;
            }
        }
        
        context.file_count = file_infos.len();
        context.node_count = total_files;
        
        info!(
            "Scan complete: {} files processed successfully",
            context.file_count
        );
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "local"
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub file_type: String,
    pub hash: String,
    pub modified: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_file_info_serialization() {
        let file_info = FileInfo {
            path: PathBuf::from("/test/path/file.txt"),
            size: 1024,
            file_type: "text/plain".to_string(),
            hash: "abc123def456".to_string(),
            modified: Utc::now(),
        };

        // Serialize
        let serialized = serde_json::to_vec(&file_info).unwrap();
        
        // Deserialize
        let deserialized: FileInfo = serde_json::from_slice(&serialized).unwrap();
        
        assert_eq!(deserialized.path, file_info.path);
        assert_eq!(deserialized.size, file_info.size);
        assert_eq!(deserialized.file_type, file_info.file_type);
        assert_eq!(deserialized.hash, file_info.hash);
    }

    #[test]
    fn test_file_info_deserialization_from_json() {
        let json_str = r#"{
            "path": "/home/user/document.pdf",
            "size": 2048,
            "file_type": "application/pdf",
            "hash": "xyz789abc456",
            "modified": "2024-01-01T12:00:00Z"
        }"#;

        let file_info: FileInfo = serde_json::from_str(json_str).unwrap();
        
        assert_eq!(file_info.path, PathBuf::from("/home/user/document.pdf"));
        assert_eq!(file_info.size, 2048);
        assert_eq!(file_info.file_type, "application/pdf");
        assert_eq!(file_info.hash, "xyz789abc456");
    }
}