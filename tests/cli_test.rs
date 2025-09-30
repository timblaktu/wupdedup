use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cli_help() -> Result<()> {
    let mut cmd = Command::cargo_bin("wupdedup")?;
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "A modular multicloud storage management",
        ))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("scan"))
        .stdout(predicate::str::contains("dedupe"))
        .stdout(predicate::str::contains("stats"));

    Ok(())
}

#[test]
fn test_cli_version() -> Result<()> {
    let mut cmd = Command::cargo_bin("wupdedup")?;
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("wupdedup"));

    Ok(())
}

#[test]
fn test_scan_command() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_files");
    fs::create_dir_all(&test_dir)?;

    // Create test files
    fs::write(test_dir.join("file1.txt"), "content1")?;
    fs::write(test_dir.join("file2.txt"), "content2")?;

    // Create a test database
    let db_file = temp_dir.path().join("test.db");

    let mut cmd = Command::cargo_bin("wupdedup")?;
    cmd.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    Ok(())
}

#[test]
fn test_scan_empty_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create a test database
    let db_file = temp_dir.path().join("test.db");

    let mut cmd = Command::cargo_bin("wupdedup")?;
    cmd.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success();

    Ok(())
}

#[test]
fn test_dedupe_command() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_files");
    fs::create_dir_all(&test_dir)?;

    // Create duplicate files
    fs::write(test_dir.join("dup1.txt"), "duplicate content")?;
    fs::write(test_dir.join("dup2.txt"), "duplicate content")?;
    fs::write(test_dir.join("unique.txt"), "unique content")?;

    // Create a test database
    let db_file = temp_dir.path().join("test.db");

    // First scan the files
    let mut scan_cmd = Command::cargo_bin("wupdedup")?;
    scan_cmd
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    // Then run dedupe
    let mut dedupe_cmd = Command::cargo_bin("wupdedup")?;
    dedupe_cmd
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("dedupe")
        .arg("--show-only")
        .assert()
        .success()
        .stdout(predicate::str::contains("Duplicate Files Report"))
        .stdout(predicate::str::contains("Files (2)"));

    Ok(())
}

#[test]
fn test_stats_command() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_files");
    fs::create_dir_all(&test_dir)?;

    // Create test files
    fs::write(test_dir.join("file1.txt"), "content1")?;
    fs::write(test_dir.join("file2.txt"), "content2")?;
    fs::write(test_dir.join("file3.txt"), "content3")?;

    let db_file = temp_dir.path().join("test.db");

    // First scan the files
    let mut scan_cmd = Command::cargo_bin("wupdedup")?;
    scan_cmd
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    // Then check stats
    let mut stats_cmd = Command::cargo_bin("wupdedup")?;
    stats_cmd
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("stats")
        .assert()
        .success()
        .stdout(predicate::str::contains("local storage: 3 files indexed"));

    Ok(())
}

#[test]
fn test_scan_with_subdirectories() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_files");
    let sub_dir = test_dir.join("subdir");
    fs::create_dir_all(&sub_dir)?;

    // Create files in root and subdirectory
    fs::write(test_dir.join("root.txt"), "root content")?;
    fs::write(sub_dir.join("sub.txt"), "sub content")?;

    // Create a test database
    let db_file = temp_dir.path().join("test.db");

    let mut cmd = Command::cargo_bin("wupdedup")?;
    cmd.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    Ok(())
}

#[test]
fn test_multiple_scans_are_idempotent() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_files");
    fs::create_dir_all(&test_dir)?;

    fs::write(test_dir.join("file1.txt"), "content1")?;
    fs::write(test_dir.join("file2.txt"), "content2")?;

    let db_file = temp_dir.path().join("test.db");

    // First scan
    let mut scan1 = Command::cargo_bin("wupdedup")?;
    scan1
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    // Second scan (should update, not duplicate)
    let mut scan2 = Command::cargo_bin("wupdedup")?;
    scan2
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    // Check stats - should still be 2 files
    let mut stats = Command::cargo_bin("wupdedup")?;
    stats
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("stats")
        .assert()
        .success()
        .stdout(predicate::str::contains("local storage: 2 files indexed"));

    Ok(())
}

#[test]
fn test_log_level_argument() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Create a test database
    let db_file = temp_dir.path().join("test.db");

    let mut cmd = Command::cargo_bin("wupdedup")?;
    cmd.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("--log-level")
        .arg("debug")
        .arg("scan")
        .arg("--local")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success();

    Ok(())
}

#[test]
fn test_invalid_directory_error() -> Result<()> {
    let mut cmd = Command::cargo_bin("wupdedup")?;
    cmd.arg("scan")
        .arg("--local")
        .arg("/nonexistent/directory/path")
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not exist"));

    Ok(())
}

#[test]
fn test_dedupe_finds_identical_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_files");
    fs::create_dir_all(&test_dir)?;

    // Create three identical files
    let content = "identical content for testing";
    fs::write(test_dir.join("copy1.txt"), content)?;
    fs::write(test_dir.join("copy2.txt"), content)?;
    fs::write(test_dir.join("copy3.txt"), content)?;

    let db_file = temp_dir.path().join("test.db");

    // Scan
    let mut scan = Command::cargo_bin("wupdedup")?;
    scan.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    // Check dedupe finds all three
    let mut dedupe = Command::cargo_bin("wupdedup")?;
    dedupe
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("dedupe")
        .arg("--show-only")
        .assert()
        .success()
        .stdout(predicate::str::contains("Files (3)"))
        .stdout(predicate::str::contains("copy1.txt"))
        .stdout(predicate::str::contains("copy2.txt"))
        .stdout(predicate::str::contains("copy3.txt"));

    Ok(())
}

#[test]
fn test_dedupe_delete_strategy() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_files");
    fs::create_dir_all(&test_dir)?;

    // Create duplicate files (file1 will be kept as it sorts first alphabetically)
    let content = "duplicate content for deletion test";
    let file1 = test_dir.join("file1.txt");
    let file2 = test_dir.join("file2.txt");
    let file3 = test_dir.join("file3.txt");

    fs::write(&file1, content)?;
    fs::write(&file2, content)?;
    fs::write(&file3, content)?;

    let db_file = temp_dir.path().join("test.db");

    // Scan
    let mut scan = Command::cargo_bin("wupdedup")?;
    scan.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    // Dedupe with delete strategy (dry run first)
    let mut dedupe_dry = Command::cargo_bin("wupdedup")?;
    dedupe_dry
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("dedupe")
        .arg("--strategy")
        .arg("delete")
        .arg("--dry-run")
        .arg("--auto")
        .assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN MODE"));

    // Verify all files still exist after dry run
    assert!(file1.exists());
    assert!(file2.exists());
    assert!(file3.exists());

    // Dedupe with delete strategy (actual)
    let mut dedupe = Command::cargo_bin("wupdedup")?;
    dedupe
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("dedupe")
        .arg("--strategy")
        .arg("delete")
        .arg("--auto")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deduplication Complete"));

    // Verify first file remains, others deleted
    assert!(file1.exists(), "Original file should remain");
    assert!(!file2.exists(), "Duplicate 1 should be deleted");
    assert!(!file3.exists(), "Duplicate 2 should be deleted");

    Ok(())
}

#[test]
fn test_dedupe_move_strategy() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_files");
    let target_dir = temp_dir.path().join("duplicates");
    fs::create_dir_all(&test_dir)?;

    // Create duplicate files (file1 will be kept as it sorts first alphabetically)
    let content = "duplicate content for move test";
    let file1 = test_dir.join("file1.txt");
    let file2 = test_dir.join("file2.txt");

    fs::write(&file1, content)?;
    fs::write(&file2, content)?;

    let db_file = temp_dir.path().join("test.db");

    // Scan
    let mut scan = Command::cargo_bin("wupdedup")?;
    scan.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    // Dedupe with move strategy
    let mut dedupe = Command::cargo_bin("wupdedup")?;
    dedupe
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("dedupe")
        .arg("--strategy")
        .arg("move")
        .arg("--target-dir")
        .arg(target_dir.to_str().unwrap())
        .arg("--auto")
        .assert()
        .success();

    // Verify first file remains, second is moved
    assert!(file1.exists(), "Original file should remain");
    assert!(
        !file2.exists(),
        "Duplicate should be moved from original location"
    );
    assert!(
        target_dir.join("file2.txt").exists(),
        "Duplicate should be in target directory"
    );

    Ok(())
}

#[test]
fn test_dedupe_archive_strategy() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_files");
    let archive_dir = temp_dir.path().join("archive");
    fs::create_dir_all(&test_dir)?;

    // Create duplicate files in subdirectory (file1 will be kept as it sorts first)
    let subdir = test_dir.join("subdir");
    fs::create_dir_all(&subdir)?;

    let content = "duplicate content for archive test";
    let file1 = subdir.join("file1.dat");
    let file2 = subdir.join("file2.dat");

    fs::write(&file1, content)?;
    fs::write(&file2, content)?;

    let db_file = temp_dir.path().join("test.db");

    // Scan
    let mut scan = Command::cargo_bin("wupdedup")?;
    scan.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    // Dedupe with archive strategy
    let mut dedupe = Command::cargo_bin("wupdedup")?;
    dedupe
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("dedupe")
        .arg("--strategy")
        .arg("archive")
        .arg("--target-dir")
        .arg(archive_dir.to_str().unwrap())
        .arg("--auto")
        .assert()
        .success();

    // Verify first file remains, second is archived with preserved path structure
    assert!(file1.exists(), "Original file should remain");
    assert!(
        !file2.exists(),
        "Duplicate should be archived from original location"
    );

    Ok(())
}

#[test]
fn test_dedupe_no_duplicates() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let test_dir = temp_dir.path().join("test_files");
    fs::create_dir_all(&test_dir)?;

    // Create unique files
    fs::write(test_dir.join("unique1.txt"), "unique content 1")?;
    fs::write(test_dir.join("unique2.txt"), "unique content 2")?;
    fs::write(test_dir.join("unique3.txt"), "unique content 3")?;

    let db_file = temp_dir.path().join("test.db");

    // Scan
    let mut scan = Command::cargo_bin("wupdedup")?;
    scan.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();

    // Check dedupe finds no duplicates
    let mut dedupe = Command::cargo_bin("wupdedup")?;
    dedupe
        .arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("dedupe")
        .arg("--show-only")
        .assert()
        .success()
        .stdout(predicate::str::contains("No duplicate files found"));

    Ok(())
}
