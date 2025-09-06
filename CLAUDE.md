# wupdedup-rs Project Context

## Overview
wupdedup-rs is a high-performance file deduplication and multicloud storage management tool written in Rust. It scans local and cloud storage to identify duplicate files, helping users manage their digital content efficiently.

## Architecture

### Core Design Patterns
- **Strategy Pattern**: Storage backends (local, SmugMug, etc.) implement a common `StorageStrategy` trait
- **Parallel Processing**: Uses rayon for CPU-bound operations (file hashing)
- **Async I/O**: Uses tokio for I/O-bound operations (file system traversal)
- **Embedded Database**: Uses redb for persistent metadata storage

### Key Components

1. **Storage Strategies** (`src/storage/`)
   - `local.rs`: Local filesystem scanner with parallel Blake3 hashing
   - `smugmug.rs`: SmugMug API integration (stubbed, not yet implemented)
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

## Testing Strategy

### Test Coverage (97 tests total, 94 passing)
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

## Development Workflow

### Building
```bash
cargo build           # Debug build
cargo build --release # Optimized build
```

### Testing
```bash
cargo test           # Run all tests
cargo test --lib     # Unit tests only
cargo test --test integration_test  # Integration tests
cargo test --test cli_test         # CLI tests
cargo bench          # Run benchmarks
```

### Code Quality
```bash
cargo fmt            # Format code
cargo clippy         # Lint code
cargo doc --open     # Generate documentation
```

## Common Tasks

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

## Known Issues and TODOs

### Current Limitations
- SmugMug integration not yet implemented
- No automatic deduplication (only detection)
- Database schema could benefit from indexes
- 3 CLI tests have minor assertion issues

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