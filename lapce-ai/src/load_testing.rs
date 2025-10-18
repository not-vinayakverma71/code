/// Load Testing with k6 equivalent - Day 40
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use anyhow::Result;

pub struct LoadTestConfig {
    pub target_rps: u64,
    pub duration: Duration,
    pub concurrent_users: usize,
    pub ramp_up_time: Duration,
}

pub struct LoadTestResults {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub requests_per_second: f64,
}

pub struct LoadTester {
    config: LoadTestConfig,
    total_requests: Arc<AtomicU64>,
    successful_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    latencies: Arc<RwLock<Vec<Duration>>>,
}

impl LoadTester {
    pub fn new(config: LoadTestConfig) -> Self {
        Self {
            config,
            total_requests: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            latencies: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn run<F, Fut>(&self, test_fn: F) -> Result<LoadTestResults>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let start = Instant::now();
        let semaphore = Arc::new(Semaphore::new(self.config.concurrent_users));
        let test_fn = Arc::new(test_fn);
        
        let mut handles = vec![];
        let request_interval = Duration::from_micros((1_000_000 / self.config.target_rps) as u64);
        
        while start.elapsed() < self.config.duration {
            let permit = semaphore.clone().acquire_owned().await?;
            let test_fn = test_fn.clone();
            let total = self.total_requests.clone();
            let success = self.successful_requests.clone();
            let failed = self.failed_requests.clone();
            let latencies = self.latencies.clone();
            
            handles.push(tokio::spawn(async move {
                let req_start = Instant::now();
                total.fetch_add(1, Ordering::Relaxed);
                
                match test_fn().await {
                    Ok(_) => {
                        success.fetch_add(1, Ordering::Relaxed);
                        let latency = req_start.elapsed();
                        latencies.write().await.push(latency);
                    }
                    Err(_) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
                
                drop(permit);
            }));
            
            tokio::time::sleep(request_interval).await;
        }
        
        for handle in handles {
            let _ = handle.await;
        }
        
        self.calculate_results(start.elapsed()).await
    }
    
    async fn calculate_results(&self, duration: Duration) -> Result<LoadTestResults> {
        let mut latencies = self.latencies.write().await;
        latencies.sort_unstable();
        
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);
        let failed = self.failed_requests.load(Ordering::Relaxed);
        
        let avg_latency = if !latencies.is_empty() {
            latencies.iter().map(|d| d.as_millis() as f64).sum::<f64>() / latencies.len() as f64
        } else {
            0.0
        };
        
        let p50 = percentile(&latencies, 50.0);
        let p95 = percentile(&latencies, 95.0);
        let p99 = percentile(&latencies, 99.0);
        
        Ok(LoadTestResults {
            total_requests: total,
            successful_requests: successful,
            failed_requests: failed,
            avg_latency_ms: avg_latency,
            p50_latency_ms: p50.as_millis() as f64,
            p95_latency_ms: p95.as_millis() as f64,
            p99_latency_ms: p99.as_millis() as f64,
            requests_per_second: total as f64 / duration.as_secs_f64(),
        })
    }
}

fn percentile(sorted_data: &[Duration], p: f64) -> Duration {
    if sorted_data.is_empty() {
        return Duration::ZERO;
    }
    
    let idx = ((sorted_data.len() as f64 - 1.0) * p / 100.0) as usize;
    sorted_data[idx]
}
