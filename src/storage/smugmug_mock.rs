use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::config::SmugMugConfig;
use crate::db::FileMetadata;
use crate::storage::{StorageStrategy, StorageStrategyContext};

/// Mock SmugMug strategy for testing without real API credentials
pub struct MockSmugMugStrategy {
    config: SmugMugConfig,
}

impl MockSmugMugStrategy {
    pub fn new(config: SmugMugConfig) -> Result<Self> {
        info!("Creating MOCK SmugMug strategy (no real API calls will be made)");
        Ok(Self { config })
    }
    
    /// Generate mock albums with realistic data
    fn generate_mock_albums(&self) -> Vec<MockAlbum> {
        vec![
            MockAlbum {
                name: "Family Vacation 2024".to_string(),
                key: "album_001".to_string(),
                image_count: 5,
            },
            MockAlbum {
                name: "Birthday Party".to_string(),
                key: "album_002".to_string(),
                image_count: 3,
            },
            MockAlbum {
                name: "Nature Photography".to_string(),
                key: "album_003".to_string(),
                image_count: 8,
            },
        ]
    }
    
    /// Generate mock images for an album
    fn generate_mock_images(&self, album: &MockAlbum) -> Vec<MockImage> {
        (0..album.image_count)
            .map(|i| MockImage {
                filename: format!("IMG_{:04}.jpg", i + 1),
                key: format!("{}_{:03}", album.key, i),
                size: 1024 * (100 + i as u64 * 50), // Varying sizes
                md5: format!("{:032x}", i * 12345), // Fake but consistent hash
                width: 1920 + (i as u32 * 100),
                height: 1080 + (i as u32 * 50),
                last_updated: format!("2024-{:02}-{:02}T00:00:00Z", (i % 12) + 1, (i % 28) + 1),
            })
            .collect()
    }
}

#[async_trait]
impl StorageStrategy for MockSmugMugStrategy {
    async fn scan_tree(&self, context: &mut StorageStrategyContext) -> Result<()> {
        warn!("MOCK MODE: Starting simulated SmugMug scan (no real API calls)");
        
        // Simulate authentication delay
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        info!("MOCK: Authenticated as user: mock_user");
        
        let albums = self.generate_mock_albums();
        info!("MOCK: Found {} albums", albums.len());
        context.node_count = albums.len();
        
        let mut total_images = 0;
        
        for album in &albums {
            debug!("MOCK: Processing album: {} (key: {})", album.name, album.key);
            
            // Simulate API delay
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            
            let images = self.generate_mock_images(album);
            debug!("MOCK: Found {} images in album {}", images.len(), album.name);
            
            // Store mock image data in database
            if let Some(ref bucket) = context.bucket {
                for image in &images {
                    let file_info = FileInfo {
                        path: format!("smugmug://{}/{}", album.name, image.filename),
                        size: image.size,
                        hash: image.md5.clone(),
                        file_type: "image".to_string(),
                        modified: image.last_updated.clone(),
                        width: Some(image.width),
                        height: Some(image.height),
                    };
                    
                    let key = format!("smugmug:{}:{}", album.key, image.key);
                    let metadata = FileMetadata {
                        hash: Some(file_info.hash.clone()),
                        size: Some(file_info.size),
                        file_type: Some(file_info.file_type.clone()),
                    };
                    
                    let serialized = serde_json::to_vec(&file_info)?;
                    bucket.put_with_indexes(&key, &serialized, Some(metadata))?;
                }
            }
            
            total_images += images.len();
        }
        
        context.file_count = total_images;
        warn!("MOCK MODE: Scan complete - {} albums, {} images (no real data fetched)", 
              context.node_count, total_images);
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "smugmug_mock"
    }
}

#[derive(Debug)]
struct MockAlbum {
    name: String,
    key: String,
    image_count: usize,
}

#[derive(Debug)]
struct MockImage {
    filename: String,
    key: String,
    size: u64,
    md5: String,
    width: u32,
    height: u32,
    last_updated: String,
}

// Reuse FileInfo from main implementation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub hash: String,
    pub file_type: String,
    pub modified: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;
    use crate::db::DB;
    
    fn create_mock_config() -> SmugMugConfig {
        SmugMugConfig {
            api_key: "mock_key".to_string(),
            api_secret: "mock_secret".to_string(),
            access_token: "mock_token".to_string(),
            access_token_secret: "mock_token_secret".to_string(),
            user_nickname: Some("mock_user".to_string()),
        }
    }
    
    #[tokio::test]
    async fn test_mock_smugmug_scan() {
        let config = create_mock_config();
        let strategy = Arc::new(MockSmugMugStrategy::new(config).unwrap());
        let mut context = StorageStrategyContext::new(strategy.clone(), "smugmug_mock".to_string());
        
        // Create temporary database
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("mock_test").unwrap();
        context.set_bucket(bucket);
        
        // Run mock scan
        let result = context.scan_tree().await;
        assert!(result.is_ok());
        
        // Verify results
        assert_eq!(context.node_count, 3); // 3 mock albums
        assert_eq!(context.file_count, 16); // 5 + 3 + 8 images
        
        // Verify data was stored in database
        let bucket = db.bucket("mock_test").unwrap();
        let count = bucket.count().unwrap();
        assert_eq!(count, 16);
    }
    
    #[test]
    fn test_mock_album_generation() {
        let config = create_mock_config();
        let strategy = MockSmugMugStrategy::new(config).unwrap();
        
        let albums = strategy.generate_mock_albums();
        assert_eq!(albums.len(), 3);
        assert_eq!(albums[0].name, "Family Vacation 2024");
        assert_eq!(albums[0].image_count, 5);
    }
    
    #[test]
    fn test_mock_image_generation() {
        let config = create_mock_config();
        let strategy = MockSmugMugStrategy::new(config).unwrap();
        
        let album = MockAlbum {
            name: "Test Album".to_string(),
            key: "test_001".to_string(),
            image_count: 3,
        };
        
        let images = strategy.generate_mock_images(&album);
        assert_eq!(images.len(), 3);
        
        // Verify each image has unique properties
        for (i, image) in images.iter().enumerate() {
            assert_eq!(image.filename, format!("IMG_{:04}.jpg", i + 1));
            assert!(image.size > 0);
            assert!(!image.md5.is_empty());
        }
    }
}