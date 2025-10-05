/// Command-line tool for testing AI providers
use anyhow::Result;
use clap::{Parser, Subcommand};
use lapce_ai_rust::ai_providers::{
    provider_manager::ProviderManager,
    traits::{ChatMessage, ChatRequest},
};
use std::time::Instant;
use futures::StreamExt;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[command(name = "test-providers")]
#[command(about = "Test AI provider integrations", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Test a specific provider
    Test {
        /// Provider name (openai, anthropic, gemini, azure, vertex, openrouter, bedrock)
        provider: String,
        
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
        
        /// Test streaming mode
        #[arg(short, long, default_value_t = false)]
        stream: bool,
        
        /// Custom prompt to test
        #[arg(short, long)]
        prompt: Option<String>,
    },
    
    /// Test all providers
    TestAll {
        /// Run quick tests only
        #[arg(short, long, default_value_t = false)]
        quick: bool,
    },
    
    /// Check health of all providers
    Health,
    
    /// Benchmark provider performance
    Benchmark {
        /// Provider to benchmark
        provider: Option<String>,
        
        /// Number of iterations
        #[arg(short = 'n', long, default_value_t = 10)]
        iterations: usize,
    },
    
    /// Interactive testing mode
    Interactive,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Test { provider, model, stream, prompt } => {
            test_provider(&provider, model, stream, prompt).await?;
        }
        Commands::TestAll { quick } => {
            test_all_providers(quick).await?;
        }
        Commands::Health => {
            check_health().await?;
        }
        Commands::Benchmark { provider, iterations } => {
            run_benchmark(provider, iterations).await?;
        }
        Commands::Interactive => {
            interactive_mode().await?;
        }
    }
    
    Ok(())
}

async fn test_provider(
    provider_name: &str,
    model: Option<String>,
    stream: bool,
    prompt: Option<String>,
) -> Result<()> {
    println!("{}", format!("🧪 Testing {} provider", provider_name).cyan().bold());
    
    let manager = ProviderManager::new();
    let provider = manager.get_provider(provider_name).await?;
    
    let model_name = model.unwrap_or_else(|| get_default_model(provider_name));
    let test_prompt = prompt.unwrap_or_else(|| "What is 2+2? Reply with just the number.".to_string());
    
    println!("  📝 Model: {}", model_name);
    println!("  💬 Prompt: {}", test_prompt);
    println!("  📡 Streaming: {}", if stream { "Yes" } else { "No" });
    
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: test_prompt,
        },
    ];
    
    let request = ChatRequest {
        model: model_name,
        messages,
        temperature: Some(0.7),
        max_tokens: Some(100),
        stream,
    };
    
    let start = Instant::now();
    
    if stream {
        println!("\n  📤 Streaming response:");
        print!("  ");
        
        let mut stream = provider.chat_stream(request).await?;
        let mut full_response = String::new();
        
        while let Some(token_result) = stream.next().await {
            match token_result {
                Ok(token) => {
                    print!("{}", token.content.yellow());
                    full_response.push_str(&token.content);
                }
                Err(e) => {
                    println!("\n  ❌ Stream error: {}", e.to_string().red());
                    return Err(e);
                }
            }
        }
        println!("\n");
        
        let duration = start.elapsed();
        println!("  ✅ {} completed in {:.2}s", "Stream".green(), duration.as_secs_f64());
        println!("  📊 Response length: {} chars", full_response.len());
    } else {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        spinner.set_message("Waiting for response...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        let response = provider.chat(request).await?;
        spinner.finish_and_clear();
        
        let duration = start.elapsed();
        
        println!("\n  📤 Response:");
        println!("  {}", response.content.yellow());
        println!("\n  ✅ {} completed in {:.2}s", "Request".green(), duration.as_secs_f64());
        println!("  📊 Response length: {} chars", response.content.len());
        if let Some(usage) = response.usage {
            println!("  🔢 Tokens - Prompt: {}, Completion: {}, Total: {}", 
                usage.prompt_tokens, 
                usage.completion_tokens,
                usage.total_tokens
            );
        }
    }
    
    Ok(())
}

async fn test_all_providers(quick: bool) -> Result<()> {
    println!("{}", "🚀 Testing all providers".cyan().bold());
    
    let providers = vec![
        "openai", "anthropic", "gemini", "azure", 
        "vertex", "openrouter", "bedrock"
    ];
    
    let mut results = Vec::new();
    
    for provider in providers {
        println!("\n{}", format!("Testing {}...", provider).blue());
        
        let start = Instant::now();
        let result = if quick {
            quick_test(provider).await
        } else {
            comprehensive_test(provider).await
        };
        
        let duration = start.elapsed();
        
        match result {
            Ok(_) => {
                println!("  ✅ {} passed ({:.2}s)", provider.green(), duration.as_secs_f64());
                results.push((provider, true, duration));
            }
            Err(e) => {
                println!("  ❌ {} failed: {}", provider.red(), e);
                results.push((provider, false, duration));
            }
        }
    }
    
    // Print summary
    println!("\n{}", "="repeat(60).blue());
    println!("{}", "📊 Test Summary".cyan().bold());
    println!("{}", "="repeat(60).blue());
    
    let passed = results.iter().filter(|(_, success, _)| *success).count();
    let total = results.len();
    
    for (provider, success, duration) in results {
        let status = if success { "✅ PASS".green() } else { "❌ FAIL".red() };
        println!("  {} {} ({:.2}s)", provider, status, duration.as_secs_f64());
    }
    
    println!("\n  {} {}/{} providers passed", 
        "Overall:".bold(),
        passed,
        total
    );
    
    Ok(())
}

async fn quick_test(provider: &str) -> Result<()> {
    let manager = ProviderManager::new();
    let provider_impl = manager.get_provider(provider).await?;
    
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "Say 'test'".to_string(),
        },
    ];
    
    let request = ChatRequest {
        model: get_default_model(provider),
        messages,
        temperature: Some(0.0),
        max_tokens: Some(10),
        stream: false,
    };
    
    let response = provider_impl.chat(request).await?;
    
    if response.content.to_lowercase().contains("test") {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Invalid response: {}", response.content))
    }
}

async fn comprehensive_test(provider: &str) -> Result<()> {
    let manager = ProviderManager::new();
    let provider_impl = manager.get_provider(provider).await?;
    
    // Test 1: Basic completion
    println!("    Testing basic completion...");
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "What is 2+2?".to_string(),
        },
    ];
    
    let request = ChatRequest {
        model: get_default_model(provider),
        messages: messages.clone(),
        temperature: Some(0.0),
        max_tokens: Some(50),
        stream: false,
    };
    
    let response = provider_impl.chat(request).await?;
    if !response.content.contains("4") {
        return Err(anyhow::anyhow!("Basic math test failed"));
    }
    
    // Test 2: Streaming
    if supports_streaming(provider) {
        println!("    Testing streaming...");
        let request = ChatRequest {
            model: get_default_model(provider),
            messages,
            temperature: Some(0.0),
            max_tokens: Some(50),
            stream: true,
        };
        
        let mut stream = provider_impl.chat_stream(request).await?;
        let mut token_count = 0;
        
        while let Some(token_result) = stream.next().await {
            token_result?;
            token_count += 1;
        }
        
        if token_count == 0 {
            return Err(anyhow::anyhow!("No tokens received in stream"));
        }
    }
    
    // Test 3: Multi-turn conversation
    println!("    Testing conversation...");
    let messages = vec![
        ChatMessage {
            role: "user".to_string(),
            content: "My name is Bob".to_string(),
        },
        ChatMessage {
            role: "assistant".to_string(),
            content: "Hello Bob!".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: "What's my name?".to_string(),
        },
    ];
    
    let request = ChatRequest {
        model: get_default_model(provider),
        messages,
        temperature: Some(0.0),
        max_tokens: Some(50),
        stream: false,
    };
    
    let response = provider_impl.chat(request).await?;
    if !response.content.to_lowercase().contains("bob") {
        return Err(anyhow::anyhow!("Conversation memory test failed"));
    }
    
    Ok(())
}

async fn check_health() -> Result<()> {
    println!("{}", "🏥 Checking provider health".cyan().bold());
    
    let manager = ProviderManager::new();
    let providers = vec![
        "openai", "anthropic", "gemini", "azure",
        "vertex", "openrouter", "bedrock"
    ];
    
    for provider_name in providers {
        print!("  {} ", provider_name);
        
        match manager.get_provider(provider_name).await {
            Ok(provider) => {
                match provider.health_check().await {
                    Ok(status) => {
                        println!("{} {}", "✅".green(), status);
                    }
                    Err(e) => {
                        println!("{} Unhealthy: {}", "⚠️".yellow(), e);
                    }
                }
            }
            Err(e) => {
                println!("{} Not available: {}", "❌".red(), e);
            }
        }
    }
    
    Ok(())
}

async fn run_benchmark(provider: Option<String>, iterations: usize) -> Result<()> {
    let providers = if let Some(p) = provider {
        vec![p]
    } else {
        vec![
            "openai".to_string(),
            "anthropic".to_string(),
            "gemini".to_string(),
        ]
    };
    
    println!("{}", format!("📊 Running {} iterations per provider", iterations).cyan().bold());
    
    let manager = ProviderManager::new();
    
    for provider_name in providers {
        println!("\n{}", format!("Benchmarking {}...", provider_name).blue());
        
        let provider = match manager.get_provider(&provider_name).await {
            Ok(p) => p,
            Err(e) => {
                println!("  ❌ Failed to get provider: {}", e);
                continue;
            }
        };
        
        let mut latencies = Vec::new();
        let mut errors = 0;
        
        let pb = ProgressBar::new(iterations as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        
        for _ in 0..iterations {
            let messages = vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: "Reply with 'ok'".to_string(),
                },
            ];
            
            let request = ChatRequest {
                model: get_default_model(&provider_name),
                messages,
                temperature: Some(0.0),
                max_tokens: Some(5),
                stream: false,
            };
            
            let start = Instant::now();
            match provider.chat(request).await {
                Ok(_) => {
                    latencies.push(start.elapsed().as_millis());
                }
                Err(_) => {
                    errors += 1;
                }
            }
            pb.inc(1);
        }
        
        pb.finish_and_clear();
        
        if !latencies.is_empty() {
            latencies.sort_unstable();
            let min = latencies[0];
            let max = latencies[latencies.len() - 1];
            let median = latencies[latencies.len() / 2];
            let mean: u128 = latencies.iter().sum::<u128>() / latencies.len() as u128;
            let p95 = latencies[(latencies.len() * 95) / 100];
            let p99 = latencies[(latencies.len() * 99) / 100];
            
            println!("  📈 Latency (ms):");
            println!("    Min: {}", min);
            println!("    Median: {}", median);
            println!("    Mean: {}", mean);
            println!("    P95: {}", p95);
            println!("    P99: {}", p99);
            println!("    Max: {}", max);
            
            if errors > 0 {
                println!("  ⚠️  Errors: {}/{}", errors, iterations);
            } else {
                println!("  ✅ Success rate: 100%");
            }
        } else {
            println!("  ❌ All requests failed");
        }
    }
    
    Ok(())
}

async fn interactive_mode() -> Result<()> {
    use std::io::{self, Write};
    
    println!("{}", "🎮 Interactive Provider Testing Mode".cyan().bold());
    println!("Type 'help' for commands, 'quit' to exit\n");
    
    let manager = ProviderManager::new();
    let mut current_provider = "openai".to_string();
    let mut current_model = get_default_model(&current_provider);
    
    loop {
        print!("{} > ", format!("[{}]", current_provider).blue());
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        match input {
            "quit" | "exit" => break,
            "help" => print_help(),
            cmd if cmd.starts_with("provider ") => {
                let provider = cmd.strip_prefix("provider ").unwrap();
                current_provider = provider.to_string();
                current_model = get_default_model(&current_provider);
                println!("  ✅ Switched to {}", current_provider);
            }
            cmd if cmd.starts_with("model ") => {
                current_model = cmd.strip_prefix("model ").unwrap().to_string();
                println!("  ✅ Using model: {}", current_model);
            }
            "list" => {
                println!("  Available providers:");
                for p in ["openai", "anthropic", "gemini", "azure", "vertex", "openrouter", "bedrock"] {
                    println!("    - {}", p);
                }
            }
            "status" => {
                println!("  Provider: {}", current_provider);
                println!("  Model: {}", current_model);
            }
            prompt if !prompt.is_empty() => {
                match manager.get_provider(&current_provider).await {
                    Ok(provider) => {
                        let messages = vec![
                            ChatMessage {
                                role: "user".to_string(),
                                content: prompt.to_string(),
                            },
                        ];
                        
                        let request = ChatRequest {
                            model: current_model.clone(),
                            messages,
                            temperature: Some(0.7),
                            max_tokens: Some(500),
                            stream: false,
                        };
                        
                        print!("  ");
                        let spinner = ProgressBar::new_spinner();
                        spinner.set_style(
                            ProgressStyle::default_spinner()
                                .template("{spinner:.green} Thinking...")
                                .unwrap(),
                        );
                        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
                        
                        match provider.chat(request).await {
                            Ok(response) => {
                                spinner.finish_and_clear();
                                println!("  {}", response.content.yellow());
                            }
                            Err(e) => {
                                spinner.finish_and_clear();
                                println!("  ❌ Error: {}", e.to_string().red());
                            }
                        }
                    }
                    Err(e) => {
                        println!("  ❌ Failed to get provider: {}", e);
                    }
                }
            }
            _ => {}
        }
    }
    
    println!("\n👋 Goodbye!");
    Ok(())
}

fn print_help() {
    println!("  Commands:");
    println!("    help                 - Show this help");
    println!("    provider <name>      - Switch provider");
    println!("    model <name>         - Set model");
    println!("    list                 - List providers");
    println!("    status              - Show current settings");
    println!("    <any text>          - Send prompt to current provider");
    println!("    quit/exit           - Exit interactive mode");
}

fn get_default_model(provider: &str) -> String {
    match provider {
        "openai" => "gpt-3.5-turbo",
        "anthropic" => "claude-3-haiku-20240307",
        "gemini" => "gemini-pro",
        "azure" => "gpt-35-turbo",
        "vertex" => "gemini-pro",
        "openrouter" => "openai/gpt-3.5-turbo",
        "bedrock" => "anthropic.claude-instant-v1",
        _ => "default-model",
    }.to_string()
}

fn supports_streaming(provider: &str) -> bool {
    matches!(provider, "openai" | "anthropic" | "gemini" | "azure" | "openrouter")
}
