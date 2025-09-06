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
    
    // TODO: Implement deduplication logic
    // - Read all file hashes from database
    // - Group files by hash
    // - Report or handle duplicates
    
    if show_only {
        info!("Showing duplicates only (no changes will be made)");
    } else {
        info!("Would process duplicates (not yet implemented)");
    }
    
    db.close()?;
    
    Ok(())
}

async fn show_stats(config: &config::Config) -> Result<()> {
    info!("Gathering statistics");
    
    // Initialize database
    let db = db::DB::init(&config.db_file)?;
    
    // Load storage contexts to get bucket names
    let contexts = storage::load_storage_strategy_contexts(config)?;
    
    for context in &contexts {
        let bucket = db.bucket(&context.name)?;
        let count = bucket.count()?;
        info!("{} storage: {} files indexed", context.name, count);
    }
    
    db.close()?;
    
    Ok(())
}
