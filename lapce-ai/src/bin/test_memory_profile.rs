/// MEMORY PROFILING WITH LIVE API TESTING
/// Tests memory usage with Gemini and AWS Bedrock APIs

use std::sync::{Arc, atomic::{AtomicU64, AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use std::fs;
use anyhow::Result;
use colored::Colorize;
use tokio::task::JoinSet;

use lapce_ai_rust::ai_providers::{
    core_trait::{AiProvider, ChatRequest, ChatMessage, CompletionRequest},
    gemini_exact::{GeminiProvider, GeminiConfig},
    bedrock_exact::{BedrockProvider, BedrockConfig},
};

const CONCURRENT_REQUESTS: usize = 10;
const TOTAL_REQUESTS: usize = 100;
const SAMPLE_INTERVAL_MS: u64 = 100;

#[derive(Default)]
struct MemoryMetrics {
    baseline_kb: AtomicU64,
    current_kb: AtomicU64,
    peak_kb: AtomicU64,
    samples: AtomicU64,
    total_allocated_kb: AtomicU64,
}

impl MemoryMetrics {
    fn new() -> Arc<Self> {
        let metrics = Arc::new(Self::default());
        let baseline = get_rss_kb();
        metrics.baseline_kb.store(baseline, Ordering::Relaxed);
        metrics.current_kb.store(baseline, Ordering::Relaxed);
        metrics.peak_kb.store(baseline, Ordering::Relaxed);
        metrics
    }
    
    fn record_sample(&self) {
        let current = get_rss_kb();
        self.current_kb.store(current, Ordering::Relaxed);
        self.samples.fetch_add(1, Ordering::Relaxed);
        
        // Update peak
        let mut peak = self.peak_kb.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_kb.compare_exchange_weak(
                peak, current, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }
    }
    
    fn print_summary(&self, provider: &str) {
        let baseline = self.baseline_kb.load(Ordering::Relaxed);
        let peak = self.peak_kb.load(Ordering::Relaxed);
        let current = self.current_kb.load(Ordering::Relaxed);
        let samples = self.samples.load(Ordering::Relaxed);
        
        println!("\n{}", format!("📊 MEMORY PROFILE: {}", provider).bright_blue().bold());
        println!("{}", "=".repeat(50).bright_blue());
        println!("• Baseline Memory: {:.2} MB", baseline as f64 / 1024.0);
        println!("• Peak Memory: {:.2} MB", peak as f64 / 1024.0);
        println!("• Final Memory: {:.2} MB", current as f64 / 1024.0);
        println!("• Memory Growth: {:.2} MB", (peak - baseline) as f64 / 1024.0);
        println!("• Growth Rate: {:.2}%", 
            ((peak - baseline) as f64 / baseline as f64) * 100.0);
        println!("• Samples Taken: {}", samples);
        
        // Check against 8MB target
        let total_mb = (peak - baseline) as f64 / 1024.0;
        if total_mb < 8.0 {
            println!("• {} Meets < 8MB requirement", "✅".green());
        } else {
            println!("• {} Exceeds 8MB requirement ({:.2} MB)", "⚠️".yellow(), total_mb);
        }
    }
}

fn get_rss_kb() -> u64 {
    if let Ok(statm) = fs::read_to_string("/proc/self/statm") {
        let parts: Vec<&str> = statm.split_whitespace().collect();
        if parts.len() > 1 {
            let rss_pages = parts[1].parse::<u64>().unwrap_or(0);
            let page_size_kb = 4; // Usually 4KB on Linux
            return rss_pages * page_size_kb;
        }
    }
    0
}

async fn start_memory_monitor(
    metrics: Arc<MemoryMetrics>,
    stop_signal: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        while !stop_signal.load(Ordering::Relaxed) {
            metrics.record_sample();
            tokio::time::sleep(Duration::from_millis(SAMPLE_INTERVAL_MS)).await;
        }
    });
}

#[derive(Default)]
struct RequestMetrics {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    total_latency_ms: AtomicU64,
    tokens_processed: AtomicU64,
}

impl RequestMetrics {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    
    fn record_success(&self, latency_ms: u64, tokens: u64) {
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
        self.tokens_processed.fetch_add(tokens, Ordering::Relaxed);
    }
    
    fn record_failure(&self) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }
    
    fn print_summary(&self, duration: Duration) {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);
        let failed = self.failed_requests.load(Ordering::Relaxed);
        let total_latency = self.total_latency_ms.load(Ordering::Relaxed);
        let tokens = self.tokens_processed.load(Ordering::Relaxed);
        
        let avg_latency = if successful > 0 {
            total_latency / successful
        } else {
            0
        };
        
        let rps = total as f64 / duration.as_secs_f64();
        
        println!("\n{}", "📈 REQUEST METRICS".bright_cyan());
        println!("• Total Requests: {}", total);
        println!("• Successful: {} {}", successful, "✅".green());
        println!("• Failed: {} {}", failed, "❌".red());
        println!("• Requests/Second: {:.2}", rps);
        println!("• Avg Latency: {}ms", avg_latency);
        println!("• Tokens Processed: {}", tokens);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    
    println!("\n{}", "🔬 MEMORY PROFILING WITH LIVE APIS".bright_blue().bold());
    println!("{}", "=".repeat(60).bright_blue());
    
    // Test 1: Gemini Memory Profile
    println!("\n{}", "1️⃣ GEMINI PROVIDER MEMORY PROFILE".bright_cyan().bold());
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        profile_gemini(api_key).await?;
    } else {
        println!("   ⚠️ Skipping: GEMINI_API_KEY not found");
        println!("   Set with: export GEMINI_API_KEY=AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU");
    }
    
    // Test 2: AWS Bedrock Memory Profile
    println!("\n{}", "2️⃣ AWS BEDROCK TITAN MEMORY PROFILE".bright_cyan().bold());
    if let (Ok(access_key), Ok(secret_key)) = (
        std::env::var("AWS_ACCESS_KEY_ID"),
        std::env::var("AWS_SECRET_ACCESS_KEY")
    ) {
        profile_bedrock_titan(access_key, secret_key).await?;
    } else {
        println!("   ⚠️ Skipping: AWS credentials not found");
        println!("   Set with: export AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=...");
    }
    
    // Test 3: Memory Stress Test
    println!("\n{}", "3️⃣ MEMORY STRESS TEST".bright_cyan().bold());
    memory_stress_test().await?;
    
    // Final Report
    println!("\n{}", "=".repeat(60).bright_blue());
    println!("{}", "📊 MEMORY PROFILING COMPLETE".bright_blue().bold());
    println!("{}", "=".repeat(60).bright_blue());
    
    let final_rss = get_rss_kb();
    println!("• Final Process Memory: {:.2} MB", final_rss as f64 / 1024.0);
    
    if final_rss < 8192 { // 8MB in KB
        println!("\n{}", "✅ PASSES < 8MB MEMORY REQUIREMENT!".bright_green().bold());
    } else {
        println!("\n{}", "⚠️ EXCEEDS 8MB TARGET".bright_yellow().bold());
    }
    
    Ok(())
}

async fn profile_gemini(api_key: String) -> Result<()> {
    println!("   Profiling Gemini 2.5 Flash...");
    
    let memory_metrics = MemoryMetrics::new();
    let request_metrics = RequestMetrics::new();
    let stop_signal = Arc::new(AtomicBool::new(false));
    
    // Start memory monitoring
    start_memory_monitor(memory_metrics.clone(), stop_signal.clone()).await;
    
    // Initialize provider
    let config = GeminiConfig {
        api_key: api_key.clone(),
        base_url: Some("https://generativelanguage.googleapis.com".to_string()),
        default_model: Some("gemini-2.5-flash".to_string()),
        api_version: Some("v1beta".to_string()),
        timeout_ms: Some(30000),
        project_id: None,
        location: None,
    };
    
    let provider = Arc::new(GeminiProvider::new(config).await?);
    
    // Record after provider initialization
    memory_metrics.record_sample();
    
    let start = Instant::now();
    let mut tasks = JoinSet::new();
    
    // Launch concurrent workers
    for worker_id in 0..CONCURRENT_REQUESTS.min(5) {
        let provider = provider.clone();
        let request_metrics = request_metrics.clone();
        let memory_metrics = memory_metrics.clone();
        
        tasks.spawn(async move {
            for i in 0..(TOTAL_REQUESTS / 5) {
                // Sample memory before request
                memory_metrics.record_sample();
                
                let request = ChatRequest {
                    model: "gemini-2.5-flash".to_string(),
                    messages: vec![ChatMessage {
                        role: "user".to_string(),
                        content: Some(format!("What is {}+{}? Just the number.", worker_id, i)),
                        name: None,
                        function_call: None,
                        tool_calls: None,
                    }],
                    temperature: Some(0.0),
                    max_tokens: Some(10),
                    stream: Some(false),
                    top_p: None,
                    stop: None,
                    presence_penalty: None,
                    frequency_penalty: None,
                    user: None,
                    functions: None,
                    function_call: None,
                    tools: None,
                    tool_choice: None,
                    response_format: None,
                    seed: None,
                    logprobs: None,
                    top_logprobs: None,
                };
                
                let req_start = Instant::now();
                match provider.chat(request).await {
                    Ok(response) => {
                        let latency = req_start.elapsed().as_millis() as u64;
                        let tokens = response.usage.as_ref()
                            .map(|u| u.total_tokens).unwrap_or(0) as u64;
                        request_metrics.record_success(latency, tokens);
                    },
                    Err(_) => {
                        request_metrics.record_failure();
                    }
                }
                
                // Sample memory after request
                memory_metrics.record_sample();
                
                // Rate limit delay
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });
    }
    
    // Wait for all tasks
    while let Some(_) = tasks.join_next().await {}
    
    stop_signal.store(true, Ordering::Relaxed);
    let duration = start.elapsed();
    
    // Print results
    memory_metrics.print_summary("Gemini");
    request_metrics.print_summary(duration);
    
    Ok(())
}

async fn profile_bedrock_titan(access_key: String, secret_key: String) -> Result<()> {
    println!("   Profiling AWS Titan Text Express...");
    
    let memory_metrics = MemoryMetrics::new();
    let request_metrics = RequestMetrics::new();
    let stop_signal = Arc::new(AtomicBool::new(false));
    
    // Start memory monitoring
    start_memory_monitor(memory_metrics.clone(), stop_signal.clone()).await;
    
    // Initialize provider
    let config = BedrockConfig {
        access_key_id: access_key,
        secret_access_key: secret_key,
        region: "us-east-1".to_string(),
        default_model: Some("amazon.titan-text-express-v1".to_string()),
        timeout_ms: Some(30000),
        session_token: None,
        base_url: None,
    };
    
    let provider = Arc::new(BedrockProvider::new(config).await?);
    
    // Record after provider initialization
    memory_metrics.record_sample();
    
    let start = Instant::now();
    let mut tasks = JoinSet::new();
    
    // Launch concurrent workers
    for worker_id in 0..CONCURRENT_REQUESTS.min(5) {
        let provider = provider.clone();
        let request_metrics = request_metrics.clone();
        let memory_metrics = memory_metrics.clone();
        
        tasks.spawn(async move {
            for i in 0..(TOTAL_REQUESTS / 5) {
                // Sample memory before request
                memory_metrics.record_sample();
                
                let request = CompletionRequest {
                    model: "amazon.titan-text-express-v1".to_string(),
                    prompt: format!("Complete: Worker {} request {} equals", worker_id, i),
                    max_tokens: Some(10),
                    temperature: Some(0.0),
                    stream: Some(false),
                    top_p: None,
                    stop: None,
                    presence_penalty: None,
                    frequency_penalty: None,
                    logit_bias: None,
                    user: None,
                    suffix: None,
                    echo: None,
                    n: None,
                    best_of: None,
                };
                
                let req_start = Instant::now();
                match provider.complete(request).await {
                    Ok(_) => {
                        let latency = req_start.elapsed().as_millis() as u64;
                        request_metrics.record_success(latency, 10);
                    },
                    Err(_) => {
                        request_metrics.record_failure();
                    }
                }
                
                // Sample memory after request
                memory_metrics.record_sample();
                
                // Rate limit delay
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });
    }
    
    // Wait for all tasks
    while let Some(_) = tasks.join_next().await {}
    
    stop_signal.store(true, Ordering::Relaxed);
    let duration = start.elapsed();
    
    // Print results
    memory_metrics.print_summary("AWS Bedrock");
    request_metrics.print_summary(duration);
    
    Ok(())
}

async fn memory_stress_test() -> Result<()> {
    println!("   Running memory stress test...");
    
    let memory_metrics = MemoryMetrics::new();
    let stop_signal = Arc::new(AtomicBool::new(false));
    
    // Start memory monitoring
    start_memory_monitor(memory_metrics.clone(), stop_signal.clone()).await;
    
    println!("   • Creating large message buffers...");
    
    // Allocate various data structures
    let mut allocations = Vec::new();
    
    // 1. Large messages
    for i in 0..100 {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: Some("x".repeat(10000)), // 10KB per message
            name: None,
            function_call: None,
            tool_calls: None,
        };
        allocations.push(serde_json::to_string(&msg)?);
        
        if i % 10 == 0 {
            memory_metrics.record_sample();
        }
    }
    
    println!("   • Serializing/deserializing requests...");
    
    // 2. Request serialization
    for _ in 0..1000 {
        let req = ChatRequest {
            model: "test".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: Some("Test".to_string()),
                name: None,
                function_call: None,
                tool_calls: None,
            }],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: Some(false),
            top_p: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            functions: None,
            function_call: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            seed: None,
            logprobs: None,
            top_logprobs: None,
        };
        
        let json = serde_json::to_string(&req)?;
        let _: ChatRequest = serde_json::from_str(&json)?;
        memory_metrics.record_sample();
    }
    
    println!("   • Testing concurrent allocations...");
    
    // 3. Concurrent allocations
    let mut handles = vec![];
    for _ in 0..10 {
        let metrics = memory_metrics.clone();
        let handle = tokio::spawn(async move {
            let mut local_data = Vec::new();
            for _ in 0..100 {
                local_data.push(vec![0u8; 1024]); // 1KB allocations
                metrics.record_sample();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await?;
    }
    
    stop_signal.store(true, Ordering::Relaxed);
    
    // Print results
    memory_metrics.print_summary("Stress Test");
    
    // Clear allocations to test memory release
    allocations.clear();
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let final_kb = get_rss_kb();
    let baseline = memory_metrics.baseline_kb.load(Ordering::Relaxed);
    println!("\n• Memory after cleanup: {:.2} MB", final_kb as f64 / 1024.0);
    println!("• Memory released: {:.2} MB", 
        (memory_metrics.peak_kb.load(Ordering::Relaxed) - final_kb) as f64 / 1024.0);
    
    Ok(())
}
