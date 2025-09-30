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

### Core Design Patterns
- **Strategy Pattern**: Storage backends (local, SmugMug, etc.) implement a common `StorageStrategy` trait
- **Parallel Processing**: Uses rayon for CPU-bound operations (file hashing)
- **Async I/O**: Uses tokio for I/O-bound operations (file system traversal)
- **Embedded Database**: Uses redb for persistent metadata storage

### Key Components

1. **Storage Strategies** (`src/storage/`)
   - `local.rs`: Local filesystem scanner with parallel Blake3 hashing
     - `FileInfo`: Public struct for file metadata (path, size, file_type, hash, modified)
   - `smugmug.rs`: SmugMug API integration with OAuth 1.0a support
   - `mod.rs`: Strategy trait and context management

2. **Database Layer** (`src/db/`)
   - Embedded redb database for metadata storage
   - Key-value store with buckets for different storage sources
   - Methods: `put()`, `get()`, `delete()`, `exists()`, `scan_prefix()`, `all()`, `count()`

3. **Configuration** (`src/config/`)
   - Hierarchical configuration: CLI args > env vars > .env > config.toml
   - Supports profiles for CPU/memory profiling
   - Per-storage backend configuration

4. **CLI Interface** (`src/main.rs`)
   - Commands: `scan`, `dedupe`, `stats`
   - Uses clap for argument parsing
   - Structured logging with tracing

### Strategy Pattern Implementation

```rust
#[async_trait]
pub trait StorageStrategy: Send + Sync {
    async fn scan_tree(&self, context: &mut StorageStrategyContext) -> Result<()>;
    fn name(&self) -> &str;
}
```

Each storage backend implements this trait, enabling uniform handling of disparate storage systems.

## Testing Strategy

### Test Coverage (99 tests total, 96+ passing)
- **Unit Tests** (43): In-module tests for individual components
- **Integration Tests** (14): `tests/integration_test.rs` - full workflow tests
- **Property Tests** (12): `tests/property_tests.rs` - invariant testing with proptest
- **CLI Tests** (11): `tests/cli_test.rs` - end-to-end command testing
- **Benchmarks**: `benches/` - performance testing for hashing and database

### Test Utilities (`tests/common/`)
- `TestFixture`: Creates isolated test environments with temp directories
- `TestScenarioBuilder`: Generates test file hierarchies
- `PerfTimer`: Performance assertion utilities
- Standard test layouts for consistent testing

## Performance Characteristics

### Hashing Performance
- Blake3 achieves ~3 GB/s on modern hardware
- 64KB chunk size optimal for I/O and hashing balance
- Parallel processing scales linearly with CPU cores

### Scanning Performance
- 1000+ files/second typical throughput
- Memory usage: O(1) for scanning (streaming)
- Database writes are batched for efficiency

## Development

### ⚠️ ZERO WARNINGS POLICY

**Critical Rule**: This project treats ALL warnings as failures. Builds must be completely warning-free.

**Enforcement Mechanisms**:
- **Cargo.toml**: `[lints.rust]` section denies warnings globally
- **CI Builds**: `RUSTFLAGS="-D warnings"` fails build on any warning
- **Development**: More permissive but still denies critical warnings (unused imports, unused variables)
- **Clippy**: Configured with `--deny warnings` in CI checks

**For Development**: 
- Use `#[allow(dead_code)]` with explicit comments for intentionally unused code
- Fix all warnings immediately - never commit code with warnings
- Use `cargo clippy` regularly during development

### Nix-based Development Environment (Recommended)

This project uses a comprehensive Nix flake for reproducible development environments with proper native dependency management.

#### Quick Start
```bash
# Enter development environment
nix develop

# Or build directly without entering shell
nix develop --command cargo build --release
```

#### Development Environment Features
- **Rust toolchain**: Stable Rust 1.89+ via Fenix (modern rustup replacement)
- **Native dependencies**: OpenSSL, pkg-config automatically available
- **Development tools**: cargo-watch, cargo-nextest, rust-analyzer, clippy, rustfmt
- **Performance tools**: valgrind, perf-tools, hyperfine for profiling
- **Cross-platform**: Works on Linux, macOS, and NixOS

#### Available Development Shells
```bash
nix develop                    # Full development environment
nix develop .#ci              # Minimal CI environment  
nix develop .#perf            # Performance testing with profiling tools
```

#### Nix Flake Commands
```bash
nix flake check               # Run all CI checks (build, test, clippy, fmt)
nix build                     # Build the application
nix run                       # Run the application
```

### Traditional Building (if not using Nix)
```bash
cargo build           # Debug build
cargo build --release # Optimized build
```

**Note**: On NixOS or when native dependencies (OpenSSL, pkg-config) are missing, use the Nix development environment above.

### Testing
```bash
# Using Nix environment (recommended)
nix develop --command cargo nextest run     # Fast parallel testing
nix develop --command cargo test            # Standard testing
nix develop --command cargo bench           # Run benchmarks

# Traditional commands (if not using Nix)
cargo test           # Run all tests
cargo test --lib     # Unit tests only
cargo test --test integration_test  # Integration tests
cargo test --test cli_test         # CLI tests
cargo bench          # Run benchmarks
```

### Code Quality
```bash
# Using Nix environment (recommended)
nix flake check                              # Run all checks (build, test, clippy, fmt)
nix develop --command cargo clippy           # Lint code
nix develop --command cargo fmt              # Format code
nix develop --command cargo doc --open       # Generate documentation

# Traditional commands (if not using Nix)
cargo fmt            # Format code
cargo clippy         # Lint code
cargo doc --open     # Generate documentation
```

## Common Development Tasks

### Adding a New Storage Backend
1. Create new file in `src/storage/` (e.g., `s3.rs`)
2. Implement `StorageStrategy` trait
3. Add configuration struct in `src/config/mod.rs`
4. Update `load_storage_strategy_contexts()` in `src/storage/mod.rs`
5. Add tests in the implementation file

### Adding a New CLI Command
1. Add variant to `Commands` enum in `src/main.rs`
2. Implement handler function (e.g., `run_new_command()`)
3. Add command logic in main match statement
4. Add CLI tests in `tests/cli_test.rs`

### Database Schema Changes
1. Modify `FileInfo` struct in `src/storage/local.rs`
2. Update serialization/deserialization logic
3. Consider migration strategy for existing databases
4. Update tests to verify new fields

## Recent Updates (2025-09-07)

### Completed: SmugMug API Integration with Mock Testing
- **Integrated official smugmug crate (v0.6)**: Replaced custom implementation with battle-tested OAuth 1.0a client
- **Proper API v2 support**: Connected to real SmugMug REST endpoints with correct authentication flow
- **Mock mode implementation**: Created comprehensive mock testing system for development without credentials
- **Stream-based processing**: Albums and images fetched as async streams for memory efficiency
- **Test coverage**: 99 total tests passing, including 9 SmugMug-specific tests (6 unit, 3 integration)

### Mock Mode Features
- **No credentials required**: Use `mock_mode = true` in config for testing
- **Realistic test data**: Generates 3 albums with 16 total images
- **Deterministic output**: Consistent data across runs for reliable testing
- **Full database integration**: Mock data stored in same format as real SmugMug data
- **CI/CD friendly**: All tests run offline without external dependencies

### SmugMug Configuration
```toml
[smugmug]
api_key = "your_key"           # Get from https://api.smugmug.com/api/developer/apply
api_secret = "your_secret"
access_token = "oauth_token"    # From OAuth 1.0a flow
access_token_secret = "oauth_secret"
mock_mode = false               # Set true for testing without credentials
```

### Completed: Real Duplicate File Processing (2025-09-06)
- **Integrated deduplication engine with database**: The dedupe command now properly deserializes FileInfo structs and processes actual files
- **Added file existence validation**: Files are checked before processing, with missing files logged and skipped
- **Enhanced error handling**: Better handling of missing files, permission errors, and edge cases
- **Consistent file ordering**: Files are sorted alphabetically to ensure deterministic behavior
- **Comprehensive test coverage**: Added 5 new integration tests for all deduplication strategies

## Environment-Specific Notes

### NixOS and OpenSSL Dependencies

This project requires OpenSSL and pkg-config for some dependencies (reqwest, object_store). The Nix flake automatically provides these dependencies.

**Why this is needed:**
- `reqwest` with `rustls-tls` still pulls in `native-tls` through default features
- `object_store` cloud backends require OpenSSL for TLS connections
- Traditional package managers install these to standard locations, but NixOS uses `/nix/store/` paths

**Build Environment Comparison:**
- **Termux**: Has `pkg-config` and `openssl-dev` in standard locations ✅
- **Ubuntu/Debian**: Install with `apt install pkg-config libssl-dev` ✅  
- **NixOS**: Use `nix develop` or install via `environment.systemPackages` ✅
- **macOS**: Additional frameworks needed (Security, SystemConfiguration) ✅

The flake handles all of these automatically across platforms.

### Direnv Integration (Optional)

For automatic environment loading:

```bash
# Create .envrc file
echo "use flake" > .envrc

# Allow direnv to load the environment
direnv allow

# Now the environment loads automatically when entering the directory
```

## Known Issues and TODOs

### Current Limitations
- SmugMug OAuth token acquisition not automated (must be obtained manually)
- SmugMug image dimensions not available in current API response structure
- Database schema could benefit from additional indexes
- Some database methods (find_by_hash, find_by_size, find_by_type) are implemented but not yet utilized

### Future Enhancements
- Add S3/GCS/Azure storage backends
- Implement content-defined chunking for large files
- Add perceptual hashing for images
- Implement automatic deduplication strategies
- Add web UI for visualization

## Dependencies

### Core Dependencies
- `tokio`: Async runtime
- `redb`: Embedded database
- `blake3`: Fast cryptographic hashing
- `rayon`: Parallel processing
- `clap`: CLI argument parsing
- `tracing`: Structured logging
- `serde`: Serialization

### Development Dependencies
- `criterion`: Benchmarking framework
- `proptest`: Property-based testing
- `assert_cmd`: CLI testing
- `tempfile`: Test isolation
- `rstest`: Test fixtures

## Performance Tuning

### Environment Variables
```bash
RAYON_NUM_THREADS=8  # Limit parallel threads
RUST_LOG=debug       # Enable debug logging
```

### Profiling
```bash
# CPU profiling
cargo build --release
perf record --call-graph=dwarf ./target/release/wupdedup-rs scan --local /path
perf report

# Memory profiling
valgrind --tool=massif ./target/release/wupdedup-rs scan --local /path
ms_print massif.out.*
```

## Debugging Tips

1. **Enable debug logging**: `RUST_LOG=debug cargo run`
2. **Test single file**: Create minimal test case in `test_scan_dir/`
3. **Database inspection**: Use redb CLI tools or write custom inspector
4. **Benchmark specific operations**: Use criterion's `bench_function`

## Code Style Guidelines

- Use `anyhow::Result` for error handling
- Prefer `tracing` over `println!` for logging
- Keep functions under 50 lines
- Write tests for all public APIs
- Document complex algorithms
- Use clippy lints: `#![warn(clippy::all)]`

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