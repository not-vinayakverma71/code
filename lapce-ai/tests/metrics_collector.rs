/// Metrics Collection and Reporting for IPC Tests
/// Generates structured output for CI/CD analysis

use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct TestMetrics {
    pub test_name: String,
    pub timestamp: u64,
    pub os: String,
    pub duration_ms: u64,
    pub throughput: ThroughputMetrics,
    pub latency: LatencyMetrics,
    pub memory: MemoryMetrics,
    pub errors: ErrorMetrics,
    pub system: SystemMetrics,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    pub total_messages: u64,
    pub messages_per_second: f64,
    pub bytes_transferred: u64,
    pub megabytes_per_second: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LatencyMetrics {
    pub min_us: f64,
    pub max_us: f64,
    pub avg_us: f64,
    pub p50_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub violations_count: u64,
    pub violation_percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub initial_mb: f64,
    pub peak_mb: f64,
    pub final_mb: f64,
    pub growth_mb: f64,
    pub growth_percentage: f64,
    pub buffer_pool_efficiency: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorMetrics {
    pub total_errors: u64,
    pub timeout_errors: u64,
    pub connection_errors: u64,
    pub recovery_failures: u64,
    pub error_rate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_cores: usize,
    pub total_memory_gb: f64,
    pub available_memory_gb: f64,
    pub load_average: [f64; 3],
}

pub struct MetricsCollector {
    start_time: Instant,
    samples: Vec<Sample>,
    errors: Vec<ErrorEvent>,
}

struct Sample {
    timestamp: Duration,
    latency_us: f64,
    memory_bytes: u64,
    active_connections: usize,
}

struct ErrorEvent {
    timestamp: Duration,
    error_type: String,
    recovery_time_ms: Option<u64>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            samples: Vec::new(),
            errors: Vec::new(),
        }
    }
    
    pub fn record_sample(&mut self, latency_us: f64, memory_bytes: u64, connections: usize) {
        self.samples.push(Sample {
            timestamp: self.start_time.elapsed(),
            latency_us,
            memory_bytes,
            active_connections: connections,
        });
    }
    
    pub fn record_error(&mut self, error_type: String, recovery_time: Option<Duration>) {
        self.errors.push(ErrorEvent {
            timestamp: self.start_time.elapsed(),
            error_type,
            recovery_time_ms: recovery_time.map(|d| d.as_millis() as u64),
        });
    }
    
    pub fn generate_report(&self, test_name: &str) -> TestMetrics {
        let duration = self.start_time.elapsed();
        
        // Calculate latency statistics
        let mut latencies: Vec<f64> = self.samples.iter()
            .map(|s| s.latency_us)
            .collect();
        latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let latency_metrics = if !latencies.is_empty() {
            LatencyMetrics {
                min_us: *latencies.first().unwrap_or(&0.0),
                max_us: *latencies.last().unwrap_or(&0.0),
                avg_us: latencies.iter().sum::<f64>() / latencies.len() as f64,
                p50_us: percentile(&latencies, 50.0),
                p95_us: percentile(&latencies, 95.0),
                p99_us: percentile(&latencies, 99.0),
                violations_count: latencies.iter().filter(|&&l| l > 10.0).count() as u64,
                violation_percentage: (latencies.iter().filter(|&&l| l > 10.0).count() as f64 
                    / latencies.len() as f64) * 100.0,
            }
        } else {
            Default::default()
        };
        
        // Calculate memory statistics  
        let memories: Vec<u64> = self.samples.iter().map(|s| s.memory_bytes).collect();
        let initial_memory = memories.first().copied().unwrap_or(0);
        let peak_memory = memories.iter().max().copied().unwrap_or(0);
        let final_memory = memories.last().copied().unwrap_or(0);
        
        let memory_metrics = MemoryMetrics {
            initial_mb: initial_memory as f64 / 1_048_576.0,
            peak_mb: peak_memory as f64 / 1_048_576.0,
            final_mb: final_memory as f64 / 1_048_576.0,
            growth_mb: (final_memory as i64 - initial_memory as i64).abs() as f64 / 1_048_576.0,
            growth_percentage: if initial_memory > 0 {
                ((final_memory as f64 - initial_memory as f64) / initial_memory as f64) * 100.0
            } else {
                0.0
            },
            buffer_pool_efficiency: calculate_buffer_efficiency(&memories),
        };
        
        // Calculate throughput
        let total_messages = self.samples.len() as u64;
        let throughput_metrics = ThroughputMetrics {
            total_messages,
            messages_per_second: total_messages as f64 / duration.as_secs_f64(),
            bytes_transferred: total_messages * 1024, // Assuming 1KB messages
            megabytes_per_second: (total_messages as f64 * 1024.0) 
                / 1_048_576.0 / duration.as_secs_f64(),
        };
        
        // Calculate error metrics
        let error_metrics = ErrorMetrics {
            total_errors: self.errors.len() as u64,
            timeout_errors: self.errors.iter()
                .filter(|e| e.error_type.contains("timeout"))
                .count() as u64,
            connection_errors: self.errors.iter()
                .filter(|e| e.error_type.contains("connection"))
                .count() as u64,
            recovery_failures: self.errors.iter()
                .filter(|e| e.recovery_time_ms.is_none())
                .count() as u64,
            error_rate: (self.errors.len() as f64 / total_messages as f64) * 100.0,
        };
        
        // Get system metrics
        let system_metrics = get_system_metrics();
        
        TestMetrics {
            test_name: test_name.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            os: std::env::consts::OS.to_string(),
            duration_ms: duration.as_millis() as u64,
            throughput: throughput_metrics,
            latency: latency_metrics,
            memory: memory_metrics,
            errors: error_metrics,
            system: system_metrics,
        }
    }
    
    pub fn save_json(&self, test_name: &str, path: &str) -> std::io::Result<()> {
        let metrics = self.generate_report(test_name);
        let json = serde_json::to_string_pretty(&metrics)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
    
    pub fn save_csv(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        
        // Write CSV header
        writeln!(file, "timestamp_ms,latency_us,memory_mb,connections,error")?;
        
        // Write samples
        for sample in &self.samples {
            writeln!(
                file,
                "{},{},{},{},",
                sample.timestamp.as_millis(),
                sample.latency_us,
                sample.memory_bytes as f64 / 1_048_576.0,
                sample.active_connections
            )?;
        }
        
        // Write errors
        for error in &self.errors {
            writeln!(
                file,
                "{},,,{}",
                error.timestamp.as_millis(),
                error.error_type
            )?;
        }
        
        Ok(())
    }
}

fn percentile(sorted_values: &[f64], p: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    
    let index = ((p / 100.0) * (sorted_values.len() - 1) as f64) as usize;
    sorted_values[index]
}

fn calculate_buffer_efficiency(memories: &[u64]) -> f64 {
    if memories.len() < 2 {
        return 100.0;
    }
    
    // Calculate variance to measure memory stability
    let mean = memories.iter().sum::<u64>() as f64 / memories.len() as f64;
    let variance = memories.iter()
        .map(|&m| (m as f64 - mean).powi(2))
        .sum::<f64>() / memories.len() as f64;
    
    // Lower variance = better buffer reuse
    let coefficient_of_variation = (variance.sqrt() / mean) * 100.0;
    
    // Invert so higher score = better efficiency
    100.0 - coefficient_of_variation.min(100.0)
}

fn get_system_metrics() -> SystemMetrics {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        
        let cpu_cores = num_cpus::get();
        
        let (total_mem, avail_mem) = super::os_specific_config::get_system_memory();
        
        let load_avg = fs::read_to_string("/proc/loadavg")
            .unwrap_or_default()
            .split_whitespace()
            .take(3)
            .map(|s| s.parse::<f64>().unwrap_or(0.0))
            .collect::<Vec<_>>();
            
        SystemMetrics {
            cpu_cores,
            total_memory_gb: total_mem as f64 / (1024.0 * 1024.0 * 1024.0),
            available_memory_gb: avail_mem as f64 / (1024.0 * 1024.0 * 1024.0),
            load_average: [
                load_avg.get(0).copied().unwrap_or(0.0),
                load_avg.get(1).copied().unwrap_or(0.0),
                load_avg.get(2).copied().unwrap_or(0.0),
            ],
        }
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        let (total_mem, avail_mem) = super::os_specific_config::get_system_memory();
        
        SystemMetrics {
            cpu_cores: num_cpus::get(),
            total_memory_gb: total_mem as f64 / (1024.0 * 1024.0 * 1024.0),
            available_memory_gb: avail_mem as f64 / (1024.0 * 1024.0 * 1024.0),
            load_average: [0.0, 0.0, 0.0],
        }
    }
}

impl Default for LatencyMetrics {
    fn default() -> Self {
        Self {
            min_us: 0.0,
            max_us: 0.0,
            avg_us: 0.0,
            p50_us: 0.0,
            p95_us: 0.0,
            p99_us: 0.0,
            violations_count: 0,
            violation_percentage: 0.0,
        }
    }
}
