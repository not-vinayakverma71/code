//! Production-grade logging configuration
//! Uses tracing for structured logging with multiple output formats

use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};
use std::path::PathBuf;

/// Log level configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    pub level: LogLevel,
    pub json_output: bool,
    pub include_timestamps: bool,
    pub include_thread_ids: bool,
    pub include_file_locations: bool,
    pub log_file: Option<PathBuf>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            json_output: false,
            include_timestamps: true,
            include_thread_ids: false,
            include_file_locations: cfg!(debug_assertions),
            log_file: None,
        }
    }
}

impl LogConfig {
    /// Production configuration (JSON, structured logs)
    pub fn production() -> Self {
        Self {
            level: LogLevel::Info,
            json_output: true,
            include_timestamps: true,
            include_thread_ids: true,
            include_file_locations: false,
            log_file: Some(PathBuf::from("/var/log/lapce-tree-sitter/app.log")),
        }
    }

    /// Development configuration (pretty output, verbose)
    pub fn development() -> Self {
        Self {
            level: LogLevel::Debug,
            json_output: false,
            include_timestamps: true,
            include_thread_ids: false,
            include_file_locations: true,
            log_file: None,
        }
    }

    /// Performance profiling configuration
    pub fn profiling() -> Self {
        Self {
            level: LogLevel::Trace,
            json_output: true,
            include_timestamps: true,
            include_thread_ids: true,
            include_file_locations: true,
            log_file: Some(PathBuf::from("./perf.log")),
        }
    }
}

/// Initialize logging with the given configuration
pub fn init_logging(config: LogConfig) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            let level = match config.level {
                LogLevel::Trace => "trace",
                LogLevel::Debug => "debug",
                LogLevel::Info => "info",
                LogLevel::Warn => "warn",
                LogLevel::Error => "error",
            };
            EnvFilter::new(format!("lapce_tree_sitter={},tree_sitter={}", level, level))
        });

    // Console output layer
    let console_layer = if config.json_output {
        fmt::layer()
            .json()
            .with_span_events(FmtSpan::CLOSE)
            .with_current_span(true)
            .with_thread_ids(config.include_thread_ids)
            .with_file(config.include_file_locations)
            .with_line_number(config.include_file_locations)
            .boxed()
    } else {
        fmt::layer()
            .pretty()
            .with_thread_ids(config.include_thread_ids)
            .with_file(config.include_file_locations)
            .with_line_number(config.include_file_locations)
            .boxed()
    };

    // File output layer (if configured)
    let file_layer = if let Some(log_file) = &config.log_file {
        if let Some(parent) = log_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;
        Some(
            fmt::layer()
                .json()
                .with_writer(file)
                .with_span_events(FmtSpan::CLOSE)
                .boxed(),
        )
    } else {
        None
    };

    // Build subscriber
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer);

    if let Some(file_layer) = file_layer {
        subscriber.with(file_layer).init();
    } else {
        subscriber.init();
    }

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        config = ?config,
        "Logging initialized"
    );

    Ok(())
}

/// Performance metrics logging
pub mod metrics {
    use std::time::{Duration, Instant};

    pub struct OperationTimer {
        name: String,
        start: Instant,
        threshold_ms: Option<u64>,
    }

    impl OperationTimer {
        pub fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                start: Instant::now(),
                threshold_ms: None,
            }
        }

        pub fn with_threshold(mut self, threshold_ms: u64) -> Self {
            self.threshold_ms = Some(threshold_ms);
            self
        }

        pub fn elapsed(&self) -> Duration {
            self.start.elapsed()
        }

        pub fn finish(self) {
            let elapsed = self.elapsed();
            let elapsed_ms = elapsed.as_millis();

            if let Some(threshold) = self.threshold_ms {
                if elapsed_ms > threshold as u128 {
                    tracing::warn!(
                        operation = %self.name,
                        elapsed_ms = elapsed_ms,
                        threshold_ms = threshold,
                        "Operation exceeded threshold"
                    );
                } else {
                    tracing::debug!(
                        operation = %self.name,
                        elapsed_ms = elapsed_ms,
                        "Operation completed"
                    );
                }
            } else {
                tracing::debug!(
                    operation = %self.name,
                    elapsed_ms = elapsed_ms,
                    "Operation completed"
                );
            }
        }
    }

    impl Drop for OperationTimer {
        fn drop(&mut self) {
            // Auto-log if not explicitly finished
            let elapsed_ms = self.elapsed().as_millis();
            tracing::trace!(
                operation = %self.name,
                elapsed_ms = elapsed_ms,
                "Operation timer dropped"
            );
        }
    }

    /// Log memory usage
    pub fn log_memory_usage(context: &str, bytes: usize) {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        tracing::info!(
            context = context,
            bytes = bytes,
            mb = format!("{:.2}", mb),
            "Memory usage"
        );
    }

    /// Log cache statistics
    pub fn log_cache_stats(
        hits: usize,
        misses: usize,
        size: usize,
        capacity: usize,
    ) {
        let hit_rate = if hits + misses > 0 {
            (hits as f64 / (hits + misses) as f64) * 100.0
        } else {
            0.0
        };

        tracing::info!(
            hits = hits,
            misses = misses,
            hit_rate = format!("{:.2}%", hit_rate),
            size = size,
            capacity = capacity,
            "Cache statistics"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_config_defaults() {
        let config = LogConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert!(!config.json_output);
        assert!(config.include_timestamps);
    }

    #[test]
    fn test_production_config() {
        let config = LogConfig::production();
        assert_eq!(config.level, LogLevel::Info);
        assert!(config.json_output);
        assert!(config.include_thread_ids);
    }

    #[test]
    fn test_development_config() {
        let config = LogConfig::development();
        assert_eq!(config.level, LogLevel::Debug);
        assert!(!config.json_output);
        assert!(config.include_file_locations);
    }
}
