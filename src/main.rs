mod config;
mod content;
mod db;
mod dedupe;
mod logging;
mod profiler;
mod storage;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use storage::local::FileInfo;
use tracing::{debug, error, info, warn};

#[derive(Parser, Debug)]
#[command(name = "wupdedup-rs")]
#[command(author = "Tim Blaktu")]
#[command(version = "0.1.0")]
#[command(about = "A modular multicloud storage management and deduplication tool", long_about = None)]
struct Cli {
    #[arg(short, long, env = "WUPDEDUP_LOG_LEVEL", default_value = "info")]
    log_level: String,

    #[arg(short, long, env = "WUPDEDUP_DB_FILE", default_value = "wupdedup.db")]
    db_file: String,

    #[arg(long, env = "WUPDEDUP_PROFILE")]
    profile: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Scan storage locations for files
    Scan {
        /// Scan local filesystem
        #[arg(long)]
        local: Option<String>,

        /// Scan SmugMug (requires API credentials)
        #[arg(long)]
        smugmug: bool,
    },

    /// Find and optionally remove duplicate files
    Dedupe {
        /// Show duplicates only (no action taken)
        #[arg(long)]
        show_only: bool,

        /// Deduplication strategy: delete, move, symlink, or archive
        #[arg(long, value_enum)]
        strategy: Option<Strategy>,

        /// Target directory for move/archive strategies
        #[arg(long)]
        target_dir: Option<String>,

        /// Automatically confirm all actions
        #[arg(long)]
        auto: bool,

        /// Perform a dry run (preview changes without applying)
        #[arg(long)]
        dry_run: bool,
    },

    /// Show statistics about stored files
    Stats,
}

#[derive(Debug, Clone, ValueEnum)]
enum Strategy {
    /// Delete duplicate files (keep the first occurrence)
    Delete,
    /// Move duplicates to a target directory
    Move,
    /// Replace duplicates with symlinks (Unix only)
    Symlink,
    /// Archive duplicates preserving directory structure
    Archive,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging
    logging::init(&cli.log_level)?;
    debug!("Starting wupdedup-rs");

    // Load configuration
    let mut config = config::Config::load()?;

    // Override config with CLI arguments
    config.log_level = cli.log_level;
    config.db_file = cli.db_file;
    config.profile.enabled = cli.profile;

    // Handle CLI commands
    if let Some(command) = cli.command {
        match command {
            Commands::Scan { local, smugmug } => {
                // Override config with command-specific options
                if let Some(path) = local {
                    config.local = Some(config::LocalConfig {
                        root_path: path.into(),
                    });
                }

                if smugmug && config.smugmug.is_none() {
                    error!("SmugMug credentials not configured");
                    return Ok(());
                }

                run_scan(&config).await?;
            }
            Commands::Dedupe {
                show_only,
                strategy,
                target_dir,
                auto,
                dry_run,
            } => {
                run_dedupe(&config, show_only, strategy, target_dir, auto, dry_run).await?;
            }
            Commands::Stats => {
                show_stats(&config).await?;
            }
        }
    } else {
        // Default behavior: run scan with configured storage strategies
        run_scan(&config).await?;
    }

    debug!("wupdedup-rs exiting");
    Ok(())
}

async fn run_scan(config: &config::Config) -> Result<()> {
    info!("Starting scan operation");

    // Initialize profiler if enabled
    let mut profiler = profiler::Profiler::new(config.profile.clone());
    profiler.start()?;

    // Load storage strategy contexts
    let mut contexts = storage::load_storage_strategy_contexts(config)?;

    // Initialize database
    let db = db::DB::init(&config.db_file)?;

    // Set up buckets for each context and run scans
    for context in &mut contexts {
        let bucket = db.bucket(&context.name)?;
        context.set_bucket(bucket);

        info!("Scanning {} storage", context.name);
        context.scan_tree().await?;

        info!(
            "Scan complete for {}: {} files, {} nodes total",
            context.name, context.file_count, context.node_count
        );
    }

    // Stop profiler
    profiler.stop()?;

    // Close database
    db.close()?;

    info!("All scans complete");
    Ok(())
}

async fn run_dedupe(
    config: &config::Config,
    show_only: bool,
    strategy: Option<Strategy>,
    target_dir: Option<String>,
    auto: bool,
    dry_run: bool,
) -> Result<()> {
    info!("Starting deduplication analysis");

    // Initialize database
    let db = db::DB::init(&config.db_file)?;

    // Get all buckets from the database directly
    // For now, we'll check known bucket names (local, smugmug)
    let bucket_names = vec!["local", "smugmug"];

    let mut total_duplicate_groups = 0;
    let mut total_duplicate_files = 0;
    let mut total_space_wasted: u64 = 0;
    let mut any_duplicates_found = false;

    println!("\n=== Duplicate Files Report ===");

    for bucket_name in &bucket_names {
        let bucket = db.bucket(bucket_name)?;

        // Skip if bucket has no entries
        if bucket.count()? == 0 {
            continue;
        }

        // Use the indexed find_duplicates method
        let duplicates = bucket.find_duplicates()?;

        if duplicates.is_empty() {
            continue;
        }

        any_duplicates_found = true;
        println!("\n--- Storage: {} ---", bucket_name);

        for (hash, files) in &duplicates {
            total_duplicate_groups += 1;
            total_duplicate_files += files.len() - 1; // Count extras only

            println!("\nHash: {}", &hash[..16.min(hash.len())]); // Show first 16 chars of hash
            println!("Files ({}):", files.len());

            for (idx, file_key) in files.iter().enumerate() {
                println!("  {}", file_key);

                // Get file size for space calculation
                if idx > 0 {
                    // Count all but the first as wasted space
                    if let Ok(Some(value)) = bucket.get(file_key) {
                        if let Ok(file_info) = serde_json::from_slice::<serde_json::Value>(&value) {
                            if let Some(size) = file_info.get("size").and_then(|s| s.as_u64()) {
                                total_space_wasted += size;
                            }
                        }
                    }
                }
            }
        }
    }

    if !any_duplicates_found {
        println!("\nNo duplicate files found.");
    }

    println!("\n=== Summary ===");
    println!("Total duplicate groups: {}", total_duplicate_groups);
    println!("Total duplicate files:  {}", total_duplicate_files);
    println!(
        "Space wasted:          {} MB",
        total_space_wasted / (1024 * 1024)
    );

    // Apply deduplication if requested
    if !show_only && total_duplicate_groups > 0 {
        if let Some(strat) = strategy {
            // Convert CLI strategy to dedupe module strategy
            let dedupe_strategy = match strat {
                Strategy::Delete => dedupe::DedupeStrategy::Delete,
                Strategy::Move => {
                    let dir = target_dir.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("--target-dir required for move strategy")
                    })?;
                    dedupe::DedupeStrategy::Move(PathBuf::from(dir))
                }
                Strategy::Symlink => dedupe::DedupeStrategy::Symlink,
                Strategy::Archive => {
                    let dir = target_dir.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("--target-dir required for archive strategy")
                    })?;
                    dedupe::DedupeStrategy::Archive(PathBuf::from(dir))
                }
            };

            let engine = dedupe::DedupeEngine::new(dedupe_strategy, dry_run, auto);

            println!("\n=== Applying Deduplication ===");
            if dry_run {
                println!("[DRY RUN MODE - No files will be modified]");
            }

            let mut total_result = dedupe::DedupeResult::default();

            // Process duplicates from each bucket
            for bucket_name in &bucket_names {
                let bucket = db.bucket(bucket_name)?;

                if bucket.count()? == 0 {
                    continue;
                }

                let duplicates = bucket.find_duplicates()?;

                for (hash, file_keys) in duplicates {
                    // Get the actual file data from the bucket and extract paths
                    let mut paths: Vec<PathBuf> = Vec::new();

                    for key in &file_keys {
                        // Get the file data from the bucket
                        if let Ok(Some(data)) = bucket.get(key) {
                            // Deserialize FileInfo to get the actual path
                            match serde_json::from_slice::<FileInfo>(&data) {
                                Ok(file_info) => {
                                    // Only process local files that exist
                                    if file_info.path.exists() {
                                        paths.push(file_info.path);
                                    } else {
                                        warn!("File no longer exists: {:?}", file_info.path);
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to deserialize file info for key '{}': {}",
                                        key, e
                                    );
                                }
                            }
                        }
                    }

                    // Only process if we have at least 2 existing files
                    if paths.len() >= 2 {
                        // Sort paths to ensure consistent ordering (alphabetically)
                        paths.sort();
                        debug!("Processing {} duplicates with hash: {}", paths.len(), hash);
                        if let Ok(result) = engine.process_duplicates(&paths) {
                            total_result.deleted += result.deleted;
                            total_result.moved += result.moved;
                            total_result.symlinked += result.symlinked;
                            total_result.archived += result.archived;
                            total_result.skipped += result.skipped;
                            total_result.space_freed += result.space_freed;
                        }
                    } else if paths.len() == 1 {
                        debug!("Only one file remains for hash {}, skipping", hash);
                    }
                }
            }

            println!("\n=== Deduplication Complete ===");
            println!("{}", total_result.summary());
        } else {
            println!("\nNote: Use --strategy flag to automatically deduplicate files.");
            println!("Available strategies: delete, move, symlink, archive");
            println!("Use --dry-run to preview changes without applying them.");
        }
    }

    db.close()?;

    Ok(())
}

async fn show_stats(config: &config::Config) -> Result<()> {
    info!("Gathering statistics");

    // Initialize database
    let db = db::DB::init(&config.db_file)?;

    // Get all buckets from the database directly
    let bucket_names = vec!["local", "smugmug", "test", "bucket1", "bucket2"];

    for bucket_name in &bucket_names {
        let bucket = db.bucket(bucket_name)?;
        let count = bucket.count()?;
        if count > 0 {
            info!("{} storage: {} files indexed", bucket_name, count);
        }
    }

    db.close()?;

    Ok(())
}
