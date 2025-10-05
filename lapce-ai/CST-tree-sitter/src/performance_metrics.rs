//! Performance metrics measurement for tree-sitter integration
//! Validates all success criteria from requirements

use std::time::{Duration, Instant};
use std::sync::Arc;
use parking_lot::RwLock;
use sysinfo::{System, Pid};

/// Performance metrics tracker
pub struct PerformanceTracker {
    // Memory metrics
    memory_start: u64,
    memory_peak: Arc<RwLock<u64>>,
    memory_samples: Arc<RwLock<Vec<u64>>>,
    
    // Parse metrics
    parse_times: Arc<RwLock<Vec<Duration>>>,
    lines_parsed: Arc<RwLock<u64>>,
    bytes_parsed: Arc<RwLock<u64>>,
    
    // Cache metrics
    cache_hits: Arc<RwLock<u64>>,
    cache_misses: Arc<RwLock<u64>>,
    
    // Incremental parse metrics
    incremental_times: Arc<RwLock<Vec<Duration>>>,
    full_parse_times: Arc<RwLock<Vec<Duration>>>,
    
    // Symbol extraction metrics
    symbol_extraction_times: Arc<RwLock<Vec<Duration>>>,
    symbols_extracted: Arc<RwLock<u64>>,
    
    // Query performance
    query_times: Arc<RwLock<Vec<Duration>>>,
    
    // System info
    system: System,
    process_id: u32,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_processes();
        
        let process_id = std::process::id();
        let memory_start = Self::get_current_memory(&system, process_id);
        
        Self {
            memory_start,
            memory_peak: Arc::new(RwLock::new(memory_start)),
            memory_samples: Arc::new(RwLock::new(Vec::new())),
            parse_times: Arc::new(RwLock::new(Vec::new())),
            lines_parsed: Arc::new(RwLock::new(0)),
            bytes_parsed: Arc::new(RwLock::new(0)),
            cache_hits: Arc::new(RwLock::new(0)),
            cache_misses: Arc::new(RwLock::new(0)),
            incremental_times: Arc::new(RwLock::new(Vec::new())),
            full_parse_times: Arc::new(RwLock::new(Vec::new())),
            symbol_extraction_times: Arc::new(RwLock::new(Vec::new())),
            symbols_extracted: Arc::new(RwLock::new(0)),
            query_times: Arc::new(RwLock::new(Vec::new())),
            system,
            process_id,
        }
    }
    
    /// Get current memory usage in bytes
    fn get_current_memory(system: &System, pid: u32) -> u64 {
        if let Some(process) = system.process(Pid::from_u32(pid)) {
            process.memory() * 1024 // Convert from KB to bytes
        } else {
            0
        }
    }
    
    /// Sample current memory usage
    pub fn sample_memory(&mut self) {
        self.system.refresh_process(Pid::from_u32(self.process_id));
        let current = Self::get_current_memory(&self.system, self.process_id);
        
        self.memory_samples.write().push(current);
        
        let mut peak = self.memory_peak.write();
        if current > *peak {
            *peak = current;
        }
    }
    
    /// Record a parse operation
    pub fn record_parse(&self, duration: Duration, lines: usize, bytes: usize) {
        self.parse_times.write().push(duration);
        *self.lines_parsed.write() += lines as u64;
        *self.bytes_parsed.write() += bytes as u64;
    }
    
    /// Record an incremental parse
    pub fn record_incremental_parse(&self, duration: Duration) {
        self.incremental_times.write().push(duration);
    }
    
    /// Record a full parse
    pub fn record_full_parse(&self, duration: Duration) {
        self.full_parse_times.write().push(duration);
    }
    
    /// Record cache hit
    pub fn record_cache_hit(&self) {
        *self.cache_hits.write() += 1;
    }
    
    /// Record cache miss
    pub fn record_cache_miss(&self) {
        *self.cache_misses.write() += 1;
    }
    
    /// Record symbol extraction
    pub fn record_symbol_extraction(&self, duration: Duration, symbol_count: usize) {
        self.symbol_extraction_times.write().push(duration);
        *self.symbols_extracted.write() += symbol_count as u64;
    }
    
    /// Record query performance
    pub fn record_query(&self, duration: Duration) {
        self.query_times.write().push(duration);
    }
    
    /// Get parse statistics
    pub fn get_parse_stats(&self) -> ParseStats {
        let parse_times = self.parse_times.read();
        let total_parses = parse_times.len() as u64;
        let average_time_ms = if parse_times.is_empty() {
            0.0
        } else {
            parse_times.iter().map(|d| d.as_secs_f64() * 1000.0).sum::<f64>() / parse_times.len() as f64
        };
        
        let lines = *self.lines_parsed.read();
        let bytes = *self.bytes_parsed.read();
        
        let total_time = parse_times.iter()
            .fold(Duration::from_secs(0), |acc, d| acc + *d);
        let avg_time = if !parse_times.is_empty() {
            total_time / parse_times.len() as u32
        } else {
            Duration::from_secs(0)
        };
        
        let lines_per_second = if total_time.as_secs_f64() > 0.0 {
            lines as f64 / total_time.as_secs_f64()
        } else {
            0.0
        };
        
        ParseStats {
            total_parses,
            average_time_ms,
            total_lines: lines,
            total_bytes: bytes,
            average_parse_time: avg_time,
            lines_per_second,
            bytes_per_second: bytes as f64 / total_time.as_secs_f64().max(0.001),
        }
    }
    
    /// Get memory usage statistics
    pub fn get_memory_stats(&self) -> MemoryStats {
        let samples = self.memory_samples.read();
        let peak = *self.memory_peak.read();
        
        let current_usage = peak.saturating_sub(self.memory_start);
        let avg_usage = if samples.is_empty() {
            current_usage
        } else {
            let sum: u64 = samples.iter()
                .map(|&s| s.saturating_sub(self.memory_start))
                .sum();
            sum / samples.len() as u64
        };
        
        MemoryStats {
            peak_usage_bytes: current_usage,
            average_usage_bytes: avg_usage,
            peak_usage_mb: current_usage as f64 / 1_048_576.0,
            average_usage_mb: avg_usage as f64 / 1_048_576.0,
        }
    }
    
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> CacheStats {
        let hits = *self.cache_hits.read();
        let misses = *self.cache_misses.read();
        let total = hits + misses;
        
        CacheStats {
            hits,
            misses,
            hit_rate: if total > 0 {
                (hits as f64 / total as f64) * 100.0
            } else {
                0.0
            },
        }
    }
    
    /// Get incremental parsing statistics
    pub fn get_incremental_stats(&self) -> IncrementalStats {
        let incremental = self.incremental_times.read();
        let full = self.full_parse_times.read();
        
        let avg_incremental = if incremental.is_empty() {
            Duration::from_secs(0)
        } else {
            incremental.iter().sum::<Duration>() / incremental.len() as u32
        };
        
        let avg_full = if full.is_empty() {
            Duration::from_secs(0)
        } else {
            full.iter().sum::<Duration>() / full.len() as u32
        };
        
        IncrementalStats {
            incremental_parses: incremental.len(),
            full_parses: full.len(),
            average_incremental_time: avg_incremental,
            average_full_time: avg_full,
            speedup_factor: if avg_incremental.as_secs_f64() > 0.0 {
                avg_full.as_secs_f64() / avg_incremental.as_secs_f64()
            } else {
                1.0
            },
        }
    }
    
    /// Get symbol extraction statistics
    pub fn get_symbol_stats(&self) -> SymbolStats {
        let times = self.symbol_extraction_times.read();
        let symbols = *self.symbols_extracted.read();
        
        let avg_time = if times.is_empty() {
            Duration::from_secs(0)
        } else {
            times.iter().sum::<Duration>() / times.len() as u32
        };
        
        SymbolStats {
            total_extractions: times.len(),
            total_symbols: symbols,
            average_extraction_time: avg_time,
            symbols_per_extraction: if times.is_empty() {
                0.0
            } else {
                symbols as f64 / times.len() as f64
            },
        }
    }
    
    /// Get query performance statistics
    pub fn get_query_stats(&self) -> QueryStats {
        let times = self.query_times.read();
        
        let avg_time = if times.is_empty() {
            Duration::from_secs(0)
        } else {
            times.iter().sum::<Duration>() / times.len() as u32
        };
        
        let p95_time = if times.is_empty() {
            Duration::from_secs(0)
        } else {
            let mut sorted = times.clone();
            sorted.sort();
            let idx = (sorted.len() as f64 * 0.95) as usize;
            sorted[idx.min(sorted.len() - 1)]
        };
        
        QueryStats {
            total_queries: times.len(),
            average_query_time: avg_time,
            p95_query_time: p95_time,
        }
    }
    
    /// Generate comprehensive report
    pub fn generate_report(&self) -> PerformanceReport {
        PerformanceReport {
            memory: self.get_memory_stats(),
            parse: self.get_parse_stats(),
            cache: self.get_cache_stats(),
            incremental: self.get_incremental_stats(),
            symbols: self.get_symbol_stats(),
            queries: self.get_query_stats(),
        }
    }
    
    /// Check if all success criteria are met
    pub fn check_success_criteria(&self) -> SuccessCriteria {
        let memory = self.get_memory_stats();
        let parse = self.get_parse_stats();
        let cache = self.get_cache_stats();
        let incremental = self.get_incremental_stats();
        let symbols = self.get_symbol_stats();
        let queries = self.get_query_stats();
        
        SuccessCriteria {
            memory_under_5mb: memory.peak_usage_mb < 5.0,
            memory_actual_mb: memory.peak_usage_mb,
            
            parse_speed_over_10k: parse.lines_per_second > 10_000.0,
            parse_speed_actual: parse.lines_per_second,
            
            incremental_under_10ms: incremental.average_incremental_time < Duration::from_millis(10),
            incremental_actual_ms: incremental.average_incremental_time.as_secs_f64() * 1000.0,
            
            symbol_extraction_under_50ms: symbols.average_extraction_time < Duration::from_millis(50),
            symbol_extraction_actual_ms: symbols.average_extraction_time.as_secs_f64() * 1000.0,
            
            cache_hit_rate_over_90: cache.hit_rate > 90.0,
            cache_hit_rate_actual: cache.hit_rate,
            
            query_performance_under_1ms: queries.average_query_time < Duration::from_millis(1),
            query_performance_actual_ms: queries.average_query_time.as_secs_f64() * 1000.0,
            
            test_coverage_over_1m_lines: parse.total_lines > 1_000_000,
            test_coverage_actual_lines: parse.total_lines,
        }
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub peak_usage_bytes: u64,
    pub average_usage_bytes: u64,
    pub peak_usage_mb: f64,
    pub average_usage_mb: f64,
}

/// Parse performance statistics
#[derive(Debug, Clone)]
pub struct ParseStats {
    pub total_parses: u64,
    pub average_time_ms: f64,
    pub total_lines: u64,
    pub total_bytes: u64,
    pub average_parse_time: Duration,
    pub lines_per_second: f64,
    pub bytes_per_second: f64,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

/// Incremental parsing statistics
#[derive(Debug, Clone)]
pub struct IncrementalStats {
    pub incremental_parses: usize,
    pub full_parses: usize,
    pub average_incremental_time: Duration,
    pub average_full_time: Duration,
    pub speedup_factor: f64,
}

/// Symbol extraction statistics
#[derive(Debug, Clone)]
pub struct SymbolStats {
    pub total_extractions: usize,
    pub total_symbols: u64,
    pub average_extraction_time: Duration,
    pub symbols_per_extraction: f64,
}

/// Query performance statistics
#[derive(Debug, Clone)]
pub struct QueryStats {
    pub total_queries: usize,
    pub average_query_time: Duration,
    pub p95_query_time: Duration,
}

/// Performance report combining all metrics
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub memory: MemoryStats,
    pub parse: ParseStats,
    pub cache: CacheStats,
    pub incremental: IncrementalStats,
    pub symbols: SymbolStats,
    pub queries: QueryStats,
}

/// Success criteria validation
#[derive(Debug)]
pub struct SuccessCriteria {
    // Memory < 5MB
    pub memory_under_5mb: bool,
    pub memory_actual_mb: f64,
    
    // Parse speed > 10K lines/sec
    pub parse_speed_over_10k: bool,
    pub parse_speed_actual: f64,
    
    // Incremental parsing < 10ms
    pub incremental_under_10ms: bool,
    pub incremental_actual_ms: f64,
    
    // Symbol extraction < 50ms for 1K lines
    pub symbol_extraction_under_50ms: bool,
    pub symbol_extraction_actual_ms: f64,
    
    // Cache hit rate > 90%
    pub cache_hit_rate_over_90: bool,
    pub cache_hit_rate_actual: f64,
    
    // Query performance < 1ms
    pub query_performance_under_1ms: bool,
    pub query_performance_actual_ms: f64,
    
    // Test coverage > 1M lines
    pub test_coverage_over_1m_lines: bool,
    pub test_coverage_actual_lines: u64,
}

impl SuccessCriteria {
    /// Check if all criteria are met
    pub fn all_passed(&self) -> bool {
        self.memory_under_5mb &&
        self.parse_speed_over_10k &&
        self.incremental_under_10ms &&
        self.symbol_extraction_under_50ms &&
        self.cache_hit_rate_over_90 &&
        self.query_performance_under_1ms &&
        self.test_coverage_over_1m_lines
    }
    
    /// Get a summary report
    pub fn summary(&self) -> String {
        let mut report = String::new();
        report.push_str("🎯 SUCCESS CRITERIA VALIDATION\n");
        report.push_str(&"=".repeat(70));
        report.push_str("\n\n");
        
        report.push_str(&format!("1. Memory Usage < 5MB: {} (Actual: {:.2} MB)\n",
            if self.memory_under_5mb { "✅ PASS" } else { "❌ FAIL" },
            self.memory_actual_mb
        ));
        
        report.push_str(&format!("2. Parse Speed > 10K lines/sec: {} (Actual: {:.0} lines/sec)\n",
            if self.parse_speed_over_10k { "✅ PASS" } else { "❌ FAIL" },
            self.parse_speed_actual
        ));
        
        report.push_str(&format!("3. Incremental Parsing < 10ms: {} (Actual: {:.2} ms)\n",
            if self.incremental_under_10ms { "✅ PASS" } else { "❌ FAIL" },
            self.incremental_actual_ms
        ));
        
        report.push_str(&format!("4. Symbol Extraction < 50ms: {} (Actual: {:.2} ms)\n",
            if self.symbol_extraction_under_50ms { "✅ PASS" } else { "❌ FAIL" },
            self.symbol_extraction_actual_ms
        ));
        
        report.push_str(&format!("5. Cache Hit Rate > 90%: {} (Actual: {:.1}%)\n",
            if self.cache_hit_rate_over_90 { "✅ PASS" } else { "❌ FAIL" },
            self.cache_hit_rate_actual
        ));
        
        report.push_str(&format!("6. Query Performance < 1ms: {} (Actual: {:.2} ms)\n",
            if self.query_performance_under_1ms { "✅ PASS" } else { "❌ FAIL" },
            self.query_performance_actual_ms
        ));
        
        report.push_str(&format!("7. Test Coverage > 1M lines: {} (Actual: {} lines)\n",
            if self.test_coverage_over_1m_lines { "✅ PASS" } else { "❌ FAIL" },
            self.test_coverage_actual_lines
        ));
        
        report.push_str(&format!("\n{}\n", "=".repeat(70)));
        report.push_str(&format!("OVERALL: {}\n", 
            if self.all_passed() { "✅ ALL CRITERIA PASSED!" } else { "❌ Some criteria not met" }
        ));
        
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_performance_tracker() {
        let mut tracker = PerformanceTracker::new();
        
        // Record some operations
        tracker.record_parse(Duration::from_millis(10), 1000, 50000);
        tracker.record_cache_hit();
        tracker.record_cache_hit();
        tracker.record_cache_miss();
        tracker.sample_memory();
        
        // Check stats
        let parse_stats = tracker.get_parse_stats();
        assert_eq!(parse_stats.total_lines, 1000);
        
        let cache_stats = tracker.get_cache_stats();
        assert_eq!(cache_stats.hits, 2);
        assert_eq!(cache_stats.misses, 1);
        assert!(cache_stats.hit_rate > 60.0);
    }
}
