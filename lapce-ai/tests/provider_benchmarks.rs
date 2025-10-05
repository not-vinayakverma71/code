/// Performance benchmarks for AI providers
use lapce_ai_rust::ai_providers::{
    provider_manager::ProviderManager,
    traits::{AIProvider, ChatMessage, ChatRequest},
};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;
use std::time::Duration;
use futures::StreamExt;

fn setup_provider_manager() -> ProviderManager {
    dotenv::dotenv().ok();
    ProviderManager::new()
}

fn benchmark_single_completion(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = setup_provider_manager();
    
    let mut group = c.benchmark_group("single_completion");
    group.measurement_time(Duration::from_secs(30));
    
    for provider in ["openai", "anthropic", "gemini"] {
        group.bench_with_input(
            BenchmarkId::from_parameter(provider),
            &provider,
            |b, &provider| {
                b.iter(|| {
                    rt.block_on(async {
                        let messages = vec![
                            ChatMessage {
                                role: "user".to_string(),
                                content: "Say 'test'".to_string(),
                            },
                        ];
                        
                        let request = ChatRequest {
                            model: get_model_for_provider(provider),
                            messages,
                            temperature: Some(0.0),
                            max_tokens: Some(5),
                            stream: false,
                        };
                        
                        let provider_impl = manager.get_provider(provider).await.unwrap();
                        let _ = provider_impl.chat(request).await;
                    });
                });
            },
        );
    }
    group.finish();
}

fn benchmark_streaming_completion(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = setup_provider_manager();
    
    let mut group = c.benchmark_group("streaming_completion");
    group.measurement_time(Duration::from_secs(30));
    
    for provider in ["openai", "anthropic"] {
        group.bench_with_input(
            BenchmarkId::from_parameter(provider),
            &provider,
            |b, &provider| {
                b.iter(|| {
                    rt.block_on(async {
                        let messages = vec![
                            ChatMessage {
                                role: "user".to_string(),
                                content: "Count to 3".to_string(),
                            },
                        ];
                        
                        let request = ChatRequest {
                            model: get_model_for_provider(provider),
                            messages,
                            temperature: Some(0.0),
                            max_tokens: Some(20),
                            stream: true,
                        };
                        
                        let provider_impl = manager.get_provider(provider).await.unwrap();
                        let mut stream = provider_impl.chat_stream(request).await.unwrap();
                        
                        let mut tokens = Vec::new();
                        while let Some(token) = stream.next().await {
                            if let Ok(t) = token {
                                tokens.push(t);
                            }
                        }
                        black_box(tokens);
                    });
                });
            },
        );
    }
    group.finish();
}

fn benchmark_parallel_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let manager = setup_provider_manager();
    
    let mut group = c.benchmark_group("parallel_requests");
    group.measurement_time(Duration::from_secs(60));
    
    for num_requests in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_requests),
            &num_requests,
            |b, &num_requests| {
                b.iter(|| {
                    rt.block_on(async {
                        let mut tasks = vec![];
                        
                        for _ in 0..num_requests {
                            let manager_clone = manager.clone();
                            let task = tokio::spawn(async move {
                                let messages = vec![
                                    ChatMessage {
                                        role: "user".to_string(),
                                        content: "Reply with 'ok'".to_string(),
                                    },
                                ];
                                
                                let request = ChatRequest {
                                    model: "gpt-3.5-turbo".to_string(),
                                    messages,
                                    temperature: Some(0.0),
                                    max_tokens: Some(5),
                                    stream: false,
                                };
                                
                                let provider = manager_clone.get_provider("openai").await.unwrap();
                                provider.chat(request).await
                            });
                            tasks.push(task);
                        }
                        
                        let results = futures::future::join_all(tasks).await;
                        black_box(results);
                    });
                });
            },
        );
    }
    group.finish();
}

fn benchmark_token_counting(c: &mut Criterion) {
    let texts = vec![
        "Short text",
        "Medium length text that contains more words and tokens to process",
        &"Very long text. ".repeat(100),
    ];
    
    let mut group = c.benchmark_group("token_counting");
    
    for (i, text) in texts.iter().enumerate() {
        let text_len = text.len();
        group.bench_with_input(
            BenchmarkId::new("tiktoken", text_len),
            text,
            |b, text| {
                b.iter(|| {
                    // Simulate token counting
                    let tokens = text.split_whitespace().count();
                    black_box(tokens);
                });
            },
        );
    }
    group.finish();
}

fn get_model_for_provider(provider: &str) -> String {
    match provider {
        "openai" => "gpt-3.5-turbo".to_string(),
        "anthropic" => "claude-3-haiku-20240307".to_string(),
        "gemini" => "gemini-pro".to_string(),
        _ => "default-model".to_string(),
    }
}

criterion_group!(
    benches,
    benchmark_single_completion,
    benchmark_streaming_completion,
    benchmark_parallel_requests,
    benchmark_token_counting
);
criterion_main!(benches);
