# wupdedup-rs User Guide

## Table of Contents
1. [Getting Started](#getting-started)
2. [Installation](#installation)
3. [Basic Usage](#basic-usage)
4. [Common Workflows](#common-workflows)
5. [Advanced Features](#advanced-features)
6. [Best Practices](#best-practices)
7. [Troubleshooting](#troubleshooting)
8. [FAQ](#faq)

## Getting Started

wupdedup-rs is a high-performance file deduplication and storage management tool that helps you:
- Find and remove duplicate files across local and cloud storage
- Analyze storage usage patterns
- Organize scattered digital content
- Verify backup integrity
- Optimize storage costs

### Quick Start

```bash
# Install (see Installation section for details)
cargo install wupdedup-rs

# Scan your home directory
wupdedup-rs scan --local ~

# Find duplicates
wupdedup-rs dedupe --analyze

# Show statistics
wupdedup-rs stats
```

## Installation

### From Source

```bash
# Clone repository
git clone https://github.com/yourusername/wupdedup-rs
cd wupdedup-rs

# Build and install
cargo build --release
cargo install --path .

# Or run directly
cargo run -- scan --local /path/to/scan
```

### Using Cargo

```bash
cargo install wupdedup-rs
```

### Binary Releases

Download pre-built binaries from the releases page:
- Linux: `wupdedup-rs-linux-x64`
- macOS: `wupdedup-rs-macos-x64`
- Windows: `wupdedup-rs-windows-x64.exe`

### Docker

```bash
docker pull wupdedup/wupdedup-rs
docker run -v /path/to/scan:/data wupdedup/wupdedup-rs scan --local /data
```

## Basic Usage

### Configuration

Create a `config.toml` file:

```toml
log_level = "info"
db_file = "~/.wupdedup/wupdedup.db"

[local]
root_path = "/home/username"
exclude_patterns = [".git", "node_modules", "*.tmp"]
```

Or use environment variables:

```bash
export WUPDEDUP_LOG_LEVEL=debug
export WUPDEDUP_LOCAL_ROOT_PATH=/home/username
```

### Scanning Files

#### Scan Local Directory

```bash
# Basic scan
wupdedup-rs scan --local /photos

# Scan with exclusions
wupdedup-rs scan --local /home \
  --exclude "*.tmp" \
  --exclude ".cache" \
  --exclude "node_modules"

# Scan without following symlinks
wupdedup-rs scan --local /data --no-follow-links

# Scan specific file types
wupdedup-rs scan --local /documents --include "*.pdf,*.doc,*.docx"
```

#### Scan Cloud Storage

```bash
# SmugMug
export WUPDEDUP_SMUGMUG_API_KEY=your_key
export WUPDEDUP_SMUGMUG_API_SECRET=your_secret
wupdedup-rs scan --smugmug

# S3
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
wupdedup-rs scan --s3 my-bucket

# Multiple locations
wupdedup-rs scan --local /photos --smugmug --s3 backup-bucket
```

### Finding Duplicates

#### Basic Deduplication

```bash
# Analyze duplicates (no changes)
wupdedup-rs dedupe --analyze

# Show duplicate report
wupdedup-rs dedupe --analyze --report duplicates.html
open duplicates.html

# Interactive mode
wupdedup-rs dedupe --interactive
```

#### Automatic Deduplication

```bash
# Delete duplicates, keep newest
wupdedup-rs dedupe --auto \
  --strategy keep-newest \
  --action delete

# Archive duplicates
wupdedup-rs dedupe --auto \
  --strategy keep-original \
  --action archive \
  --archive-to /backup/duplicates

# Create hard links (save space, keep all paths)
wupdedup-rs dedupe --auto \
  --strategy keep-oldest \
  --action hardlink

# Dry run (preview changes)
wupdedup-rs dedupe --auto \
  --strategy keep-largest \
  --action delete \
  --dry-run
```

### Viewing Statistics

```bash
# Basic stats
wupdedup-rs stats

# Detailed statistics
wupdedup-rs stats --detailed

# Group by file type
wupdedup-rs stats --group-by type

# Export to CSV
wupdedup-rs stats --format csv --output storage-report.csv

# HTML report
wupdedup-rs stats --format html --output report.html
```

## Common Workflows

### Workflow 1: Clean Up Photos Collection

```bash
# Step 1: Scan all photo locations
wupdedup-rs scan \
  --local /photos \
  --local /backup/photos \
  --local /external-drive/photos

# Step 2: Analyze duplicates
wupdedup-rs dedupe --analyze --report photo-duplicates.html

# Step 3: Review report
open photo-duplicates.html

# Step 4: Clean up duplicates interactively
wupdedup-rs dedupe --interactive

# Or automatically
wupdedup-rs dedupe --auto \
  --strategy keep-oldest \
  --action archive \
  --archive-to /backup/photo-duplicates \
  --types "jpg,jpeg,png,raw,dng"
```

### Workflow 2: Verify Backup Integrity

```bash
# Step 1: Scan source and backup
wupdedup-rs scan --local /important-data
wupdedup-rs scan --local /backup/important-data

# Step 2: Compare locations
wupdedup-rs compare \
  --source /important-data \
  --dest /backup/important-data \
  --verify-integrity

# Step 3: Show missing files
wupdedup-rs compare \
  --source /important-data \
  --dest /backup/important-data \
  --show-missing > missing-files.txt

# Step 4: Sync missing files
wupdedup-rs compare \
  --source /important-data \
  --dest /backup/important-data \
  --sync --sync-direction to-dest
```

### Workflow 3: Organize Downloads Folder

```bash
# Step 1: Scan downloads
wupdedup-rs scan --local ~/Downloads

# Step 2: Find duplicates
wupdedup-rs dedupe --analyze

# Step 3: Clean up old duplicates
wupdedup-rs dedupe --auto \
  --strategy keep-newest \
  --action delete \
  --min-size 1MB  # Ignore small files

# Step 4: Show space recovered
wupdedup-rs stats --since today
```

### Workflow 4: Cloud Storage Migration

```bash
# Step 1: Scan current storage
wupdedup-rs scan --local /data

# Step 2: Remove duplicates before migration
wupdedup-rs dedupe --auto \
  --strategy keep-newest \
  --action delete

# Step 3: Estimate storage needs
wupdedup-rs stats --detailed --format json > storage-analysis.json

# Step 4: Perform migration (example with rclone)
rclone copy /data s3:new-bucket --progress

# Step 5: Verify migration
wupdedup-rs scan --s3 new-bucket
wupdedup-rs compare --source /data --dest s3://new-bucket
```

### Workflow 5: Regular Maintenance

Create a script for regular maintenance:

```bash
#!/bin/bash
# maintenance.sh

# Weekly maintenance script
LOG_FILE="/var/log/wupdedup-maintenance.log"
REPORT_DIR="/home/user/reports"

echo "Starting maintenance - $(date)" >> $LOG_FILE

# Scan for new files
wupdedup-rs scan --local /home/user \
  --exclude ".cache" >> $LOG_FILE 2>&1

# Generate weekly report
wupdedup-rs stats --format html \
  --output "$REPORT_DIR/weekly-$(date +%Y%m%d).html"

# Find and report duplicates
wupdedup-rs dedupe --analyze \
  --report "$REPORT_DIR/duplicates-$(date +%Y%m%d).html"

# Clean up old downloads
wupdedup-rs dedupe --auto \
  --strategy keep-newest \
  --action delete \
  --min-size 10MB \
  --types "zip,tar,gz,dmg,exe,deb,rpm" \
  --dry-run >> $LOG_FILE 2>&1

echo "Maintenance completed - $(date)" >> $LOG_FILE
```

Schedule with cron:
```bash
# Run every Sunday at 2 AM
0 2 * * 0 /home/user/scripts/maintenance.sh
```

## Advanced Features

### Perceptual Hashing for Images

Find visually similar images:

```bash
# Find near-duplicate images
wupdedup-rs dedupe \
  --perceptual \
  --threshold 0.95 \
  --types "jpg,jpeg,png" \
  --analyze

# Group similar photos
wupdedup-rs dedupe \
  --perceptual \
  --threshold 0.85 \
  --group-by similarity \
  --output similar-photos.json
```

### Content-Based Deduplication

```bash
# Verify duplicates by content
wupdedup-rs dedupe \
  --content-match \
  --verify \
  --analyze

# Find duplicates ignoring metadata
wupdedup-rs dedupe \
  --ignore-metadata \
  --content-only
```

### Custom Filters

```bash
# Complex filtering
wupdedup-rs scan --local /data \
  --include "*.doc,*.pdf" \
  --exclude "temp*" \
  --min-size 1KB \
  --max-size 100MB \
  --modified-after "2023-01-01" \
  --modified-before "2024-01-01"
```

### Parallel Processing

```bash
# Use all CPU cores
wupdedup-rs scan --local /large-dataset --parallel 0

# Limit to 4 workers
wupdedup-rs scan --local /large-dataset --parallel 4

# Optimize for SSDs
wupdedup-rs scan --local /ssd-drive \
  --parallel 16 \
  --batch-size 5000
```

### Database Management

```bash
# Verify database integrity
wupdedup-rs db verify

# Repair corrupted database
wupdedup-rs db repair

# Export database
wupdedup-rs db export backup-$(date +%Y%m%d).db

# Import database
wupdedup-rs db import backup-20240101.db

# Compact database
wupdedup-rs db compact

# Show database statistics
wupdedup-rs db stats
```

### Profiling and Performance

```bash
# Enable profiling
export WUPDEDUP_PROFILE_ENABLED=true
export WUPDEDUP_PROFILE_MODE=cpu

# Run with profiling
wupdedup-rs scan --local /large-dataset

# Analyze profile
wupdedup-rs profile --analyze profile.data

# Memory profiling
export WUPDEDUP_PROFILE_MODE=memory
wupdedup-rs scan --local /large-dataset --memory-limit 2GB
```

## Best Practices

### 1. Start Small

Begin with a small directory to understand the tool:
```bash
wupdedup-rs scan --local ~/Documents
wupdedup-rs dedupe --analyze
```

### 2. Always Analyze First

Never run automatic deduplication without analysis:
```bash
# Good practice
wupdedup-rs dedupe --analyze --report report.html
# Review report
wupdedup-rs dedupe --auto --strategy keep-newest

# Bad practice
wupdedup-rs dedupe --auto --action delete  # Dangerous!
```

### 3. Use Dry Run

Test actions before applying:
```bash
wupdedup-rs dedupe --auto \
  --strategy keep-newest \
  --action delete \
  --dry-run
```

### 4. Regular Backups

Before major operations:
```bash
# Backup database
wupdedup-rs db export pre-cleanup-$(date +%Y%m%d).db

# Backup files
rsync -av /important-data /backup/
```

### 5. Incremental Scanning

For large datasets, scan incrementally:
```bash
# Initial full scan
wupdedup-rs scan --local /data

# Later, scan only new/modified files
wupdedup-rs scan --local /data --incremental
```

### 6. Use Appropriate Strategies

Choose the right deduplication strategy:
- `keep-newest`: For downloads and temporary files
- `keep-oldest`: For photos and original documents
- `keep-largest`: For quality media files
- `keep-original`: For organized vs. backup locations

### 7. Monitor Performance

For large operations:
```bash
# Monitor progress
wupdedup-rs scan --local /huge-dataset --verbose

# Limit resource usage
wupdedup-rs scan --local /huge-dataset \
  --memory-limit 4GB \
  --parallel 4
```

## Troubleshooting

### Issue: "Database is locked"

**Solution:**
```bash
# Check for running instances
ps aux | grep wupdedup-rs

# Kill stuck process
kill -9 <PID>

# Repair database
wupdedup-rs db repair
```

### Issue: "Out of memory"

**Solution:**
```bash
# Reduce batch size
wupdedup-rs scan --local /data --batch-size 100

# Limit memory usage
wupdedup-rs scan --local /data --memory-limit 2GB

# Reduce parallel workers
wupdedup-rs scan --local /data --parallel 2
```

### Issue: "Slow scanning"

**Solution:**
```bash
# Enable profiling
export WUPDEDUP_PROFILE_ENABLED=true
wupdedup-rs scan --local /data

# Check for antivirus interference
# Exclude wupdedup-rs from real-time scanning

# Use faster hash algorithm (less secure)
wupdedup-rs scan --local /data --hash-algo xxhash
```

### Issue: "Permission denied"

**Solution:**
```bash
# Run with sudo (Linux/macOS)
sudo wupdedup-rs scan --local /protected-directory

# Or fix permissions
sudo chown -R $USER:$USER /data
chmod -R u+r /data
```

### Issue: "Network timeout" (Cloud storage)

**Solution:**
```bash
# Increase timeout
export WUPDEDUP_NETWORK_TIMEOUT=120

# Reduce concurrent connections
export WUPDEDUP_CONNECTION_POOL_SIZE=10

# Add retry logic
export WUPDEDUP_RETRY_ATTEMPTS=5
```

## FAQ

### Q: How does wupdedup-rs determine duplicates?

A: By default, files are considered duplicates if they have identical Blake3 hashes. You can also use:
- Content comparison for verification
- Perceptual hashing for similar images
- Metadata matching for quick checks

### Q: Is it safe to delete duplicates automatically?

A: While the tool is designed to be safe, always:
1. Run analysis first
2. Use dry-run mode
3. Keep backups
4. Start with archiving instead of deletion

### Q: Can I undo deletions?

A: Not directly, but you can:
- Use `--action archive` to move files instead
- Keep database backups to track what was deleted
- Use filesystem snapshots if available

### Q: How fast is scanning?

A: Performance depends on:
- Storage type (SSD > HDD > Network)
- File sizes (many small files are slower)
- CPU cores (for parallel processing)

Typical speeds:
- Local SSD: 1-2 GB/s
- Local HDD: 100-200 MB/s
- Network: 10-100 MB/s

### Q: How much memory does it use?

A: Memory usage is proportional to:
- Batch size (default: 1000 files)
- Cache size (default: 100MB)
- Parallel workers

You can limit memory:
```bash
wupdedup-rs scan --memory-limit 1GB
```

### Q: Can I scan network drives?

A: Yes, mount them first:
```bash
# Linux/macOS
mount -t smbfs //server/share /mnt/share
wupdedup-rs scan --local /mnt/share

# Windows
net use Z: \\server\share
wupdedup-rs scan --local Z:\
```

### Q: Does it work with encrypted files?

A: Yes, if they're decrypted at the filesystem level. It cannot deduplicate within encrypted archives.

### Q: Can I use it on mobile devices?

A: Yes, on Android via Termux:
```bash
pkg install rust
cargo install wupdedup-rs
```

### Q: How do I contribute?

A: Contributions welcome!
1. Fork the repository
2. Create a feature branch
3. Add tests
4. Submit a pull request

### Q: Where can I get help?

A: 
- GitHub Issues: Report bugs and request features
- Discussions: Ask questions and share workflows
- Documentation: Check docs/ directory
- Examples: See examples/ directory

## Appendix

### Exit Codes

- `0`: Success
- `1`: General error
- `2`: Configuration error
- `3`: Database error
- `4`: Storage access error
- `5`: Network error

### File Patterns

Glob patterns supported:
- `*` - Match any characters
- `?` - Match single character
- `[abc]` - Match any of a, b, c
- `[!abc]` - Match anything except a, b, c
- `**` - Match directories recursively

Examples:
- `*.jpg` - All JPEG files
- `DSC*.raw` - RAW files from camera
- `**/*.tmp` - All temp files recursively
- `backup-[0-9]*` - Numbered backups

### Performance Tuning

For different scenarios:

**Many small files:**
```toml
[performance]
parallel_workers = 16
batch_size = 10000
cache_size = "100MB"
```

**Large media files:**
```toml
[performance]
parallel_workers = 4
batch_size = 100
cache_size = "1GB"
buffer_size = "64MB"
```

**Network storage:**
```toml
[performance]
parallel_workers = 8
batch_size = 500
cache_size = "500MB"
[network]
connection_pool_size = 20
timeout_seconds = 60
```

**Limited resources:**
```toml
[performance]
parallel_workers = 2
batch_size = 100
cache_size = "50MB"
memory_limit = "500MB"
```

### Security Notes

- Credentials are never stored in the database
- Use environment variables for sensitive data
- Database can be encrypted at filesystem level
- Network traffic uses HTTPS/TLS
- No data is sent to external services

### Compatibility

- **Operating Systems**: Linux, macOS, Windows, BSD
- **Architectures**: x86_64, ARM64, ARM32
- **Rust Version**: 1.75+
- **Database Format**: Platform-independent
- **Config Format**: TOML (compatible across versions)