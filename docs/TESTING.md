# Testing Infrastructure

## Overview

The wupdedup-rs project has a comprehensive test suite following Rust best practices for Test-Driven Development (TDD). The test infrastructure supports parallel execution, property-based testing, performance benchmarking, and code coverage reporting.

## Test Structure

### Unit Tests
Located within each module alongside the implementation code:
- `src/db/mod.rs` - Database operations tests
- `src/content/mod.rs` - Content type detection tests  
- `src/storage/local.rs` - Local storage strategy tests

### Integration Tests
Located in `tests/` directory:
- `integration_test.rs` - Full stack integration tests
- `property_tests.rs` - Property-based tests for invariants
- `common/mod.rs` - Shared test fixtures and utilities

### Benchmarks
Located in `benches/` directory:
- `hash_benchmark.rs` - Blake3 hashing performance
- `db_benchmark.rs` - Database operation performance

## Test Dependencies

```toml
[dev-dependencies]
tempfile = "3.14"           # Temporary directories for tests
pretty_assertions = "1.4"   # Better assertion output
criterion = "0.5"           # Benchmarking framework
proptest = "1.5"           # Property-based testing
rstest = "0.23"            # Parameterized tests
rstest_reuse = "0.7"       # Reusable test templates
mockall = "0.13"           # Mocking framework
serial_test = "3.1"        # Serial test execution
once_cell = "1.20"         # Lazy statics
test-case = "3.1"          # Test case generation
wiremock = "0.6"           # HTTP mocking
```

## Running Tests

### All Tests
```bash
cargo test
```

### Unit Tests Only
```bash
cargo test --lib
```

### Integration Tests Only
```bash
cargo test --test '*'
```

### Property Tests
```bash
cargo test --test property_tests
```

### With Output
```bash
cargo test -- --nocapture
```

### Specific Test
```bash
cargo test test_scan_empty_directory
```

### Ignored Tests (Slow)
```bash
cargo test -- --ignored
```

### Parallel Execution
Tests run in parallel by default. Use `#[serial]` attribute for tests requiring exclusive access.

## Test Fixtures

### TestFixture
Main test fixture providing:
- Temporary directory management
- Database initialization
- File creation utilities
- Standard test layouts
- Duplicate file scenarios

Example usage:
```rust
let fixture = TestFixture::new()?.with_db()?;
fixture.create_standard_layout()?;
```

### TestScenarioBuilder
Builder pattern for complex test scenarios:
```rust
let fixture = TestScenarioBuilder::new()?
    .with_files(1000)
    .with_duplicate_group(vec!["file1.txt", "file2.txt"])
    .build()?;
```

## Property-Based Testing

Using `proptest` for testing invariants:
- Hash determinism
- Collision resistance
- Database consistency
- Path handling robustness
- Concurrent operation safety

Example:
```rust
proptest! {
    #[test]
    fn prop_hash_deterministic(
        content in prop::collection::vec(any::<u8>(), 0..1000)
    ) {
        // Test that same content always produces same hash
    }
}
```

## Performance Testing

### Running Benchmarks
```bash
cargo bench
```

### Specific Benchmark
```bash
cargo bench hash_benchmark
```

### With HTML Report
```bash
cargo bench -- --save-baseline before
# Make changes
cargo bench -- --baseline before
```

## Code Coverage

### Generate Coverage Report
```bash
./scripts/test_coverage.sh
```

### Coverage Reports
- HTML: `tarpaulin-report.html`
- XML: `cobertura.xml`

### CI Integration
GitHub Actions workflow automatically:
- Runs tests on multiple platforms (Linux, macOS, Windows)
- Tests against stable, beta, and nightly Rust
- Generates and uploads coverage to Codecov
- Runs clippy and formatting checks

## Testing Best Practices

### 1. Test Organization
- Keep unit tests close to implementation
- Use descriptive test names
- Group related tests in modules

### 2. Test Data
- Use fixtures for consistent test data
- Create minimal reproducible scenarios
- Clean up resources in Drop implementations

### 3. Assertions
- Use specific assertions (`assert_eq!` over `assert!`)
- Include context in assertion messages
- Test both success and failure cases

### 4. Performance
- Use `#[ignore]` for slow tests
- Parallelize where possible
- Mock expensive operations

### 5. Coverage Goals
- Aim for >80% code coverage
- Focus on critical paths
- Test edge cases and error conditions

## Common Test Patterns

### Testing Async Code
```rust
#[tokio::test]
async fn test_async_operation() -> Result<()> {
    let result = async_function().await?;
    assert_eq!(result, expected);
    Ok(())
}
```

### Testing File Operations
```rust
let temp_dir = tempdir()?;
let file_path = temp_dir.path().join("test.txt");
fs::write(&file_path, b"content")?;
```

### Testing Database Operations
```rust
let db = DB::init(":memory:")?;
let bucket = db.bucket("test")?;
bucket.put("key", b"value")?;
```

### Testing Concurrent Operations
```rust
#[test]
#[serial]
fn test_exclusive_access() {
    // Test requiring exclusive access
}
```

## Troubleshooting

### Test Failures
1. Run with `--nocapture` to see output
2. Use `RUST_BACKTRACE=1` for stack traces
3. Run single test in isolation
4. Check for race conditions in parallel tests

### Flaky Tests
1. Use `#[serial]` for tests with shared state
2. Ensure proper cleanup in Drop
3. Avoid time-dependent assertions
4. Mock external dependencies

### Performance Issues
1. Use `#[ignore]` for slow tests
2. Reduce test data size
3. Mock expensive operations
4. Profile with `cargo test -- --profile`