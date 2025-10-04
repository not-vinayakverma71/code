/// CPU Profiling & Optimization - Day 42 PM
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct CpuProfiler {
    samples: Arc<RwLock<Vec<CpuSample>>>,
    function_times: Arc<RwLock<HashMap<String, FunctionProfile>>>,
    sampling_interval: Duration,
}

#[derive(Clone, Debug)]
pub struct CpuSample {
    pub timestamp: Instant,
    pub cpu_usage_percent: f32,
    pub thread_count: usize,
    pub context_switches: u64,
}

#[derive(Clone, Debug)]
pub struct FunctionProfile {
    pub name: String,
    pub call_count: u64,
    pub total_time: Duration,
    pub avg_time: Duration,
    pub max_time: Duration,
    pub min_time: Duration,
}

impl CpuProfiler {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(RwLock::new(Vec::new())),
            function_times: Arc::new(RwLock::new(HashMap::new())),
            sampling_interval: Duration::from_millis(100),
        }
    }
    
    pub async fn start_profiling(&self) {
        let samples = self.samples.clone();
        let interval = self.sampling_interval;
        
        tokio::spawn(async move {
            loop {
                let sample = CpuSample {
                    timestamp: Instant::now(),
                    cpu_usage_percent: get_cpu_usage(),
                    thread_count: get_thread_count(),
                    context_switches: 0,
                };
                
                samples.write().await.push(sample);
                tokio::time::sleep(interval).await;
            }
        });
    }
    
    pub async fn profile_function<F, R>(&self, name: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        
        let mut profiles = self.function_times.write().await;
        let profile = profiles.entry(name.to_string()).or_insert(FunctionProfile {
            name: name.to_string(),
            call_count: 0,
            total_time: Duration::ZERO,
            avg_time: Duration::ZERO,
            max_time: Duration::ZERO,
            min_time: Duration::MAX,
        });
        
        profile.call_count += 1;
        profile.total_time += duration;
        profile.avg_time = profile.total_time / profile.call_count as u32;
        profile.max_time = profile.max_time.max(duration);
        profile.min_time = profile.min_time.min(duration);
        
        result
    }
    
    pub async fn get_hotspots(&self, limit: usize) -> Vec<FunctionProfile> {
        let profiles = self.function_times.read().await;
        let mut sorted: Vec<_> = profiles.values().cloned().collect();
        sorted.sort_by_key(|p| std::cmp::Reverse(p.total_time));
        sorted.truncate(limit);
        sorted
    }
    
    pub async fn analyze_performance(&self) -> PerformanceAnalysis {
        let samples = self.samples.read().await;
        let profiles = self.function_times.read().await;
        
        let avg_cpu = if !samples.is_empty() {
            samples.iter().map(|s| s.cpu_usage_percent).sum::<f32>() / samples.len() as f32
        } else {
            0.0
        };
        
        let max_cpu = samples.iter().map(|s| s.cpu_usage_percent).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);
        
        let total_function_time: Duration = profiles.values().map(|p| p.total_time).sum();
        
        PerformanceAnalysis {
            avg_cpu_usage: avg_cpu,
            max_cpu_usage: max_cpu,
            total_profiled_time: total_function_time,
            optimization_opportunities: detect_optimization_opportunities(&profiles),
        }
    }
}

#[derive(Debug)]
pub struct PerformanceAnalysis {
    pub avg_cpu_usage: f32,
    pub max_cpu_usage: f32,
    pub total_profiled_time: Duration,
    pub optimization_opportunities: Vec<OptimizationOpportunity>,
}

#[derive(Debug, Clone)]
pub struct OptimizationOpportunity {
    pub function: String,
    pub issue: String,
    pub suggestion: String,
    pub potential_speedup: f32,
}

fn detect_optimization_opportunities(profiles: &HashMap<String, FunctionProfile>) -> Vec<OptimizationOpportunity> {
    let mut opportunities = Vec::new();
    
    for profile in profiles.values() {
        // High frequency, short duration functions
        if profile.call_count > 1000 && profile.avg_time < Duration::from_micros(100) {
            opportunities.push(OptimizationOpportunity {
                function: profile.name.clone(),
                issue: "High frequency micro-function".to_string(),
                suggestion: "Consider inlining or batching".to_string(),
                potential_speedup: 1.2,
            });
        }
        
        // Functions with high variance
        if profile.max_time > profile.avg_time * 10 {
            opportunities.push(OptimizationOpportunity {
                function: profile.name.clone(),
                issue: "High execution time variance".to_string(),
                suggestion: "Investigate edge cases or cache misses".to_string(),
                potential_speedup: 1.5,
            });
        }
        
        // Slow functions
        if profile.avg_time > Duration::from_millis(100) {
            opportunities.push(OptimizationOpportunity {
                function: profile.name.clone(),
                issue: "Slow average execution time".to_string(),
                suggestion: "Profile internally and optimize algorithm".to_string(),
                potential_speedup: 2.0,
            });
        }
    }
    
    opportunities
}

fn get_cpu_usage() -> f32 {
    // Simplified - would use actual system metrics
    rand::random::<f32>() * 100.0
}

fn get_thread_count() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}
