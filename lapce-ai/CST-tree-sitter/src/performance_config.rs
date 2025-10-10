//! Performance configuration and SLO definitions
//! 
//! Service Level Objectives (SLOs) for CST-tree-sitter cache performance

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Performance SLO configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSLO {
    /// Target p50 latency in milliseconds for cache get operations
    pub get_p50_ms: f64,
    /// Target p95 latency in milliseconds for cache get operations
    pub get_p95_ms: f64,
    /// Target p99 latency in milliseconds for cache get operations
    pub get_p99_ms: f64,
    
    /// Target p50 latency in milliseconds for cache store operations
    pub store_p50_ms: f64,
    /// Target p95 latency in milliseconds for cache store operations
    pub store_p95_ms: f64,
    /// Target p99 latency in milliseconds for cache store operations
    pub store_p99_ms: f64,
    
    /// Minimum cache hit ratio (0.0 to 1.0)
    pub min_hit_ratio: f64,
    
    /// Maximum memory usage in MB
    pub max_memory_mb: usize,
    
    /// Minimum throughput operations per second
    pub min_throughput_ops: f64,
    
    /// Maximum tier promotion latency in milliseconds
    pub max_promotion_latency_ms: f64,
    
    /// Maximum tier demotion latency in milliseconds
    pub max_demotion_latency_ms: f64,
}

impl Default for PerformanceSLO {
    fn default() -> Self {
        Self {
            // Get operation latency targets
            get_p50_ms: 0.5,      // 500 microseconds
            get_p95_ms: 2.0,      // 2 milliseconds
            get_p99_ms: 10.0,     // 10 milliseconds
            
            // Store operation latency targets
            store_p50_ms: 1.0,    // 1 millisecond
            store_p95_ms: 5.0,    // 5 milliseconds
            store_p99_ms: 20.0,   // 20 milliseconds
            
            // Cache effectiveness
            min_hit_ratio: 0.85,  // 85% hit rate
            
            // Resource constraints
            max_memory_mb: 100,   // 100MB max memory
            
            // Throughput requirements
            min_throughput_ops: 1000.0,  // 1000 ops/sec
            
            // Tier transition latencies
            max_promotion_latency_ms: 5.0,
            max_demotion_latency_ms: 10.0,
        }
    }
}

/// Language-specific performance tuning parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageTuning {
    /// Average file size hint in bytes
    pub avg_file_size: usize,
    
    /// Compression algorithm preference
    pub compression: CompressionPolicy,
    
    /// Cache priority (higher = keep in hot tier longer)
    pub cache_priority: f64,
    
    /// Expected access pattern
    pub access_pattern: AccessPattern,
    
    /// Bytecode segment size override
    pub segment_size_override: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionPolicy {
    /// No compression
    None,
    /// LZ4 for fast compression
    Lz4,
    /// Zstd for better compression ratio
    Zstd(i32),  // Compression level
    /// Automatic selection based on size
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessPattern {
    /// Random access pattern
    Random,
    /// Sequential access pattern
    Sequential,
    /// Hot/cold pattern (some files accessed frequently)
    HotCold,
    /// Write-once-read-many pattern
    WriteOnceReadMany,
}

/// Auto-tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTuneConfig {
    /// Enable auto-tuning based on system resources
    pub enabled: bool,
    
    /// Target CPU usage percentage
    pub target_cpu_percent: f64,
    
    /// Target memory usage percentage
    pub target_memory_percent: f64,
    
    /// Minimum free memory to maintain (MB)
    pub min_free_memory_mb: usize,
    
    /// Adjustment interval in seconds
    pub adjustment_interval_secs: u64,
    
    /// Enable ML-based prediction (future)
    pub ml_prediction: bool,
}

impl Default for AutoTuneConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target_cpu_percent: 25.0,     // Use up to 25% CPU
            target_memory_percent: 10.0,   // Use up to 10% of system memory
            min_free_memory_mb: 500,       // Keep 500MB free
            adjustment_interval_secs: 60,  // Adjust every minute
            ml_prediction: false,           // Disabled for now
        }
    }
}

/// Complete performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Service level objectives
    pub slo: PerformanceSLO,
    
    /// Language-specific tuning
    pub language_tuning: HashMap<String, LanguageTuning>,
    
    /// Auto-tuning configuration
    pub auto_tune: AutoTuneConfig,
    
    /// Tier size ratios
    pub hot_tier_ratio: f64,
    pub warm_tier_ratio: f64,
    pub cold_tier_ratio: f64,
    
    /// Compression policy per tier
    pub hot_compression: CompressionPolicy,
    pub warm_compression: CompressionPolicy,
    pub cold_compression: CompressionPolicy,
    pub frozen_compression: CompressionPolicy,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        let mut language_tuning = HashMap::new();
        
        // Rust tuning
        language_tuning.insert("rust".to_string(), LanguageTuning {
            avg_file_size: 5000,
            compression: CompressionPolicy::Zstd(3),
            cache_priority: 1.2,  // Higher priority
            access_pattern: AccessPattern::HotCold,
            segment_size_override: None,
        });
        
        // JavaScript tuning  
        language_tuning.insert("javascript".to_string(), LanguageTuning {
            avg_file_size: 3000,
            compression: CompressionPolicy::Lz4,
            cache_priority: 1.0,
            access_pattern: AccessPattern::Random,
            segment_size_override: None,
        });
        
        // Python tuning
        language_tuning.insert("python".to_string(), LanguageTuning {
            avg_file_size: 2000,
            compression: CompressionPolicy::Lz4,
            cache_priority: 1.1,
            access_pattern: AccessPattern::Sequential,
            segment_size_override: None,
        });
        
        // C/C++ tuning
        language_tuning.insert("cpp".to_string(), LanguageTuning {
            avg_file_size: 10000,
            compression: CompressionPolicy::Zstd(5),
            cache_priority: 0.9,  // Lower priority, larger files
            access_pattern: AccessPattern::WriteOnceReadMany,
            segment_size_override: Some(512 * 1024), // Larger segments
        });
        
        Self {
            slo: PerformanceSLO::default(),
            language_tuning,
            auto_tune: AutoTuneConfig::default(),
            hot_tier_ratio: 0.2,   // 20% hot
            warm_tier_ratio: 0.3,   // 30% warm
            cold_tier_ratio: 0.5,   // 50% cold
            hot_compression: CompressionPolicy::None,
            warm_compression: CompressionPolicy::Lz4,
            cold_compression: CompressionPolicy::Zstd(3),
            frozen_compression: CompressionPolicy::Zstd(6),
        }
    }
}

/// SLO validation result
#[derive(Debug, Serialize, Deserialize)]
pub struct SLOValidation {
    pub passed: bool,
    pub get_p50_actual: f64,
    pub get_p95_actual: f64,
    pub get_p99_actual: f64,
    pub store_p50_actual: f64,
    pub store_p95_actual: f64,
    pub store_p99_actual: f64,
    pub hit_ratio_actual: f64,
    pub memory_mb_actual: usize,
    pub throughput_actual: f64,
    pub violations: Vec<String>,
}

impl SLOValidation {
    pub fn validate(slo: &PerformanceSLO, metrics: &crate::PerformanceMetrics) -> Self {
        let mut violations = Vec::new();
        
        if metrics.p50_latency_ms > slo.get_p50_ms {
            violations.push(format!(
                "Get P50 latency {:.2}ms exceeds SLO {:.2}ms",
                metrics.p50_latency_ms, slo.get_p50_ms
            ));
        }
        
        if metrics.p95_latency_ms > slo.get_p95_ms {
            violations.push(format!(
                "Get P95 latency {:.2}ms exceeds SLO {:.2}ms",
                metrics.p95_latency_ms, slo.get_p95_ms
            ));
        }
        
        if metrics.p99_latency_ms > slo.get_p99_ms {
            violations.push(format!(
                "Get P99 latency {:.2}ms exceeds SLO {:.2}ms",
                metrics.p99_latency_ms, slo.get_p99_ms
            ));
        }
        
        if metrics.memory_mb > slo.max_memory_mb as f64 {
            violations.push(format!(
                "Memory usage {:.1}MB exceeds SLO {}MB",
                metrics.memory_mb, slo.max_memory_mb
            ));
        }
        
        if metrics.throughput < slo.min_throughput_ops {
            violations.push(format!(
                "Throughput {:.0} ops/s below SLO {:.0} ops/s",
                metrics.throughput, slo.min_throughput_ops
            ));
        }
        
        Self {
            passed: violations.is_empty(),
            get_p50_actual: metrics.p50_latency_ms,
            get_p95_actual: metrics.p95_latency_ms,
            get_p99_actual: metrics.p99_latency_ms,
            store_p50_actual: metrics.p50_latency_ms,  // Would need separate store metrics
            store_p95_actual: metrics.p95_latency_ms,
            store_p99_actual: metrics.p99_latency_ms,
            hit_ratio_actual: 0.0,  // Would calculate from hit/miss counters
            memory_mb_actual: metrics.memory_mb as usize,
            throughput_actual: metrics.throughput,
            violations,
        }
    }
    
    /// Output validation result as JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Auto-tuner that adjusts cache parameters based on system resources
pub struct AutoTuner {
    config: AutoTuneConfig,
    performance_config: PerformanceConfig,
}

impl AutoTuner {
    pub fn new(config: AutoTuneConfig, performance_config: PerformanceConfig) -> Self {
        Self { config, performance_config }
    }
    
    /// Adjust cache configuration based on current system state
    pub fn tune(&mut self) -> crate::Phase4Config {
        if !self.config.enabled {
            return self.default_config();
        }
        
        // Get system info
        let cpu_count = num_cpus::get();
        let memory_mb = self.get_available_memory_mb();
        
        // Calculate memory budget based on target and constraints
        let target_memory = (memory_mb as f64 * self.config.target_memory_percent / 100.0) as usize;
        let max_memory = memory_mb.saturating_sub(self.config.min_free_memory_mb);
        let memory_budget_mb = target_memory.min(max_memory).min(self.performance_config.slo.max_memory_mb);
        
        // Adjust tier ratios based on available memory
        let (hot_ratio, warm_ratio) = if memory_budget_mb < 50 {
            (0.5, 0.3)  // More aggressive with limited memory
        } else if memory_budget_mb < 100 {
            (0.4, 0.3)  
        } else {
            (self.performance_config.hot_tier_ratio, self.performance_config.warm_tier_ratio)
        };
        
        crate::Phase4Config {
            memory_budget_mb,
            hot_tier_ratio: hot_ratio as f32,
            warm_tier_ratio: warm_ratio as f32,
            segment_size: self.calculate_segment_size(memory_budget_mb),
            storage_dir: std::env::temp_dir().join("cst_cache"),
            enable_compression: memory_budget_mb < 100,  // Enable compression when memory is tight
            test_mode: false,
        }
    }
    
    fn default_config(&self) -> crate::Phase4Config {
        crate::Phase4Config {
            memory_budget_mb: 50,
            hot_tier_ratio: 0.4,
            warm_tier_ratio: 0.3,
            segment_size: 256 * 1024,
            storage_dir: std::env::temp_dir().join("cst_cache"),
            enable_compression: true,
            test_mode: false,
        }
    }
    
    fn get_available_memory_mb(&self) -> usize {
        #[cfg(target_os = "linux")]
        {
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemAvailable:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<usize>() {
                                return kb / 1024;
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback: use sysinfo
        use sysinfo::System;
        let mut system = System::new();
        system.refresh_memory();
        (system.available_memory() / 1024 / 1024) as usize
    }
    
    fn calculate_segment_size(&self, memory_budget_mb: usize) -> usize {
        if memory_budget_mb < 20 {
            128 * 1024  // 128KB segments for very limited memory
        } else if memory_budget_mb < 50 {
            256 * 1024  // 256KB segments
        } else if memory_budget_mb < 100 {
            512 * 1024  // 512KB segments
        } else {
            1024 * 1024  // 1MB segments for plenty of memory
        }
    }
}
