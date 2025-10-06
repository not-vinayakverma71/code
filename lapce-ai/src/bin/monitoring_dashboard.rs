/// REAL-TIME MONITORING DASHBOARD FOR LAPCE AI
/// Provides live metrics and system health monitoring

use std::sync::{Arc, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::{HashMap, VecDeque};
use std::fs;
use anyhow::Result;
use colored::Colorize;
use tokio::sync::RwLock;

const REFRESH_INTERVAL_MS: u64 = 1000;
const HISTORY_SIZE: usize = 60; // 1 minute of history

#[derive(Debug, Clone)]
struct SystemMetrics {
    timestamp: u64,
    cpu_usage: f32,
    memory_mb: f32,
    requests_per_sec: f32,
    active_connections: u32,
    error_rate: f32,
}

#[derive(Debug)]
struct ProviderMetrics {
    name: String,
    requests_total: AtomicU64,
    requests_failed: AtomicU64,
    avg_latency_ms: AtomicU64,
    is_healthy: AtomicBool,
    last_error: RwLock<Option<String>>,
}

impl ProviderMetrics {
    fn new(name: String) -> Self {
        Self {
            name,
            requests_total: AtomicU64::new(0),
            requests_failed: AtomicU64::new(0),
            avg_latency_ms: AtomicU64::new(0),
            is_healthy: AtomicBool::new(true),
            last_error: RwLock::new(None),
        }
    }
    
    fn success_rate(&self) -> f32 {
        let total = self.requests_total.load(Ordering::Relaxed);
        if total == 0 {
            return 100.0;
        }
        let failed = self.requests_failed.load(Ordering::Relaxed);
        ((total - failed) as f32 / total as f32) * 100.0
    }
}

struct Dashboard {
    system_history: Arc<RwLock<VecDeque<SystemMetrics>>>,
    providers: Arc<HashMap<String, Arc<ProviderMetrics>>>,
    start_time: Instant,
    is_running: Arc<AtomicBool>,
}

impl Dashboard {
    fn new() -> Self {
        let mut providers = HashMap::new();
        
        // Initialize provider metrics
        for name in &["OpenAI", "Anthropic", "Gemini", "AWS Bedrock", "Azure", "xAI", "Vertex AI"] {
            providers.insert(
                name.to_string(),
                Arc::new(ProviderMetrics::new(name.to_string()))
            );
        }
        
        Self {
            system_history: Arc::new(RwLock::new(VecDeque::with_capacity(HISTORY_SIZE))),
            providers: Arc::new(providers),
            start_time: Instant::now(),
            is_running: Arc::new(AtomicBool::new(true)),
        }
    }
    
    async fn start(&self) {
        // Start metric collection
        self.start_metric_collector().await;
        
        // Start dashboard renderer
        self.start_renderer().await;
    }
    
    async fn start_metric_collector(&self) {
        let history = self.system_history.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            while is_running.load(Ordering::Relaxed) {
                let metrics = collect_system_metrics();
                
                let mut hist = history.write().await;
                if hist.len() >= HISTORY_SIZE {
                    hist.pop_front();
                }
                hist.push_back(metrics);
                
                tokio::time::sleep(Duration::from_millis(REFRESH_INTERVAL_MS)).await;
            }
        });
    }
    
    async fn start_renderer(&self) {
        let history = self.system_history.clone();
        let providers = self.providers.clone();
        let start_time = self.start_time;
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            while is_running.load(Ordering::Relaxed) {
                clear_screen();
                render_dashboard(&history, &providers, start_time).await;
                tokio::time::sleep(Duration::from_millis(REFRESH_INTERVAL_MS)).await;
            }
        });
    }
    
    async fn simulate_activity(&self) {
        // Simulate some provider activity
        let providers = self.providers.clone();
        
        for _ in 0..100 {
            for (_, provider) in providers.iter() {
                // Simulate requests
                provider.requests_total.fetch_add(rand::random::<u64>() % 10, Ordering::Relaxed);
                
                // Simulate some failures
                if rand::random::<u8>() % 10 == 0 {
                    provider.requests_failed.fetch_add(1, Ordering::Relaxed);
                    provider.is_healthy.store(false, Ordering::Relaxed);
                } else {
                    provider.is_healthy.store(true, Ordering::Relaxed);
                }
                
                // Update latency
                provider.avg_latency_ms.store(
                    50 + rand::random::<u64>() % 200,
                    Ordering::Relaxed
                );
            }
            
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}

fn collect_system_metrics() -> SystemMetrics {
    SystemMetrics {
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        cpu_usage: get_cpu_usage(),
        memory_mb: get_memory_mb(),
        requests_per_sec: rand::random::<f32>() * 100.0, // Simulated
        active_connections: rand::random::<u32>() % 50,  // Simulated
        error_rate: rand::random::<f32>() * 5.0,        // Simulated
    }
}

fn get_cpu_usage() -> f32 {
    // Simple CPU usage approximation
    if let Ok(loadavg) = fs::read_to_string("/proc/loadavg") {
        if let Some(first) = loadavg.split_whitespace().next() {
            if let Ok(load) = first.parse::<f32>() {
                return (load * 100.0).min(100.0);
            }
        }
    }
    0.0
}

fn get_memory_mb() -> f32 {
    if let Ok(statm) = fs::read_to_string("/proc/self/statm") {
        let parts: Vec<&str> = statm.split_whitespace().collect();
        if parts.len() > 1 {
            let rss_pages = parts[1].parse::<f32>().unwrap_or(0.0);
            return rss_pages * 4.0 / 1024.0; // Convert pages to MB
        }
    }
    0.0
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}

async fn render_dashboard(
    history: &Arc<RwLock<VecDeque<SystemMetrics>>>,
    providers: &Arc<HashMap<String, Arc<ProviderMetrics>>>,
    start_time: Instant,
) {
    let uptime = start_time.elapsed();
    let hist = history.read().await;
    let latest = hist.back();
    
    // Header
    println!("{}", "╔═══════════════════════════════════════════════════════════════════╗".bright_blue());
    println!("{}", "║          LAPCE AI MONITORING DASHBOARD v1.0                      ║".bright_blue().bold());
    println!("{}", "╚═══════════════════════════════════════════════════════════════════╝".bright_blue());
    
    // System Overview
    println!("\n{}", "📊 SYSTEM METRICS".bright_cyan().bold());
    println!("{}", "─".repeat(70).bright_cyan());
    
    if let Some(metrics) = latest {
        println!("  CPU Usage:      {} {}",
            format!("{:5.1}%", metrics.cpu_usage).yellow(),
            render_bar(metrics.cpu_usage as usize, 100, 30)
        );
        println!("  Memory:         {} MB",
            format!("{:6.1}", metrics.memory_mb).green()
        );
        println!("  Requests/sec:   {}",
            format!("{:6.1}", metrics.requests_per_sec).cyan()
        );
        println!("  Connections:    {}",
            format!("{:6}", metrics.active_connections).magenta()
        );
        println!("  Error Rate:     {}%",
            format!("{:5.1}", metrics.error_rate).red()
        );
    }
    
    println!("  Uptime:         {}",
        format_duration(uptime).bright_white()
    );
    
    // Performance Graph
    println!("\n{}", "📈 PERFORMANCE HISTORY (1 min)".bright_cyan().bold());
    println!("{}", "─".repeat(70).bright_cyan());
    render_graph(&hist).await;
    
    // Provider Status
    println!("\n{}", "🔌 PROVIDER STATUS".bright_cyan().bold());
    println!("{}", "─".repeat(70).bright_cyan());
    println!("  {:<15} {:>10} {:>10} {:>10} {:>10} {:>8}",
        "Provider", "Requests", "Failed", "Success%", "Latency", "Status"
    );
    println!("  {}", "─".repeat(70));
    
    for (name, metrics) in providers.iter() {
        let total = metrics.requests_total.load(Ordering::Relaxed);
        let failed = metrics.requests_failed.load(Ordering::Relaxed);
        let latency = metrics.avg_latency_ms.load(Ordering::Relaxed);
        let is_healthy = metrics.is_healthy.load(Ordering::Relaxed);
        
        let status = if is_healthy {
            "✅ OK".green()
        } else {
            "❌ ERR".red()
        };
        
        let success_rate = metrics.success_rate();
        let rate_color = if success_rate > 95.0 {
            success_rate.to_string().green()
        } else if success_rate > 80.0 {
            success_rate.to_string().yellow()
        } else {
            success_rate.to_string().red()
        };
        
        println!("  {:<15} {:>10} {:>10} {:>9.1}% {:>9}ms {}",
            name,
            total,
            failed,
            rate_color,
            latency,
            status
        );
    }
    
    // Alerts
    println!("\n{}", "⚠️  ALERTS".bright_yellow().bold());
    println!("{}", "─".repeat(70).bright_yellow());
    
    let alerts = check_alerts(latest, providers).await;
    if alerts.is_empty() {
        println!("  {}", "✅ No active alerts".green());
    } else {
        for alert in alerts {
            println!("  • {}", alert.red());
        }
    }
    
    // Footer
    println!("\n{}", "─".repeat(70).bright_blue());
    println!("{} {} | {} {}",
        "💡".bright_white(),
        "Press Ctrl+C to exit".bright_white(),
        "🔄".bright_white(),
        format!("Refreshing every {}ms", REFRESH_INTERVAL_MS).bright_white()
    );
}

fn render_bar(value: usize, max: usize, width: usize) -> String {
    let filled = (value * width / max).min(width);
    let empty = width - filled;
    
    let bar = format!("{}{}",
        "█".repeat(filled).bright_green(),
        "░".repeat(empty).bright_black()
    );
    
    format!("[{}]", bar)
}

async fn render_graph(history: &VecDeque<SystemMetrics>) {
    if history.is_empty() {
        println!("  No data yet...");
        return;
    }
    
    let max_height = 10;
    let width = history.len().min(60);
    
    // Create CPU usage graph
    let cpu_values: Vec<usize> = history.iter()
        .rev()
        .take(width)
        .map(|m| (m.cpu_usage as usize * max_height / 100).min(max_height))
        .collect();
    
    for row in (0..=max_height).rev() {
        print!("  {:3}% │", row * 10);
        for &value in &cpu_values {
            if value >= row {
                print!("{}", "▓".bright_green());
            } else {
                print!(" ");
            }
        }
        println!();
    }
    
    print!("       └");
    println!("{}", "─".repeat(width));
    println!("        {}{}",
        "└── Time (seconds ago) ──┘".bright_black(),
        format!(" ({} samples)", width).bright_black()
    );
}

async fn check_alerts(
    latest: Option<&SystemMetrics>,
    providers: &HashMap<String, Arc<ProviderMetrics>>,
) -> Vec<String> {
    let mut alerts = Vec::new();
    
    if let Some(metrics) = latest {
        if metrics.cpu_usage > 80.0 {
            alerts.push(format!("High CPU usage: {:.1}%", metrics.cpu_usage));
        }
        if metrics.memory_mb > 100.0 {
            alerts.push(format!("High memory usage: {:.1} MB", metrics.memory_mb));
        }
        if metrics.error_rate > 5.0 {
            alerts.push(format!("High error rate: {:.1}%", metrics.error_rate));
        }
    }
    
    for (name, provider) in providers {
        if !provider.is_healthy.load(Ordering::Relaxed) {
            alerts.push(format!("{} provider is unhealthy", name));
        }
        if provider.success_rate() < 80.0 {
            alerts.push(format!("{} has low success rate: {:.1}%", 
                name, provider.success_rate()));
        }
    }
    
    alerts
}

fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

// Implement a basic rand function for simulation
mod rand {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    pub fn random<T>() -> T 
    where T: From<u8> + std::ops::Rem<Output = T>
    {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        T::from((nanos % 256) as u8)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("{}", "Starting Monitoring Dashboard...".bright_green().bold());
    
    let dashboard = Dashboard::new();
    
    // Start dashboard components
    dashboard.start().await;
    
    // Simulate some activity (in production, this would be real metrics)
    dashboard.simulate_activity().await;
    
    // Keep running
    tokio::signal::ctrl_c().await?;
    
    dashboard.is_running.store(false, Ordering::Relaxed);
    println!("\n{}", "Dashboard stopped.".bright_red().bold());
    
    Ok(())
}
