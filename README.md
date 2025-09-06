# wupdedup-rs

A high-performance file deduplication and multicloud storage management tool written in Rust.

## Features

- **Fast Scanning**: Parallel file scanning with Blake3 hashing for superior performance
- **Automatic Deduplication**: Remove, move, archive, or symlink duplicate files with multiple strategies
- **Multiple Storage Backends**: Support for local filesystem and SmugMug (more coming soon)
- **Efficient Database**: Uses embedded redb for fast, reliable metadata storage
- **Safe Operations**: Dry-run mode, file existence validation, and comprehensive error handling
- **Cross-platform**: Works on Linux, macOS, and Windows

## Installation

```bash
# Clone the repository
git clone https://github.com/timblaktu/wupdedup
cd wupdedup

# Build the release version
cargo build --release

# The binary will be at target/release/wupdedup-rs
```

## Usage

### Scan local files

```bash
# Scan a directory
wupdedup-rs scan --local /path/to/directory

# Scan with custom database location
wupdedup-rs --db-file custom.db scan --local /path/to/directory

# Scan with debug logging
wupdedup-rs --log-level debug scan --local /path/to/directory
```

### Find and process duplicates

```bash
# Show duplicate files without taking action
wupdedup-rs dedupe --show-only

# Delete duplicates (keeps first file alphabetically)
wupdedup-rs dedupe --strategy delete --auto

# Move duplicates to a directory
wupdedup-rs dedupe --strategy move --target-dir /path/to/duplicates --auto

# Archive duplicates (preserves directory structure)
wupdedup-rs dedupe --strategy archive --target-dir /path/to/archive --auto

# Create symlinks to original (Unix only)
wupdedup-rs dedupe --strategy symlink --auto

# Dry run - preview changes without applying them
wupdedup-rs dedupe --strategy delete --dry-run --auto

# Interactive mode (prompts for each duplicate)
wupdedup-rs dedupe --strategy delete
```

### View statistics

```bash
# Show indexed file counts
wupdedup-rs stats
```

## Configuration

wupdedup-rs can be configured through:

1. **Command-line arguments** (highest priority)
2. **Environment variables** (prefix with `WUPDEDUP_`)
3. **.env file** in the current directory
4. **config.toml** or **config.local.toml** files

### Environment Variables

```bash
# Set log level
export WUPDEDUP_LOG_LEVEL=debug

# Set database file location
export WUPDEDUP_DB_FILE=/path/to/database.db

# Set local scan root
export WUPDEDUP_LOCAL_ROOT_PATH=/path/to/scan
```

### Configuration File (config.toml)

```toml
log_level = "info"
db_file = "wupdedup.db"

[profile]
enabled = false
mode = "cpu"  # Options: cpu, memory, trace

[local]
root_path = "/path/to/files"

[smugmug]
url = "https://api.smugmug.com"
api_key = "your_api_key"
api_secret = "your_api_secret"
user_token = "your_user_token"
user_secret = "your_user_secret"
destination = "Albums"
file_names = "original"
use_metadata_times = true
force_metadata_times = false
```

## Performance

wupdedup-rs is optimized for performance:

- **Blake3 hashing**: Extremely fast cryptographic hashing
- **Parallel processing**: Uses all available CPU cores via rayon
- **Efficient I/O**: Chunked file reading with 64KB buffers
- **Smart caching**: Embedded database prevents redundant work

### Benchmarks

Run performance benchmarks with:

```bash
cargo bench
```

Typical performance on modern hardware:
- **Hashing**: ~3 GB/s for large files
- **Scanning**: 1000+ files/second
- **Database**: Sub-millisecond lookups

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --lib           # Unit tests
cargo test --test integration_test  # Integration tests
cargo test --test cli_test  # CLI end-to-end tests

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

### Project Structure

```
wupdedup/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── config/           # Configuration management
│   ├── db/               # Database abstraction with indexing
│   ├── dedupe/           # Deduplication engine
│   │   └── mod.rs        # Strategies: delete, move, archive, symlink
│   ├── storage/          # Storage strategy implementations
│   │   ├── local.rs      # Local filesystem scanner with FileInfo
│   │   └── smugmug.rs    # SmugMug API integration
│   ├── content.rs        # MIME type detection
│   ├── logging.rs        # Structured logging setup
│   └── profiler/         # Performance profiling
├── tests/
│   ├── integration_test.rs  # Integration tests
│   ├── property_tests.rs    # Property-based tests
│   └── cli_test.rs          # CLI tests
├── benches/
│   ├── hash_benchmark.rs    # Hashing performance
│   └── db_benchmark.rs      # Database performance
└── docs/
    └── TESTING.md           # Testing documentation
```

## Architecture

wupdedup-rs uses a strategy pattern for storage backends, allowing easy extension:

```rust
#[async_trait]
pub trait StorageStrategy: Send + Sync {
    async fn scan_tree(&self, context: &mut StorageStrategyContext) -> Result<()>;
    fn name(&self) -> &str;
}
```

Each storage backend (local, SmugMug, etc.) implements this trait, enabling uniform handling of disparate storage systems.

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Write tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

MIT License - see LICENSE file for details

## Acknowledgments

- Original Go implementation: [smugmug-backup](https://github.com/tommyblue/smugmug-backup)
- Blake3 team for the blazing fast hash algorithm
- The Rust community for excellent crates and tooling