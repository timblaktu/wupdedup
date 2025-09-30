# wupdedup Architecture Documentation

## System Overview

wupdedup is a multicloud storage management and deduplication system designed to analyze and organize digital content scattered across various storage providers. The system employs a modular architecture using the Strategy Pattern to abstract storage providers and enable extensible support for new storage backends.

## Core Design Principles

1. **Modularity**: Each component has a single responsibility and clear interfaces
2. **Extensibility**: New storage providers can be added without modifying core logic
3. **Performance**: Leverage Rust's zero-cost abstractions and parallelism
4. **Safety**: Memory and type safety guaranteed at compile time
5. **Async-First**: Non-blocking I/O for scalable operations

## Architecture Layers

```
┌─────────────────────────────────────────────────────┐
│                   CLI Interface                      │
│                   (clap + tokio)                     │
├─────────────────────────────────────────────────────┤
│                 Business Logic Layer                 │
│         (Scanning, Deduplication, Analysis)          │
├─────────────────────────────────────────────────────┤
│                 Storage Abstraction                  │
│              (Strategy Pattern Traits)               │
├──────────────┬──────────────┬──────────────┬────────┤
│    Local     │   SmugMug    │   S3/Cloud   │  ...   │
│   Storage    │     API      │   Storage    │        │
├──────────────┴──────────────┴──────────────┴────────┤
│                  Database Layer                      │
│                     (redb)                          │
├─────────────────────────────────────────────────────┤
│              Supporting Services                     │
│     (Config, Logging, Profiling, Content Type)      │
└─────────────────────────────────────────────────────┘
```

## Component Architecture

### 1. CLI Layer (`main.rs`)
- **Purpose**: Command-line interface and application entry point
- **Technology**: clap v4 for argument parsing, tokio for async runtime
- **Commands**:
  - `scan`: Scan storage locations for files
  - `stats`: Display statistics about stored data
  - `dedupe`: Find and handle duplicate files
  - `compare`: Compare files across storage locations

### 2. Storage Abstraction (`storage/mod.rs`)

#### Strategy Pattern Implementation
```rust
#[async_trait]
pub trait StorageStrategy: Send + Sync {
    async fn scan_tree(&self, context: &mut StorageStrategyContext) -> Result<()>;
    fn name(&self) -> &str;
}

pub struct StorageStrategyContext {
    pub storage_strategy: Arc<dyn StorageStrategy>,
    pub name: String,
    pub bucket: Option<Bucket>,
    pub file_count: usize,
    pub node_count: usize,
}
```

#### Storage Providers
- **LocalStrategy** (`storage/local.rs`): Local filesystem scanning
- **SmugMugStrategy** (`storage/smugmug.rs`): SmugMug API integration
- **CloudStrategy** (planned): S3, GCS, Azure Blob Storage

### 3. Database Layer (`db/mod.rs`)

#### redb Integration
- **Purpose**: Persistent storage for file metadata and hashes
- **Design**: Key-value store with bucket abstraction
- **Operations**:
  - `put`: Store file metadata
  - `get`: Retrieve file metadata
  - `scan`: Iterate over entries
  - `scan_prefix`: Search by key prefix

#### Data Model
```rust
// File entry stored in database
{
    key: "path/to/file.jpg",
    value: {
        hash: "blake3_hash_value",
        size: 1024000,
        modified: "2024-01-01T00:00:00Z",
        content_type: "image/jpeg",
        storage: "local",
        metadata: {...}
    }
}
```

### 4. Content Analysis (`content/mod.rs`)
- **MIME Type Detection**: Using `infer` crate for magic byte detection
- **Content Classification**: Image, video, audio, document detection
- **Hashing**: Blake3 for fast cryptographic hashing
- **Future**: Perceptual hashing for media similarity

### 5. Configuration (`config/mod.rs`)

#### Configuration Hierarchy
1. Default values
2. Configuration files (`config.toml`, `config.local.toml`)
3. Environment variables (`WUPDEDUP_*`)
4. Command-line arguments

#### Configuration Structure
```rust
pub struct Config {
    pub log_level: String,
    pub db_file: String,
    pub profile: ProfileConfig,
    pub local: Option<LocalConfig>,
    pub smugmug: Option<SmugMugConfig>,
}
```

### 6. Logging and Observability (`logging.rs`)
- **Structured Logging**: Using `tracing` crate
- **Log Levels**: trace, debug, info, warn, error
- **Output Formats**: Human-readable or JSON
- **Integration**: OpenTelemetry compatible

### 7. Profiling (`profiler/mod.rs`)
- **CPU Profiling**: Track execution time
- **Memory Profiling**: Monitor allocations
- **Trace Profiling**: Flamegraph generation
- **Benchmarking**: Performance regression detection

## Data Flow

### Scanning Workflow
```
1. User initiates scan command
   ↓
2. Load configuration and initialize storage strategies
   ↓
3. Create database connection and buckets
   ↓
4. For each storage strategy:
   a. Initialize strategy context
   b. Walk file tree (parallel with Rayon)
   c. For each file:
      - Calculate hash (Blake3)
      - Extract metadata
      - Store in database
   ↓
5. Generate scan report
```

### Deduplication Workflow
```
1. Query database for all file entries
   ↓
2. Group files by hash value
   ↓
3. Identify duplicate groups (same hash)
   ↓
4. For each duplicate group:
   a. Verify duplicates (optional byte comparison)
   b. Apply deduplication strategy:
      - Report only
      - Delete duplicates
      - Create hard links
      - Move to archive
   ↓
5. Update database and generate report
```

## Concurrency Model

### Parallel Processing
- **File Scanning**: Rayon parallel iterator for directory traversal
- **Hash Calculation**: Thread pool for CPU-intensive operations
- **Database Operations**: Arc<Database> for thread-safe access

### Async Operations
- **Network I/O**: Tokio async runtime for API calls
- **File I/O**: Async file operations with tokio::fs
- **Database Writes**: Batched async transactions

## Error Handling

### Error Strategy
```rust
// Using anyhow for error propagation
use anyhow::{Result, Context};

// All fallible operations return Result<T>
pub fn operation() -> Result<Data> {
    let data = risky_operation()
        .context("Failed to perform risky operation")?;
    Ok(data)
}
```

### Error Categories
1. **Configuration Errors**: Invalid settings, missing credentials
2. **Storage Errors**: Access denied, network failures
3. **Database Errors**: Corruption, disk full
4. **Content Errors**: Unsupported formats, corrupt files

## Security Considerations

### Credential Management
- Environment variables for sensitive data
- No credentials in configuration files
- Secure token storage for OAuth

### Data Integrity
- Cryptographic hashing (Blake3)
- Database transactions for consistency
- Verification before destructive operations

### Access Control
- File system permissions respected
- API rate limiting
- Audit logging for operations

## Performance Optimizations

### Memory Management
- Streaming file processing (no full file loads)
- Bounded channels for backpressure
- Arena allocators for bulk operations

### I/O Optimization
- Buffered readers/writers
- Memory-mapped files for large datasets
- Connection pooling for APIs

### Caching
- LRU cache for frequently accessed metadata
- Hash cache to avoid recalculation
- API response caching

## Extensibility Points

### Adding New Storage Providers
1. Implement `StorageStrategy` trait
2. Add configuration structure
3. Register in `load_storage_strategy_contexts`
4. Add CLI options if needed

### Adding New Deduplication Algorithms
1. Implement deduplication trait
2. Add to algorithm registry
3. Update CLI with new options

### Adding New Content Analyzers
1. Implement analyzer trait
2. Register with content module
3. Update database schema if needed

## Testing Strategy

### Unit Tests
- Module-level tests for each component
- Mock traits for dependency injection
- Property-based testing with proptest

### Integration Tests
- End-to-end workflow tests
- Real filesystem operations in temp directories
- Database operation verification

### Performance Tests
- Benchmarks with criterion
- Memory profiling with valgrind
- Load testing with large datasets

## Deployment Considerations

### Binary Distribution
- Single static binary
- Cross-compilation for multiple platforms
- Container images for cloud deployment

### Configuration Management
- Environment-based configuration
- Secrets management integration
- Configuration validation on startup

### Monitoring
- Prometheus metrics endpoint
- Health check endpoints
- Structured logging for aggregation

## Future Architecture Enhancements

### Distributed Processing
- Worker pool architecture
- Message queue integration
- Distributed hash table for deduplication

### Machine Learning Integration
- Semantic deduplication
- Content classification
- Anomaly detection

### Web Interface
- REST API server
- WebSocket for real-time updates
- React/Vue.js dashboard

## Conclusion

The wupdedup architecture provides a solid foundation for multicloud storage management with clear separation of concerns, extensible design patterns, and performance-oriented implementation. The use of Rust ensures memory safety and performance while the modular design allows for easy extension and maintenance.