use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub enum DedupeStrategy {
    Delete,            // Delete duplicate files (keep first)
    Move(PathBuf),     // Move duplicates to specified directory
    Symlink,           // Replace duplicates with symlinks to first
    Archive(PathBuf),  // Move duplicates to archive directory with metadata
}

pub struct DedupeEngine {
    strategy: DedupeStrategy,
    dry_run: bool,
    auto_confirm: bool,
}

impl DedupeEngine {
    pub fn new(strategy: DedupeStrategy, dry_run: bool, auto_confirm: bool) -> Self {
        Self {
            strategy,
            dry_run,
            auto_confirm,
        }
    }
    
    /// Process a group of duplicate files
    pub fn process_duplicates(&self, files: &[PathBuf]) -> Result<DedupeResult> {
        if files.len() < 2 {
            return Ok(DedupeResult::default());
        }
        
        // Keep the first file as the original
        let original = &files[0];
        let duplicates = &files[1..];
        
        info!("Processing {} duplicates of {}", duplicates.len(), original.display());
        
        let mut result = DedupeResult::default();
        
        for duplicate in duplicates {
            if !duplicate.exists() {
                warn!("File not found: {}", duplicate.display());
                result.skipped += 1;
                continue;
            }
            
            if !self.auto_confirm && !self.dry_run {
                // In real implementation, would prompt user here
                debug!("Would prompt for confirmation to dedupe: {}", duplicate.display());
            }
            
            match self.apply_strategy(original, duplicate)? {
                DedupeAction::Deleted => result.deleted += 1,
                DedupeAction::Moved => result.moved += 1,
                DedupeAction::Symlinked => result.symlinked += 1,
                DedupeAction::Archived => result.archived += 1,
                DedupeAction::Skipped => result.skipped += 1,
            }
            
            result.space_freed += fs::metadata(duplicate)
                .map(|m| m.len())
                .unwrap_or(0);
        }
        
        Ok(result)
    }
    
    fn apply_strategy(&self, original: &Path, duplicate: &Path) -> Result<DedupeAction> {
        if self.dry_run {
            info!("[DRY RUN] Would apply {:?} to {}", self.strategy, duplicate.display());
            return Ok(match self.strategy {
                DedupeStrategy::Delete => DedupeAction::Deleted,
                DedupeStrategy::Move(_) => DedupeAction::Moved,
                DedupeStrategy::Symlink => DedupeAction::Symlinked,
                DedupeStrategy::Archive(_) => DedupeAction::Archived,
            });
        }
        
        match &self.strategy {
            DedupeStrategy::Delete => {
                fs::remove_file(duplicate)
                    .with_context(|| format!("Failed to delete {}", duplicate.display()))?;
                info!("Deleted: {}", duplicate.display());
                Ok(DedupeAction::Deleted)
            }
            
            DedupeStrategy::Move(target_dir) => {
                let file_name = duplicate.file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;
                let target_path = target_dir.join(file_name);
                
                // Create target directory if it doesn't exist
                fs::create_dir_all(target_dir)?;
                
                // Add timestamp if file already exists at target
                let final_target = if target_path.exists() {
                    let timestamp = chrono::Utc::now().timestamp();
                    let stem = target_path.file_stem().unwrap_or_default();
                    let ext = target_path.extension();
                    let new_name = if let Some(ext) = ext {
                        format!("{}_{}.{}", 
                            stem.to_string_lossy(), 
                            timestamp,
                            ext.to_string_lossy())
                    } else {
                        format!("{}_{}", stem.to_string_lossy(), timestamp)
                    };
                    target_dir.join(new_name)
                } else {
                    target_path
                };
                
                fs::rename(duplicate, &final_target)
                    .with_context(|| format!("Failed to move {} to {}", 
                        duplicate.display(), final_target.display()))?;
                info!("Moved: {} -> {}", duplicate.display(), final_target.display());
                Ok(DedupeAction::Moved)
            }
            
            DedupeStrategy::Symlink => {
                #[cfg(unix)]
                {
                    fs::remove_file(duplicate)?;
                    std::os::unix::fs::symlink(original, duplicate)
                        .with_context(|| format!("Failed to create symlink from {} to {}", 
                            duplicate.display(), original.display()))?;
                    info!("Symlinked: {} -> {}", duplicate.display(), original.display());
                    Ok(DedupeAction::Symlinked)
                }
                #[cfg(not(unix))]
                {
                    warn!("Symlinks not supported on this platform");
                    Ok(DedupeAction::Skipped)
                }
            }
            
            DedupeStrategy::Archive(archive_dir) => {
                // Similar to Move but preserves directory structure
                let relative_path = duplicate.strip_prefix("/").unwrap_or(duplicate);
                let target_path = archive_dir.join(relative_path);
                
                // Create parent directories
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                fs::rename(duplicate, &target_path)
                    .with_context(|| format!("Failed to archive {} to {}", 
                        duplicate.display(), target_path.display()))?;
                info!("Archived: {} -> {}", duplicate.display(), target_path.display());
                Ok(DedupeAction::Archived)
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct DedupeResult {
    pub deleted: usize,
    pub moved: usize,
    pub symlinked: usize,
    pub archived: usize,
    pub skipped: usize,
    pub space_freed: u64,
}

impl DedupeResult {
    pub fn total_processed(&self) -> usize {
        self.deleted + self.moved + self.symlinked + self.archived
    }
    
    pub fn summary(&self) -> String {
        format!(
            "Processed: {} files (deleted: {}, moved: {}, symlinked: {}, archived: {}, skipped: {}), Space freed: {} MB",
            self.total_processed(),
            self.deleted,
            self.moved,
            self.symlinked,
            self.archived,
            self.skipped,
            self.space_freed / (1024 * 1024)
        )
    }
}

#[derive(Debug)]
enum DedupeAction {
    Deleted,
    Moved,
    Symlinked,
    Archived,
    Skipped,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_dedupe_strategy_creation() {
        let delete = DedupeStrategy::Delete;
        let move_strat = DedupeStrategy::Move(PathBuf::from("/tmp/duplicates"));
        let symlink = DedupeStrategy::Symlink;
        let archive = DedupeStrategy::Archive(PathBuf::from("/tmp/archive"));
        
        assert!(matches!(delete, DedupeStrategy::Delete));
        assert!(matches!(move_strat, DedupeStrategy::Move(_)));
        assert!(matches!(symlink, DedupeStrategy::Symlink));
        assert!(matches!(archive, DedupeStrategy::Archive(_)));
    }
    
    #[test]
    fn test_dedupe_engine_dry_run() {
        let engine = DedupeEngine::new(DedupeStrategy::Delete, true, false);
        
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        
        fs::write(&file1, "content").unwrap();
        fs::write(&file2, "content").unwrap();
        
        let files = vec![file1.clone(), file2.clone()];
        let result = engine.process_duplicates(&files).unwrap();
        
        // In dry run, files should not be deleted
        assert!(file1.exists());
        assert!(file2.exists());
        assert_eq!(result.deleted, 1); // But counted as would be deleted
    }
    
    #[test]
    fn test_dedupe_result_summary() {
        let mut result = DedupeResult::default();
        result.deleted = 5;
        result.moved = 3;
        result.symlinked = 2;
        result.archived = 1;
        result.skipped = 1;
        result.space_freed = 10 * 1024 * 1024; // 10 MB
        
        assert_eq!(result.total_processed(), 11);
        assert!(result.summary().contains("11 files"));
        assert!(result.summary().contains("10 MB"));
    }
}