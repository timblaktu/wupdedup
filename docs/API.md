# wupdedup-rs API and Interface Documentation

## Command Line Interface

### Global Options
```bash
wupdedup-rs [OPTIONS] <COMMAND>

Options:
  -c, --config <FILE>     Configuration file path [default: config.toml]
  -v, --verbose          Increase logging verbosity
  -q, --quiet            Decrease logging verbosity
  --json                 Output in JSON format
  --no-color             Disable colored output
  -h, --help             Print help
  -V, --version          Print version
```

### Commands

#### `scan` - Scan Storage Locations
```bash
wupdedup-rs scan [OPTIONS]

Options:
  --local <PATH>         Scan local filesystem path
  --smugmug              Scan SmugMug galleries
  --s3 <BUCKET>          Scan S3 bucket
  --gcs <BUCKET>         Scan Google Cloud Storage bucket
  --azure <CONTAINER>    Scan Azure blob container
  
  --recursive            Scan directories recursively [default: true]
  --follow-links         Follow symbolic links
  --exclude <PATTERN>    Exclude files matching pattern (can be repeated)
  --include <PATTERN>    Only include files matching pattern
  
  --parallel <N>         Number of parallel workers [default: CPU count]
  --batch-size <N>       Database batch commit size [default: 1000]
  --no-hash              Skip hash calculation
  --verify               Verify file integrity
  
  --tags <KEY=VALUE>     Add metadata tags (can be repeated)
  --bucket-name <NAME>   Database bucket name [default: strategy name]
  
Examples:
  # Scan local directory
  wupdedup-rs scan --local /photos
  
  # Scan with exclusions
  wupdedup-rs scan --local /home --exclude "*.tmp" --exclude ".git"
  
  # Scan multiple locations
  wupdedup-rs scan --local /photos --smugmug --s3 my-bucket
```

#### `dedupe` - Find and Handle Duplicates
```bash
wupdedup-rs dedupe [OPTIONS]

Options:
  --analyze              Analyze duplicates without taking action
  --auto                 Automatically handle duplicates
  --interactive          Interactive mode for manual selection
  
  --strategy <STRATEGY>  Deduplication strategy:
                         keep-newest    Keep newest file
                         keep-oldest    Keep oldest file
                         keep-largest   Keep largest file
                         keep-smallest  Keep smallest file
                         keep-original  Keep file in primary location
  
  --action <ACTION>      Action to take on duplicates:
                         delete         Delete duplicate files
                         archive        Move to archive location
                         hardlink       Replace with hard links
                         symlink        Replace with symbolic links
                         report         Generate report only
  
  --min-size <BYTES>     Minimum file size to consider
  --max-size <BYTES>     Maximum file size to consider
  --types <TYPES>        File types to process (comma-separated)
  
  --perceptual           Use perceptual hashing for images
  --threshold <VALUE>    Similarity threshold (0.0-1.0) [default: 1.0]
  --content-match        Verify duplicates by content comparison
  
  --dry-run              Simulate actions without making changes
  --archive-to <PATH>    Archive location for removed duplicates
  
Examples:
  # Analyze duplicates
  wupdedup-rs dedupe --analyze --report duplicates.html
  
  # Auto-cleanup keeping newest
  wupdedup-rs dedupe --auto --strategy keep-newest --action delete
  
  # Interactive deduplication
  wupdedup-rs dedupe --interactive
```

#### `stats` - Display Statistics
```bash
wupdedup-rs stats [OPTIONS]

Options:
  --detailed             Show detailed statistics
  --group-by <TYPE>      Group statistics by:
                         type        File type
                         size        Size ranges
                         date        Date ranges
                         storage     Storage location
                         extension   File extension
  
  --format <FORMAT>      Output format:
                         table       ASCII table [default]
                         json        JSON format
                         csv         CSV format
                         html        HTML report
  
  --output <FILE>        Write output to file
  --since <DATE>         Show stats since date
  --bucket <NAME>        Specific bucket to analyze
  
Examples:
  # Basic statistics
  wupdedup-rs stats
  
  # Detailed stats grouped by type
  wupdedup-rs stats --detailed --group-by type
  
  # Export to CSV
  wupdedup-rs stats --format csv --output stats.csv
```

#### `compare` - Compare Storage Locations
```bash
wupdedup-rs compare [OPTIONS]

Options:
  --source <LOCATION>    Source location
  --dest <LOCATION>      Destination location
  
  --verify-integrity     Verify file integrity with checksums
  --show-missing         Show files missing in destination
  --show-different       Show files with different content
  --show-newer           Show newer files in source
  
  --sync                 Synchronize differences
  --sync-direction       Sync direction: to-dest, to-source, bidirectional
  
Examples:
  # Compare local and S3
  wupdedup-rs compare --source /local/photos --dest s3://backup/photos
  
  # Verify backup completeness
  wupdedup-rs compare --source /data --dest /backup --verify-integrity
```

#### `verify` - Verify File Integrity
```bash
wupdedup-rs verify [OPTIONS]

Options:
  --location <PATH>      Location to verify
  --checksum             Verify checksums
  --deep                 Deep verification (read full content)
  --fix                  Attempt to fix issues
  
Examples:
  # Verify local storage
  wupdedup-rs verify --location /photos --checksum
  
  # Deep verification
  wupdedup-rs verify --location /backup --deep
```

#### `db` - Database Management
```bash
wupdedup-rs db <SUBCOMMAND>

Subcommands:
  verify                 Verify database integrity
  repair                 Repair database issues
  export <FILE>          Export database to file
  import <FILE>          Import database from file
  compact                Compact database
  stats                  Show database statistics
  
Examples:
  # Verify database
  wupdedup-rs db verify
  
  # Export for backup
  wupdedup-rs db export backup.db
```

## Rust API Documentation

### Core Traits

#### `StorageStrategy` Trait
```rust
#[async_trait]
pub trait StorageStrategy: Send + Sync {
    /// Scan the storage tree and populate the context
    async fn scan_tree(&self, context: &mut StorageStrategyContext) -> Result<()>;
    
    /// Return the strategy name
    fn name(&self) -> &str;
    
    /// Optional: Get file content
    async fn get_content(&self, path: &Path) -> Result<Vec<u8>> {
        Err(anyhow!("Content retrieval not implemented"))
    }
    
    /// Optional: Delete file
    async fn delete(&self, path: &Path) -> Result<()> {
        Err(anyhow!("Delete not implemented"))
    }
    
    /// Optional: Move file
    async fn move_file(&self, from: &Path, to: &Path) -> Result<()> {
        Err(anyhow!("Move not implemented"))
    }
}
```

#### `ContentAnalyzer` Trait
```rust
pub trait ContentAnalyzer {
    /// Analyze content and return metadata
    fn analyze(&self, content: &[u8]) -> ContentMetadata;
    
    /// Get content type
    fn get_type(&self, content: &[u8]) -> Option<String>;
    
    /// Calculate hash
    fn hash(&self, content: &[u8]) -> String;
}
```

### Core Structures

#### `StorageStrategyContext`
```rust
pub struct StorageStrategyContext {
    /// The storage strategy implementation
    pub storage_strategy: Arc<dyn StorageStrategy>,
    
    /// Strategy name
    pub name: String,
    
    /// Database bucket
    pub bucket: Option<Bucket>,
    
    /// Statistics
    pub file_count: usize,
    pub node_count: usize,
    pub total_size: u64,
    pub errors: Vec<String>,
}

impl StorageStrategyContext {
    /// Create new context
    pub fn new(strategy: Arc<dyn StorageStrategy>) -> Self;
    
    /// Set database bucket
    pub fn set_bucket(&mut self, bucket: Bucket);
    
    /// Add file entry
    pub fn add_file(&mut self, entry: FileEntry) -> Result<()>;
    
    /// Get statistics
    pub fn stats(&self) -> StorageStats;
}
```

#### `FileEntry`
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// File path
    pub path: PathBuf,
    
    /// Blake3 hash
    pub hash: String,
    
    /// File size in bytes
    pub size: u64,
    
    /// Last modified time
    pub modified: SystemTime,
    
    /// Content type (MIME)
    pub content_type: Option<String>,
    
    /// Storage location
    pub storage: String,
    
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}
```

#### `DeduplicationResult`
```rust
pub struct DeduplicationResult {
    /// Duplicate groups found
    pub groups: Vec<DuplicateGroup>,
    
    /// Total duplicates
    pub duplicate_count: usize,
    
    /// Space that can be recovered
    pub recoverable_space: u64,
    
    /// Actions taken
    pub actions: Vec<DedupeAction>,
}

pub struct DuplicateGroup {
    /// Hash identifying the group
    pub hash: String,
    
    /// Files in this group
    pub files: Vec<FileEntry>,
    
    /// Total size of duplicates
    pub total_size: u64,
    
    /// Suggested action
    pub suggested_action: DedupeAction,
}
```

### Database API

#### `Database` Struct
```rust
pub struct Database {
    db: Arc<redb::Database>,
}

impl Database {
    /// Open or create database
    pub fn new(path: &Path) -> Result<Self>;
    
    /// Create a new bucket
    pub fn create_bucket(&self, name: &str) -> Result<Bucket>;
    
    /// Get existing bucket
    pub fn get_bucket(&self, name: &str) -> Result<Option<Bucket>>;
    
    /// List all buckets
    pub fn list_buckets(&self) -> Result<Vec<String>>;
    
    /// Delete a bucket
    pub fn delete_bucket(&self, name: &str) -> Result<()>;
    
    /// Get database statistics
    pub fn stats(&self) -> Result<DatabaseStats>;
}
```

#### `Bucket` Struct
```rust
pub struct Bucket {
    name: String,
    db: Arc<redb::Database>,
}

impl Bucket {
    /// Store a file entry
    pub fn put(&self, key: &str, value: &FileEntry) -> Result<()>;
    
    /// Get a file entry
    pub fn get(&self, key: &str) -> Result<Option<FileEntry>>;
    
    /// Delete an entry
    pub fn delete(&self, key: &str) -> Result<()>;
    
    /// Scan all entries
    pub fn scan<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(&str, &FileEntry) -> Result<()>;
    
    /// Scan entries with prefix
    pub fn scan_prefix<F>(&self, prefix: &str, callback: F) -> Result<()>
    where
        F: Fn(&str, &FileEntry) -> Result<()>;
    
    /// Count entries
    pub fn count(&self) -> Result<usize>;
}
```

### Configuration API

#### `Config` Struct
```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// Log level
    pub log_level: String,
    
    /// Database file path
    pub db_file: String,
    
    /// Profiling configuration
    pub profile: ProfileConfig,
    
    /// Local storage configuration
    pub local: Option<LocalConfig>,
    
    /// SmugMug configuration
    pub smugmug: Option<SmugMugConfig>,
    
    /// S3 configuration
    pub s3: Option<S3Config>,
}

impl Config {
    /// Load configuration from file
    pub fn from_file(path: &Path) -> Result<Self>;
    
    /// Load from environment variables
    pub fn from_env() -> Result<Self>;
    
    /// Merge with another config (other takes precedence)
    pub fn merge(self, other: Config) -> Config;
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()>;
}
```

### Content Analysis API

#### `ContentType` Functions
```rust
/// Get MIME type from file content
pub fn get_type(content: &[u8]) -> Option<String>;

/// Check if content is an image
pub fn is_image(content: &[u8]) -> bool;

/// Check if content is a video
pub fn is_video(content: &[u8]) -> bool;

/// Check if content is audio
pub fn is_audio(content: &[u8]) -> bool;

/// Check if content is a document
pub fn is_document(content: &[u8]) -> bool;

/// Calculate Blake3 hash
pub fn calculate_hash(content: &[u8]) -> String;

/// Calculate perceptual hash for images
pub fn perceptual_hash(image_data: &[u8]) -> Result<String>;
```

### Profiling API

#### `Profiler` Struct
```rust
pub struct Profiler {
    cpu_profiler: Option<CpuProfiler>,
    memory_profiler: Option<MemoryProfiler>,
}

impl Profiler {
    /// Create new profiler
    pub fn new(config: ProfileConfig) -> Self;
    
    /// Start profiling
    pub fn start(&mut self);
    
    /// Stop profiling and save results
    pub fn stop(&mut self, output_path: &Path) -> Result<()>;
    
    /// Mark a profiling section
    pub fn section(&self, name: &str) -> ProfileSection;
}
```

## Environment Variables

```bash
# Core configuration
WUPDEDUP_LOG_LEVEL=debug|info|warn|error
WUPDEDUP_DB_FILE=/path/to/database.db
WUPDEDUP_CONFIG_FILE=/path/to/config.toml

# Performance tuning
WUPDEDUP_PARALLEL_WORKERS=8
WUPDEDUP_BATCH_SIZE=1000
WUPDEDUP_CACHE_SIZE=1073741824  # 1GB in bytes
WUPDEDUP_MEMORY_LIMIT=4294967296 # 4GB in bytes

# Local storage
WUPDEDUP_LOCAL_ROOT_PATH=/path/to/scan
WUPDEDUP_LOCAL_FOLLOW_LINKS=true|false
WUPDEDUP_LOCAL_EXCLUDE_PATTERNS="*.tmp,*.cache"

# SmugMug
WUPDEDUP_SMUGMUG_API_KEY=your_api_key
WUPDEDUP_SMUGMUG_API_SECRET=your_api_secret
WUPDEDUP_SMUGMUG_USER_ALIAS=your_alias

# S3
WUPDEDUP_S3_BUCKET=bucket-name
WUPDEDUP_S3_REGION=us-east-1
WUPDEDUP_S3_ACCESS_KEY=your_access_key
WUPDEDUP_S3_SECRET_KEY=your_secret_key

# Profiling
WUPDEDUP_PROFILE_ENABLED=true|false
WUPDEDUP_PROFILE_MODE=cpu|memory|trace|all
WUPDEDUP_PROFILE_OUTPUT=/path/to/output
```

## Configuration File Format

### config.toml
```toml
# Core settings
log_level = "info"
db_file = "wupdedup.db"

# Performance
[performance]
parallel_workers = 8
batch_size = 1000
cache_size = "1GB"
memory_limit = "4GB"

# Profiling
[profile]
enabled = false
mode = "cpu"
output_path = "./profiles"

# Local storage
[local]
root_path = "/path/to/scan"
follow_links = false
exclude_patterns = ["*.tmp", "*.cache", ".git"]

# SmugMug
[smugmug]
api_key = "your_key"
api_secret = "your_secret"
user_alias = "your_alias"

# S3
[s3]
bucket = "my-bucket"
region = "us-east-1"
access_key = "key"
secret_key = "secret"

# Deduplication settings
[deduplication]
strategy = "keep-newest"
action = "report"
perceptual_threshold = 0.95
verify_content = true
```

## REST API (Future)

### Endpoints

#### System
- `GET /health` - Health check
- `GET /metrics` - Prometheus metrics
- `GET /version` - Version information

#### Scanning
- `POST /scan` - Start scan operation
- `GET /scan/{id}` - Get scan status
- `DELETE /scan/{id}` - Cancel scan

#### Files
- `GET /files` - List files
- `GET /files/{hash}` - Get files by hash
- `DELETE /files/{path}` - Delete file

#### Deduplication
- `POST /dedupe` - Start deduplication
- `GET /dedupe/{id}` - Get dedup status
- `POST /dedupe/{id}/action` - Apply action

#### Statistics
- `GET /stats` - Global statistics
- `GET /stats/storage/{name}` - Storage statistics
- `GET /stats/duplicates` - Duplicate statistics

## Error Codes

| Code | Description |
|------|-------------|
| 0    | Success |
| 1    | General error |
| 2    | Configuration error |
| 3    | Database error |
| 4    | Storage access error |
| 5    | Network error |
| 6    | Authentication error |
| 7    | Invalid argument |
| 8    | Operation cancelled |
| 9    | Resource not found |
| 10   | Permission denied |

## Examples

### Using as a Library

```rust
use wupdedup_rs::{Config, Database, storage::LocalStrategy, StorageStrategyContext};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::from_env()?;
    
    // Initialize database
    let db = Database::new(&config.db_file)?;
    
    // Create storage strategy
    let strategy = Arc::new(LocalStrategy::new(config.local.unwrap()));
    
    // Create context
    let mut context = StorageStrategyContext::new(strategy.clone());
    
    // Set database bucket
    let bucket = db.create_bucket("local")?;
    context.set_bucket(bucket);
    
    // Perform scan
    strategy.scan_tree(&mut context).await?;
    
    // Print statistics
    println!("Files scanned: {}", context.file_count);
    println!("Total size: {} bytes", context.total_size);
    
    Ok(())
}
```

### Custom Storage Strategy

```rust
use wupdedup_rs::storage::{StorageStrategy, StorageStrategyContext};
use async_trait::async_trait;

pub struct CustomStrategy {
    // Your fields
}

#[async_trait]
impl StorageStrategy for CustomStrategy {
    async fn scan_tree(&self, context: &mut StorageStrategyContext) -> Result<()> {
        // Your implementation
        Ok(())
    }
    
    fn name(&self) -> &str {
        "custom"
    }
}
```

## Migration Guide

### From Go Version

1. **Database Migration**
   ```bash
   # Export from Go version
   wupdedup export --format json > data.json
   
   # Import to Rust version
   wupdedup-rs db import data.json
   ```

2. **Configuration Changes**
   - Go uses YAML, Rust uses TOML
   - Environment variable prefix: `FILESAGE_` → `WUPDEDUP_`
   - New fields for Blake3 and redb configuration

3. **API Changes**
   - Async operations by default
   - Result types instead of error returns
   - Trait-based storage strategies

## Performance Tuning

### Optimal Settings for Large Datasets
```toml
[performance]
parallel_workers = 16  # 2x CPU cores
batch_size = 5000      # Larger batches
cache_size = "4GB"     # More cache
memory_limit = "8GB"   # Higher limit

[database]
sync_mode = "async"    # Faster writes
cache_pages = 10000    # More cached pages
```

### Network Storage
```toml
[network]
connection_pool_size = 50
retry_attempts = 3
timeout_seconds = 30
rate_limit = 1000  # requests per second
```

## Troubleshooting

### Common Issues

1. **Database Lock Error**
   - Ensure only one instance is running
   - Check file permissions
   - Use `wupdedup-rs db repair`

2. **High Memory Usage**
   - Reduce batch_size
   - Lower cache_size
   - Enable memory_limit

3. **Slow Scanning**
   - Increase parallel_workers
   - Enable profiling to identify bottlenecks
   - Check network latency for cloud storage

4. **Missing Files**
   - Check exclude patterns
   - Verify permissions
   - Enable debug logging