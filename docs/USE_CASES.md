# wupdedup-rs Use Cases and Operational Documentation

## Overview

This document describes the primary use cases, operational scenarios, and workflows for wupdedup-rs. Each use case includes context, actors, preconditions, steps, and expected outcomes.

## Primary Use Cases

### UC1: Personal Photo Library Deduplication

**Actor**: Home User  
**Goal**: Clean up duplicate photos across multiple devices and cloud services  
**Frequency**: Monthly/Quarterly

**Scenario**:
Sarah has been taking photos for years and backing them up to multiple services:
- Local hard drive backups
- Google Photos
- SmugMug galleries
- iCloud Photos
- External USB drives

Over time, she has accumulated thousands of duplicates due to:
- Multiple backup attempts
- Different photo editing apps saving copies
- Cloud service sync issues
- Manual organization attempts

**Workflow**:
```bash
# 1. Initial scan of all storage locations
wupdedup-rs scan --local /media/photos --local /backup/photos
wupdedup-rs scan --smugmug --google-photos

# 2. Analyze duplicates
wupdedup-rs dedupe --analyze --report duplicates.html

# 3. Review and clean up (interactive mode)
wupdedup-rs dedupe --interactive

# 4. Automated cleanup (after review)
wupdedup-rs dedupe --auto --keep-newest --archive-to /backup/duplicates
```

**Expected Outcome**:
- 30-50% storage space recovered
- Single source of truth for each photo
- Organized archive of removed duplicates
- Report showing cleanup statistics

### UC2: Professional Photography Workflow

**Actor**: Professional Photographer  
**Goal**: Manage RAW files and exports across multiple projects  
**Frequency**: After each shoot/project

**Scenario**:
Marcus is a wedding photographer who:
- Shoots 2000-5000 RAW images per event
- Creates multiple exports (web, print, client delivery)
- Maintains backups on local RAID and cloud storage
- Needs to track versions and edits

**Workflow**:
```bash
# 1. Import and scan new shoot
wupdedup-rs scan --local /shoots/2024-01-wedding-smith \
    --tags "event:wedding,client:smith,date:2024-01-15"

# 2. Identify similar shots (near-duplicates)
wupdedup-rs dedupe --perceptual --threshold 0.95 \
    --group-by similarity --output similar-shots.json

# 3. Track exports and versions
wupdedup-rs track --source /shoots/2024-01-wedding-smith/RAW \
    --derivatives /exports/smith-wedding/

# 4. Verify backup completeness
wupdedup-rs compare --local /shoots/2024-01-wedding-smith \
    --s3 s3://backup/shoots/2024-01-wedding-smith \
    --verify-integrity
```

**Expected Outcome**:
- Organized shot groups for easier culling
- Tracked relationship between RAW and exports
- Verified backup integrity
- Efficient storage usage

### UC3: Corporate Document Management

**Actor**: IT Administrator  
**Goal**: Manage document sprawl across departmental shares  
**Frequency**: Weekly/Monthly

**Scenario**:
TechCorp has:
- 50TB of shared documents
- Multiple versions of policies and procedures
- Duplicated training materials
- Redundant project files across teams

**Workflow**:
```bash
# 1. Scheduled scan of network shares
wupdedup-rs scan --smb //fileserver/shared \
    --exclude "*.tmp,~*" \
    --schedule "0 2 * * SUN"

# 2. Generate department reports
wupdedup-rs stats --group-by department \
    --output /reports/storage-usage.csv

# 3. Identify duplicate documents
wupdedup-rs dedupe --content-match \
    --min-size 100KB \
    --types "doc,docx,pdf,xlsx" \
    --output /reports/duplicates.csv

# 4. Compliance audit trail
wupdedup-rs audit --changes-since "2024-01-01" \
    --include-deleted \
    --output /compliance/audit-trail.json
```

**Expected Outcome**:
- 20-30% storage reduction
- Compliance audit reports
- Department storage metrics
- Automated cleanup recommendations

### UC4: Cloud Migration Preparation

**Actor**: Cloud Architect  
**Goal**: Prepare for migration from on-premises to cloud storage  
**Frequency**: One-time with periodic updates

**Scenario**:
Enterprise planning to migrate 100TB from on-premises to AWS S3:
- Need to identify what to migrate
- Remove duplicates before migration
- Estimate cloud storage costs
- Plan migration phases

**Workflow**:
```bash
# 1. Comprehensive scan and analysis
wupdedup-rs scan --local /data --recursive \
    --export metadata.db

# 2. Deduplication analysis
wupdedup-rs dedupe --analyze \
    --estimate-savings \
    --output migration-analysis.json

# 3. Classification and tiering
wupdedup-rs classify --by-age --by-access \
    --suggest-tier "hot,cool,archive" \
    --output tiering-plan.csv

# 4. Migration simulation
wupdedup-rs migrate --dry-run \
    --source /data \
    --dest s3://migration-bucket \
    --estimate-cost \
    --parallel 10
```

**Expected Outcome**:
- Cost savings estimate
- Tiered storage plan
- Migration timeline
- Cleaned dataset ready for migration

### UC5: Media Production Archive

**Actor**: Video Editor / Production House  
**Goal**: Manage project archives and footage library  
**Frequency**: Per project completion

**Scenario**:
Production company with:
- Hundreds of completed projects
- Terabytes of stock footage
- Multiple versions of edits
- Need for quick footage discovery

**Workflow**:
```bash
# 1. Archive completed project
wupdedup-rs archive --project "Commercial-2024-Q1" \
    --source /active/projects/commercial-q1 \
    --dest /archive/2024/Q1/ \
    --compress --verify

# 2. Index footage library
wupdedup-rs index --media /footage \
    --extract-metadata \
    --generate-thumbnails \
    --tags-from-path

# 3. Find similar footage
wupdedup-rs search --similar-to /reference/sunset.mp4 \
    --threshold 0.8 \
    --in /footage/stock

# 4. Project cleanup
wupdedup-rs cleanup --project "Commercial-2024-Q1" \
    --remove-temp \
    --remove-cache \
    --keep-masters
```

**Expected Outcome**:
- Organized project archives
- Searchable footage library
- Reduced storage costs
- Fast footage discovery

## Operational Scenarios

### Scenario 1: Disaster Recovery

**Situation**: Primary storage failure, need to verify backup completeness

```bash
# Verify backup integrity
wupdedup-rs verify --backup /mnt/backup \
    --compare-with metadata-backup.db \
    --check-integrity \
    --report missing-files.txt

# Restore from backup
wupdedup-rs restore --from /mnt/backup \
    --to /data/restored \
    --verify-after \
    --parallel 20
```

### Scenario 2: Storage Capacity Planning

**Situation**: Running out of storage, need to identify cleanup opportunities

```bash
# Analyze storage usage patterns
wupdedup-rs analyze --growth-trend \
    --period "6 months" \
    --predict "3 months" \
    --output capacity-report.html

# Identify candidates for deletion
wupdedup-rs suggest-cleanup \
    --older-than "2 years" \
    --not-accessed "1 year" \
    --duplicates \
    --large-files \
    --output cleanup-candidates.csv
```

### Scenario 3: Compliance Audit

**Situation**: Need to demonstrate data governance and retention compliance

```bash
# Generate compliance report
wupdedup-rs compliance \
    --retention-policy /policies/retention.yaml \
    --check-violations \
    --include-pii-scan \
    --output compliance-report.pdf

# Apply retention policies
wupdedup-rs apply-retention \
    --policy /policies/retention.yaml \
    --action "archive" \
    --dry-run
```

### Scenario 4: Performance Optimization

**Situation**: Slow file access, need to optimize storage layout

```bash
# Analyze access patterns
wupdedup-rs analyze-access \
    --period "30 days" \
    --identify-hot-files \
    --suggest-cache \
    --output access-patterns.json

# Optimize storage layout
wupdedup-rs optimize \
    --defragment \
    --rebalance \
    --move-hot-to-ssd \
    --schedule "weekend"
```

## Integration Workflows

### Git LFS Deduplication

```bash
# Scan Git LFS storage
wupdedup-rs scan --git-lfs /repos \
    --track-versions

# Optimize LFS storage
wupdedup-rs optimize-lfs \
    --dedupe-across-repos \
    --clean-old-versions \
    --keep-recent 5
```

### CI/CD Pipeline Integration

```yaml
# .gitlab-ci.yml example
dedupe-artifacts:
  stage: post-build
  script:
    - wupdedup-rs scan --local ./artifacts
    - wupdedup-rs dedupe --auto --in-place
    - wupdedup-rs report --format json > dedupe-report.json
  artifacts:
    reports:
      deduplication: dedupe-report.json
```

### Backup System Integration

```bash
# Pre-backup deduplication
wupdedup-rs dedupe --source /data \
    --before-backup \
    --link-duplicates

# Post-backup verification
wupdedup-rs verify --backup /mnt/backup \
    --compare-source /data \
    --check-dedup-consistency
```

## Monitoring and Alerting

### Prometheus Metrics

```yaml
# Exposed metrics
wupdedup_scan_duration_seconds
wupdedup_files_scanned_total
wupdedup_duplicates_found_total
wupdedup_storage_saved_bytes
wupdedup_errors_total
```

### Alert Rules

```yaml
- alert: HighDuplicationRate
  expr: rate(wupdedup_duplicates_found_total[1h]) > 100
  annotations:
    summary: "High duplication rate detected"

- alert: ScanFailure
  expr: increase(wupdedup_errors_total[1h]) > 5
  annotations:
    summary: "Multiple scan failures"
```

## Best Practices

### 1. Regular Scanning
- Schedule weekly incremental scans
- Monthly full scans
- Immediate scan after large imports

### 2. Staged Deduplication
- Always analyze before deletion
- Start with oldest duplicates
- Keep audit trail of deletions

### 3. Backup Before Cleanup
- Verify backups exist
- Test restore procedure
- Keep deletion archive for 30 days

### 4. Performance Tuning
```bash
# Optimal settings for large datasets
export WUPDEDUP_PARALLEL_WORKERS=8
export WUPDEDUP_CACHE_SIZE=1GB
export WUPDEDUP_BATCH_SIZE=1000

wupdedup-rs scan --parallel 8 \
    --cache-size 1GB \
    --batch-commits 1000
```

### 5. Security Considerations
- Run with minimum required permissions
- Encrypt sensitive metadata
- Audit all deletion operations
- Use read-only mode for analysis

## Troubleshooting Common Issues

### Issue: Slow Scanning
```bash
# Enable profiling
WUPDEDUP_PROFILE_ENABLED=true \
WUPDEDUP_PROFILE_MODE=cpu \
wupdedup-rs scan --local /data

# Analyze bottlenecks
wupdedup-rs profile --analyze profile.data
```

### Issue: High Memory Usage
```bash
# Limit memory usage
wupdedup-rs scan --memory-limit 4GB \
    --batch-size 100 \
    --no-cache
```

### Issue: Database Corruption
```bash
# Verify database integrity
wupdedup-rs db --verify

# Repair if needed
wupdedup-rs db --repair

# Rebuild from scan
wupdedup-rs db --rebuild --from-scan /data
```

## Command Quick Reference

### Essential Commands
```bash
# Scan locations
wupdedup-rs scan [--local PATH] [--s3 BUCKET] [--smugmug]

# Find duplicates
wupdedup-rs dedupe [--analyze] [--auto] [--interactive]

# Show statistics
wupdedup-rs stats [--detailed] [--group-by TYPE]

# Compare locations
wupdedup-rs compare --source PATH --dest PATH

# Verify integrity
wupdedup-rs verify [--checksum] [--deep]
```

### Advanced Operations
```bash
# Database management
wupdedup-rs db [--verify] [--repair] [--export] [--import]

# Profile operations
wupdedup-rs profile [--start] [--stop] [--analyze]

# Batch operations
wupdedup-rs batch --commands-from FILE

# API server mode
wupdedup-rs serve --port 8080 --api-key KEY
```

## Conclusion

wupdedup-rs provides comprehensive solutions for various storage management challenges, from personal photo organization to enterprise-scale deduplication. The flexible architecture and extensive CLI options enable users to customize workflows for their specific needs while maintaining data integrity and operational efficiency.