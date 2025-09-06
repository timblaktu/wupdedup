mod config;
mod content;
mod db;
mod logging;
mod profiler;
mod storage;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{debug, error, info};

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
    
    /// Find duplicate files
    Dedupe {
        /// Show duplicates only
        #[arg(long)]
        show_only: bool,
    },
    
    /// Show statistics about stored files
    Stats,
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
            Commands::Dedupe { show_only } => {
                run_dedupe(&config, show_only).await?;
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

async fn run_dedupe(config: &config::Config, show_only: bool) -> Result<()> {
    info!("Starting deduplication analysis");
    
    // Initialize database
    let db = db::DB::init(&config.db_file)?;
    
    // Get all buckets from the database directly
    // For now, we'll check known bucket names (local, smugmug)
    let bucket_names = vec!["local", "smugmug", "test", "bucket1", "bucket2"];
    
    // Collect all files from all buckets
    let mut all_files: std::collections::HashMap<String, Vec<(String, String)>> = std::collections::HashMap::new();
    
    for bucket_name in &bucket_names {
        let bucket = db.bucket(bucket_name)?;
        let items = bucket.all()?;
        
        for (key, value) in items {
            // Deserialize the FileInfo
            if let Ok(file_info) = serde_json::from_slice::<serde_json::Value>(&value) {
                if let Some(hash) = file_info.get("hash").and_then(|h| h.as_str()) {
                    all_files.entry(hash.to_string())
                        .or_insert_with(Vec::new)
                        .push((bucket_name.to_string(), key));
                }
            }
        }
    }
    
    // Find and report duplicates
    let mut duplicate_groups = 0;
    let mut duplicate_files = 0;
    let mut space_wasted: u64 = 0;
    
    println!("\n=== Duplicate Files Report ===");
    
    for (hash, files) in &all_files {
        if files.len() > 1 {
            duplicate_groups += 1;
            duplicate_files += files.len() - 1; // Count extras only
            
            println!("\nHash: {}", &hash[..16]); // Show first 16 chars of hash
            println!("Files ({}):", files.len());
            
            for (storage, path) in files {
                println!("  [{:8}] {}", storage, path);
                
                // Get file size for space calculation
                if let Ok(bucket) = db.bucket(storage) {
                    if let Ok(Some(value)) = bucket.get(path) {
                        if let Ok(file_info) = serde_json::from_slice::<serde_json::Value>(&value) {
                            if let Some(size) = file_info.get("size").and_then(|s| s.as_u64()) {
                                if files.iter().position(|(s, p)| s == storage && p == path).unwrap() > 0 {
                                    space_wasted += size;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("\n=== Summary ===");
    println!("Total duplicate groups: {}", duplicate_groups);
    println!("Total duplicate files:  {}", duplicate_files);
    println!("Space wasted:          {} MB", space_wasted / (1024 * 1024));
    
    if !show_only && duplicate_groups > 0 {
        println!("\nNote: Use --show-only flag to preview duplicates without making changes.");
        println!("Automatic deduplication not yet implemented.");
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
