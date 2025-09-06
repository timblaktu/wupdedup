use anyhow::Result;
use tracing::{Level, debug};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init(log_level: &str) -> Result<()> {
    // Convert string log level to tracing Level
    let level = match log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    
    // Create an env filter that respects both config and environment variables
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level.to_string()));
    
    // Initialize the tracing subscriber
    tracing_subscriber::registry()
        .with(fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true))
        .with(env_filter)
        .init();
    
    debug!("Logging initialized with level: {}", log_level);
    Ok(())
}

#[allow(dead_code)]
pub fn init_json(log_level: &str) -> Result<()> {
    // Convert string log level to tracing Level
    let level = match log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    
    // Create an env filter that respects both config and environment variables
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level.to_string()));
    
    // Initialize the tracing subscriber with JSON formatting
    tracing_subscriber::registry()
        .with(fmt::layer()
            .json()
            .with_target(true)
            .with_current_span(true)
            .with_file(true)
            .with_line_number(true))
        .with(env_filter)
        .init();
    
    debug!("JSON logging initialized with level: {}", log_level);
    Ok(())
}