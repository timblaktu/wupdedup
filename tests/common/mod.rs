use anyhow::Result;
use once_cell::sync::Lazy;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::{tempdir, TempDir};
use wupdedup_rs::config::{Config, LocalConfig};
use wupdedup_rs::db::DB;

// Global test mutex for tests that need exclusive access
pub static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Test fixture for managing temporary directories and files
pub struct TestFixture {
    pub temp_dir: TempDir,
    pub root_path: PathBuf,
    pub db_path: PathBuf,
    pub db: Option<DB>,
}

impl TestFixture {
    /// Create a new test fixture with temp directory
    pub fn new() -> Result<Self> {
        let temp_dir = tempdir()?;
        let root_path = temp_dir.path().join("test_files");
        let db_path = temp_dir.path().join("test.db");

        fs::create_dir_all(&root_path)?;

        Ok(Self {
            temp_dir,
            root_path,
            db_path,
            db: None,
        })
    }

    /// Initialize database for the fixture
    pub fn with_db(mut self) -> Result<Self> {
        // Ensure the parent directory exists
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.db = Some(DB::init(self.db_path.to_str().unwrap())?);
        Ok(self)
    }

    /// Create a test file with specific content
    pub fn create_file(&self, relative_path: &str, content: &[u8]) -> Result<PathBuf> {
        let file_path = self.root_path.join(relative_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    /// Create multiple test files
    pub fn create_files(&self, files: &[(&str, &[u8])]) -> Result<Vec<PathBuf>> {
        files
            .iter()
            .map(|(path, content)| self.create_file(path, content))
            .collect()
    }

    /// Create a standard test file structure
    pub fn create_standard_layout(&self) -> Result<()> {
        self.create_files(&[
            ("photos/vacation/beach.jpg", b"fake jpeg content 1"),
            ("photos/vacation/sunset.jpg", b"fake jpeg content 2"),
            ("photos/family/portrait.jpg", b"fake jpeg content 3"),
            ("documents/report.pdf", b"fake pdf content"),
            ("documents/notes.txt", b"some text notes"),
            ("downloads/installer.exe", b"fake exe content"),
            ("downloads/archive.zip", b"fake zip content"),
        ])?;
        Ok(())
    }

    /// Create files with duplicate content
    pub fn create_duplicates(&self) -> Result<()> {
        let duplicate_content = b"this is duplicate content";
        self.create_files(&[
            ("originals/file1.txt", duplicate_content),
            ("copies/file1_copy.txt", duplicate_content),
            ("backups/file1_backup.txt", duplicate_content),
            ("unique/different.txt", b"unique content here"),
        ])?;
        Ok(())
    }

    /// Create large file for performance testing
    pub fn create_large_file(&self, name: &str, size_mb: usize) -> Result<PathBuf> {
        let content = vec![0u8; size_mb * 1024 * 1024];
        self.create_file(name, &content)
    }

    /// Get test configuration
    pub fn get_config(&self) -> Config {
        Config {
            log_level: "debug".to_string(),
            db_file: self.db_path.to_string_lossy().to_string(),
            local: Some(LocalConfig {
                root_path: self.root_path.clone(),
            }),
            smugmug: None,
            profile: Default::default(),
        }
    }

    /// Verify file exists in database
    pub fn verify_in_db(&self, relative_path: &str) -> Result<bool> {
        if let Some(db) = &self.db {
            let bucket = db.bucket("test")?;
            let full_path = self.root_path.join(relative_path);
            let key = full_path.to_string_lossy();
            Ok(bucket.get(&key)?.is_some())
        } else {
            Ok(false)
        }
    }

    /// Count total files in database
    pub fn count_db_files(&self) -> Result<usize> {
        if let Some(db) = &self.db {
            let bucket = db.bucket("test")?;
            Ok(bucket.count()?)
        } else {
            Ok(0)
        }
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        // Ensure database is closed before temp dir is removed
        if let Some(db) = self.db.take() {
            let _ = db.close();
        }
    }
}

/// Builder for creating complex test scenarios
pub struct TestScenarioBuilder {
    fixture: TestFixture,
    file_count: usize,
    duplicate_groups: Vec<Vec<String>>,
}

impl TestScenarioBuilder {
    pub fn new() -> Result<Self> {
        Ok(Self {
            fixture: TestFixture::new()?,
            file_count: 0,
            duplicate_groups: Vec::new(),
        })
    }

    pub fn with_files(mut self, count: usize) -> Self {
        self.file_count = count;
        self
    }

    pub fn with_duplicate_group(mut self, paths: Vec<String>) -> Self {
        self.duplicate_groups.push(paths);
        self
    }

    pub fn build(self) -> Result<TestFixture> {
        // Create unique files
        for i in 0..self.file_count {
            let content = format!("unique content {}", i);
            self.fixture
                .create_file(&format!("file_{}.txt", i), content.as_bytes())?;
        }

        // Create duplicate groups
        for (group_idx, paths) in self.duplicate_groups.iter().enumerate() {
            let content = format!("duplicate group {}", group_idx);
            for path in paths {
                self.fixture.create_file(path, content.as_bytes())?;
            }
        }

        Ok(self.fixture)
    }
}

/// Assertion helpers
pub mod assertions {
    use super::*;
    use pretty_assertions::assert_eq;

    pub fn assert_file_exists(fixture: &TestFixture, relative_path: &str) {
        let full_path = fixture.root_path.join(relative_path);
        assert!(full_path.exists(), "File should exist: {:?}", full_path);
    }

    pub fn assert_file_content(fixture: &TestFixture, relative_path: &str, expected: &[u8]) {
        let full_path = fixture.root_path.join(relative_path);
        let actual = fs::read(&full_path).expect("Should read file");
        assert_eq!(
            actual, expected,
            "File content mismatch for {:?}",
            full_path
        );
    }

    pub fn assert_files_have_same_hash(
        fixture: &TestFixture,
        path1: &str,
        path2: &str,
    ) -> Result<()> {
        use blake3::Hasher;

        let file1 = fixture.root_path.join(path1);
        let file2 = fixture.root_path.join(path2);

        let hash1 = {
            let content = fs::read(&file1)?;
            let mut hasher = Hasher::new();
            hasher.update(&content);
            hasher.finalize()
        };

        let hash2 = {
            let content = fs::read(&file2)?;
            let mut hasher = Hasher::new();
            hasher.update(&content);
            hasher.finalize()
        };

        assert_eq!(hash1, hash2, "Files should have same hash");
        Ok(())
    }
}

/// Performance testing helpers
pub mod perf {
    use super::*;
    use std::time::{Duration, Instant};

    pub struct PerfTimer {
        start: Instant,
        name: String,
    }

    impl PerfTimer {
        pub fn new(name: impl Into<String>) -> Self {
            Self {
                start: Instant::now(),
                name: name.into(),
            }
        }

        pub fn elapsed(&self) -> Duration {
            self.start.elapsed()
        }

        pub fn assert_under(&self, max_duration: Duration) {
            let elapsed = self.elapsed();
            assert!(
                elapsed < max_duration,
                "{} took {:?}, expected under {:?}",
                self.name,
                elapsed,
                max_duration
            );
        }
    }

    impl Drop for PerfTimer {
        fn drop(&mut self) {
            println!("{} took {:?}", self.name, self.elapsed());
        }
    }
}

/// Test data generators
pub mod generators {
    use proptest::prelude::*;

    pub fn file_path_strategy() -> impl Strategy<Value = String> {
        "[a-z]+(/[a-z]+){0,3}\\.[a-z]{2,4}"
    }

    pub fn file_content_strategy() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 0..10000)
    }

    pub fn hash_strategy() -> impl Strategy<Value = String> {
        "[0-9a-f]{64}"
    }
}
