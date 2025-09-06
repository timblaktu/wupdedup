use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_cli_help() -> Result<()> {
    let mut cmd = Command::cargo_bin("wupdedup-rs")?;
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("A modular multicloud storage management"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("scan"))
        .stdout(predicate::str::contains("dedupe"))
        .stdout(predicate::str::contains("stats"));
    
    Ok(())
}

#[test]
fn test_cli_version() -> Result<()> {
    let mut cmd = Command::cargo_bin("wupdedup-rs")?;
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("wupdedup-rs"));
    
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
    
    let mut cmd = Command::cargo_bin("wupdedup-rs")?;
    cmd.arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success()
        .success();
    
    Ok(())
}

#[test]
fn test_scan_empty_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    
    let mut cmd = Command::cargo_bin("wupdedup-rs")?;
    cmd.arg("scan")
        .arg("--local")
        .arg(temp_dir.path().to_str().unwrap())
        .assert()
        .success()
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
    let mut scan_cmd = Command::cargo_bin("wupdedup-rs")?;
    scan_cmd.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();
    
    // Then run dedupe
    let mut dedupe_cmd = Command::cargo_bin("wupdedup-rs")?;
    dedupe_cmd.arg("--db-file")
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
    let mut scan_cmd = Command::cargo_bin("wupdedup-rs")?;
    scan_cmd.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();
    
    // Then check stats
    let mut stats_cmd = Command::cargo_bin("wupdedup-rs")?;
    stats_cmd.arg("--db-file")
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
    
    let mut cmd = Command::cargo_bin("wupdedup-rs")?;
    cmd.arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success()
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
    let mut scan1 = Command::cargo_bin("wupdedup-rs")?;
    scan1.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();
    
    // Second scan (should update, not duplicate)
    let mut scan2 = Command::cargo_bin("wupdedup-rs")?;
    scan2.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();
    
    // Check stats - should still be 2 files
    let mut stats = Command::cargo_bin("wupdedup-rs")?;
    stats.arg("--db-file")
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
    
    let mut cmd = Command::cargo_bin("wupdedup-rs")?;
    cmd.arg("--log-level")
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
    let mut cmd = Command::cargo_bin("wupdedup-rs")?;
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
    let mut scan = Command::cargo_bin("wupdedup-rs")?;
    scan.arg("--db-file")
        .arg(db_file.to_str().unwrap())
        .arg("scan")
        .arg("--local")
        .arg(test_dir.to_str().unwrap())
        .assert()
        .success();
    
    // Check dedupe finds all three
    let mut dedupe = Command::cargo_bin("wupdedup-rs")?;
    dedupe.arg("--db-file")
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