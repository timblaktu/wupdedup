use anyhow::Result;
use async_trait::async_trait;
use futures::{StreamExt, future::BoxFuture, FutureExt};
use serde::{Deserialize, Serialize};
use smugmug::v2::{Album, Client, Creds, Node, User};
use smugmug::v2::{NodeTypeFilters, SortDirection, SortMethod};
use tracing::{debug, info, warn, error};

use crate::config::SmugMugConfig;
use crate::db::FileMetadata;
use crate::storage::{StorageStrategy, StorageStrategyContext};

pub struct SmugMugStrategy {
    config: SmugMugConfig,
    client: Client,
}

impl SmugMugStrategy {
    pub fn new(config: SmugMugConfig) -> Result<Self> {
        // Create OAuth 1.0a credentials
        let creds = Creds::from_tokens(
            &config.api_key,
            Some(&config.api_secret),
            Some(&config.access_token),
            Some(&config.access_token_secret),
        );
        
        let client = Client::new(creds);
        
        Ok(Self { config, client })
    }
    
    async fn process_album(&self, album: Album, context: &mut StorageStrategyContext) -> Result<usize> {
        let album_key = album.album_key.clone();
        let album_name = if album.name.is_empty() {
            "Unnamed".to_string()
        } else {
            album.name.clone()
        };
        
        debug!("Processing album: {} (key: {})", album_name, album_key);
        
        // Get images in the album - returns a stream
        let images_stream = match album.images() {
            Ok(stream) => stream,
            Err(e) => {
                warn!("Failed to fetch images for album {}: {}", album_name, e);
                return Ok(0);
            }
        };
        
        let mut image_count = 0;
        
        // Collect images from the stream
        futures::pin_mut!(images_stream);
        while let Some(image_result) = images_stream.next().await {
            match image_result {
                Ok(image) => {
                    if let Err(e) = self.process_image(image, &album_name, context).await {
                        warn!("Failed to process image: {}", e);
                        continue;
                    }
                    image_count += 1;
                }
                Err(e) => {
                    warn!("Error fetching image: {}", e);
                }
            }
        }
        
        Ok(image_count)
    }
    
    async fn process_image(&self, image: smugmug::v2::Image, album_name: &str, context: &StorageStrategyContext) -> Result<()> {
        // Extract image metadata from the Image struct
        let image_key = image.image_key.clone();
        let filename = if image.file_name.is_empty() {
            "unknown.jpg".to_string()
        } else {
            image.file_name.clone()
        };
        
        // Get file size and hash if available
        let archived_size = image.archived_size.unwrap_or(0);
        let archived_md5 = image.archived_md5.clone().unwrap_or_default();
        
        // Get dimensions if available - these fields might not exist in the API
        // Using 0 as default for now
        let width = 0u32;
        let height = 0u32;
        
        // Get last updated time - convert to string
        let last_updated = format!("{:?}", image.last_updated);
        
        // Create FileInfo for database storage
        let file_info = FileInfo {
            path: format!("smugmug://{}/{}", album_name, filename),
            size: archived_size as u64,
            hash: if archived_md5.is_empty() { 
                format!("smugmug:{}", image_key)  // Use image key as fallback
            } else {
                archived_md5
            },
            file_type: "image".to_string(),
            modified: last_updated,
            width: Some(width as u32),
            height: Some(height as u32),
        };
        
        // Store in database if bucket is available
        if let Some(ref bucket) = context.bucket {
            let key = format!("smugmug:{}:{}", album_name, image_key);
            
            // Create metadata for indexing
            let metadata = FileMetadata {
                hash: Some(file_info.hash.clone()),
                size: Some(file_info.size),
                file_type: Some(file_info.file_type.clone()),
            };
            
            let serialized = serde_json::to_vec(&file_info)?;
            bucket.put_with_indexes(&key, &serialized, Some(metadata))?;
        }
        
        Ok(())
    }
}

#[async_trait]
impl StorageStrategy for SmugMugStrategy {
    async fn scan_tree(&self, context: &mut StorageStrategyContext) -> Result<()> {
        info!("Starting SmugMug scan");
        
        // Get authenticated user info
        let user = match User::authenticated_user_info(self.client.clone()).await {
            Ok(u) => u,
            Err(e) => {
                error!("Failed to get authenticated user info: {}", e);
                anyhow::bail!("SmugMug authentication failed: {}", e);
            }
        };
        
        let nickname = user.nick_name.clone().unwrap_or_else(|| "Unknown".to_string());
        info!("Authenticated as SmugMug user: {}", nickname);
        
        // Get the user's root node
        let root_node = match user.node().await {
            Ok(node) => node,
            Err(e) => {
                error!("Failed to get user root node: {}", e);
                anyhow::bail!("Failed to access SmugMug user data: {}", e);
            }
        };
        
        // Get all albums (recursively through folder structure)
        let albums = self.collect_all_albums(&root_node).await?;
        
        info!("Found {} SmugMug albums", albums.len());
        context.node_count = albums.len();
        
        // Process each album
        let mut total_images = 0;
        for album in albums {
            match self.process_album(album, context).await {
                Ok(count) => total_images += count,
                Err(e) => warn!("Failed to process album: {}", e),
            }
        }
        
        context.file_count = total_images;
        info!("SmugMug scan complete: {} albums, {} images", context.node_count, total_images);
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "smugmug"
    }
}

impl SmugMugStrategy {
    fn collect_all_albums<'a>(&'a self, node: &'a Node) -> BoxFuture<'a, Result<Vec<Album>>> {
        async move {
        let mut albums = Vec::new();
        
        // Get direct album children of this node - returns a stream
        let children_stream = node.children(
            NodeTypeFilters::Album,
            SortDirection::Ascending,
            SortMethod::Name,
        )?;
        
        // Collect album nodes from stream
        futures::pin_mut!(children_stream);
        while let Some(child_result) = children_stream.next().await {
            match child_result {
                Ok(child_node) => {
                    // Convert Node to Album
                    if let Ok(album) = child_node.album().await {
                        albums.push(album);
                    }
                }
                Err(e) => {
                    warn!("Error fetching child node: {}", e);
                }
            }
        }
        
        // Recursively get albums from folder children
        let folders_stream = node.children(
            NodeTypeFilters::Folder,
            SortDirection::Ascending,
            SortMethod::Name,
        )?;
        
        futures::pin_mut!(folders_stream);
        while let Some(folder_result) = folders_stream.next().await {
            match folder_result {
                Ok(folder_node) => {
                    // Recursively collect albums from subfolders
                    if let Ok(sub_albums) = self.collect_all_albums(&folder_node).await {
                        albums.extend(sub_albums);
                    }
                }
                Err(e) => {
                    warn!("Error fetching folder node: {}", e);
                }
            }
        }
        
        Ok(albums)
        }.boxed()
    }
}

// FileInfo struct for storing in database
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

    fn create_test_config() -> SmugMugConfig {
        SmugMugConfig {
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
            access_token: "test_token".to_string(),
            access_token_secret: "test_token_secret".to_string(),
            user_nickname: Some("test_user".to_string()),
        }
    }

    #[test]
    fn test_smugmug_strategy_creation() {
        let config = create_test_config();
        let strategy = SmugMugStrategy::new(config.clone());
        
        assert!(strategy.is_ok());
        let strategy = strategy.unwrap();
        assert_eq!(strategy.name(), "smugmug");
    }

    #[test]
    fn test_smugmug_config_validation() {
        let mut config = create_test_config();
        assert!(config.valid().is_ok());
        
        // Test missing API key
        config.api_key = "".to_string();
        assert!(config.valid().is_err());
        
        // Test missing access token
        config = create_test_config();
        config.access_token = "".to_string();
        assert!(config.valid().is_err());
    }

    #[tokio::test]
    async fn test_smugmug_context_creation() {
        let config = create_test_config();
        let strategy = SmugMugStrategy::new(config).unwrap();
        let strategy = Arc::new(strategy);
        let context = StorageStrategyContext::new(strategy.clone(), "smugmug".to_string());
        
        assert_eq!(context.name, "smugmug");
        assert_eq!(context.file_count, 0);
        assert_eq!(context.node_count, 0);
        assert!(context.bucket.is_none());
    }
    
    #[test]
    fn test_file_info_serialization() {
        let file_info = FileInfo {
            path: "smugmug://TestAlbum/image.jpg".to_string(),
            size: 1024,
            hash: "abc123".to_string(),
            file_type: "image".to_string(),
            modified: "2024-01-01T00:00:00Z".to_string(),
            width: Some(1920),
            height: Some(1080),
        };
        
        let serialized = serde_json::to_string(&file_info).unwrap();
        let deserialized: FileInfo = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized.path, file_info.path);
        assert_eq!(deserialized.size, file_info.size);
        assert_eq!(deserialized.hash, file_info.hash);
        assert_eq!(deserialized.width, file_info.width);
        assert_eq!(deserialized.height, file_info.height);
    }
}