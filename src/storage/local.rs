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
                bucket.put(&key, &value)?;
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
struct FileInfo {
    path: PathBuf,
    size: u64,
    file_type: String,
    hash: String,
    modified: chrono::DateTime<chrono::Utc>,
}