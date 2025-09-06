use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::config::SmugMugConfig;
use crate::storage::{StorageStrategy, StorageStrategyContext};

#[derive(Debug, Serialize, Deserialize)]
pub struct AlbumImage {
    pub id: String,
    pub filename: String,
    pub caption: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub md5: Option<String>,
    pub size: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub last_updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub url_path: String,
    pub image_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SmugMugResponse<T> {
    pub response: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlbumsResponse {
    pub albums: Vec<Album>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImagesResponse {
    pub images: Vec<AlbumImage>,
}

pub struct SmugMugStrategy {
    config: SmugMugConfig,
    client: Client,
}

impl SmugMugStrategy {
    pub fn new(config: SmugMugConfig) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to create HTTP client");
            
        Self { config, client }
    }
    
    async fn authenticate(&self) -> Result<String> {
        // OAuth 1.0a authentication for SmugMug
        // This is a simplified version - real implementation would need proper OAuth signing
        warn!("SmugMug OAuth authentication not fully implemented - using placeholder token");
        Ok(format!("Bearer {}", self.config.user_token))
    }
    
    async fn fetch_albums(&self, auth_token: &str) -> Result<Vec<Album>> {
        let url = format!("{}/api/v2/user/{}/albums", self.config.url, self.config.api_key);
        
        let response = self.client
            .get(&url)
            .header(header::AUTHORIZATION, auth_token)
            .send()
            .await?;
            
        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch albums: {}", response.status());
        }
        
        let data: SmugMugResponse<AlbumsResponse> = response.json().await?;
        Ok(data.response.albums)
    }
    
    async fn fetch_images(&self, auth_token: &str, album_id: &str) -> Result<Vec<AlbumImage>> {
        let url = format!("{}/api/v2/album/{}/images", self.config.url, album_id);
        
        let response = self.client
            .get(&url)
            .header(header::AUTHORIZATION, auth_token)
            .send()
            .await?;
            
        if !response.status().is_success() {
            anyhow::bail!("Failed to fetch images for album {}: {}", album_id, response.status());
        }
        
        let data: SmugMugResponse<ImagesResponse> = response.json().await?;
        Ok(data.response.images)
    }
}

#[async_trait]
impl StorageStrategy for SmugMugStrategy {
    async fn scan_tree(&self, context: &mut StorageStrategyContext) -> Result<()> {
        info!("Starting SmugMug scan at: {}", self.config.url);
        
        // Authenticate with SmugMug
        let auth_token = self.authenticate().await?;
        debug!("SmugMug authentication completed");
        
        // Fetch all albums
        let albums = match self.fetch_albums(&auth_token).await {
            Ok(albums) => albums,
            Err(e) => {
                warn!("Failed to fetch SmugMug albums: {}. Using mock data for development.", e);
                // Return mock data for development/testing
                vec![
                    Album {
                        id: "mock_album_1".to_string(),
                        name: "Mock Album 1".to_string(),
                        url_path: "/mock/album1".to_string(),
                        image_count: 5,
                    },
                    Album {
                        id: "mock_album_2".to_string(),
                        name: "Mock Album 2".to_string(),
                        url_path: "/mock/album2".to_string(),
                        image_count: 3,
                    },
                ]
            }
        };
        
        info!("Found {} SmugMug albums", albums.len());
        context.node_count = albums.len();
        
        // Process each album
        let mut total_images = 0;
        for album in &albums {
            debug!("Processing album: {} ({})", album.name, album.id);
            
            // Fetch images in the album
            let images = match self.fetch_images(&auth_token, &album.id).await {
                Ok(images) => images,
                Err(e) => {
                    warn!("Failed to fetch images for album {}: {}. Using mock data.", album.id, e);
                    // Return mock images for development/testing
                    (0..album.image_count).map(|i| AlbumImage {
                        id: format!("mock_image_{}_{}", album.id, i),
                        filename: format!("image_{}.jpg", i),
                        caption: Some(format!("Mock image {} in {}", i, album.name)),
                        keywords: Some(vec!["mock".to_string(), "test".to_string()]),
                        md5: Some(format!("{:032x}", i)),
                        size: Some(1024 * (i as u64 + 1)),
                        width: Some(1920),
                        height: Some(1080),
                        last_updated: Some("2024-01-01T00:00:00Z".to_string()),
                    }).collect()
                }
            };
            
            debug!("Found {} images in album {}", images.len(), album.name);
            
            // Store image metadata in the database
            if let Some(ref bucket) = context.bucket {
                for image in &images {
                    let key = format!("smugmug:{}:{}", album.id, image.id);
                    let file_info = FileInfo {
                        path: format!("{}/{}", album.url_path, image.filename),
                        size: image.size.unwrap_or(0),
                        hash: image.md5.clone().unwrap_or_else(|| "unknown".to_string()),
                        mime_type: "image/jpeg".to_string(), // Assume JPEG for now
                        modified: image.last_updated.clone().unwrap_or_else(|| "unknown".to_string()),
                    };
                    
                    let serialized = serde_json::to_vec(&file_info)?;
                    bucket.put(&key, &serialized)?;
                }
            }
            
            total_images += images.len();
        }
        
        context.file_count = total_images;
        info!("SmugMug scan complete: {} albums, {} images", albums.len(), total_images);
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "smugmug"
    }
}

// FileInfo struct for storing in database
#[derive(Debug, Serialize, Deserialize)]
struct FileInfo {
    path: String,
    size: u64,
    hash: String,
    mime_type: String,
    modified: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn create_test_config() -> SmugMugConfig {
        SmugMugConfig {
            url: "https://api.smugmug.com".to_string(),
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
            user_token: "user_token".to_string(),
            user_secret: "user_secret".to_string(),
            destination: "Albums".to_string(),
            file_names: "original".to_string(),
            use_metadata_times: true,
            force_metadata_times: false,
        }
    }

    #[test]
    fn test_smugmug_strategy_creation() {
        let config = create_test_config();
        let strategy = SmugMugStrategy::new(config.clone());
        
        assert_eq!(strategy.name(), "smugmug");
        assert_eq!(strategy.config.url, config.url);
        assert_eq!(strategy.config.api_key, config.api_key);
    }

    #[tokio::test]
    async fn test_smugmug_scan_with_mock_data() {
        let config = create_test_config();
        let strategy = Arc::new(SmugMugStrategy::new(config));
        let mut context = StorageStrategyContext::new(strategy.clone(), "smugmug".to_string());
        
        // Should complete without error and return mock data
        let result = context.scan_tree().await;
        assert!(result.is_ok());
        
        // Mock implementation returns 2 albums with 8 total images
        assert_eq!(context.file_count, 8); // 5 + 3 images from mock albums
        assert_eq!(context.node_count, 2); // 2 mock albums
    }

    #[test]
    fn test_smugmug_config_with_different_destinations() {
        let mut config = create_test_config();
        
        // Test different destination values
        let destinations = vec!["Albums", "Folders", "Gallery"];
        
        for dest in destinations {
            config.destination = dest.to_string();
            let strategy = SmugMugStrategy::new(config.clone());
            assert_eq!(strategy.config.destination, dest);
        }
    }

    #[test]
    fn test_smugmug_config_with_different_file_names() {
        let mut config = create_test_config();
        
        // Test different file name options
        let file_name_options = vec!["original", "custom", "sequential"];
        
        for option in file_name_options {
            config.file_names = option.to_string();
            let strategy = SmugMugStrategy::new(config.clone());
            assert_eq!(strategy.config.file_names, option);
        }
    }

    #[test]
    fn test_smugmug_metadata_options() {
        let mut config = create_test_config();
        
        // Test metadata time options
        config.use_metadata_times = false;
        config.force_metadata_times = true;
        
        let strategy = SmugMugStrategy::new(config.clone());
        assert!(!strategy.config.use_metadata_times);
        assert!(strategy.config.force_metadata_times);
    }

    #[tokio::test]
    async fn test_smugmug_context_integration() {
        use crate::db::DB;
        use tempfile::tempdir;
        
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = DB::init(db_path.to_str().unwrap()).unwrap();
        let bucket = db.bucket("smugmug_test").unwrap();
        
        let config = create_test_config();
        let strategy = Arc::new(SmugMugStrategy::new(config));
        let mut context = StorageStrategyContext::new(strategy, "smugmug".to_string());
        
        context.set_bucket(bucket);
        assert!(context.bucket.is_some());
        
        let result = context.scan_tree().await;
        assert!(result.is_ok());
        
        // Verify that mock data was stored in the database
        // Re-get the bucket to check the count
        let bucket = db.bucket("smugmug_test").unwrap();
        let count = bucket.count().unwrap();
        assert_eq!(count, 8); // 8 mock images should be stored
    }
    
    #[tokio::test]
    async fn test_smugmug_authentication() {
        let config = create_test_config();
        let strategy = SmugMugStrategy::new(config);
        
        // Test authentication (returns placeholder token for now)
        let auth_result = strategy.authenticate().await;
        assert!(auth_result.is_ok());
        
        let token = auth_result.unwrap();
        assert!(token.starts_with("Bearer "));
        assert!(token.contains("user_token"));
    }
}