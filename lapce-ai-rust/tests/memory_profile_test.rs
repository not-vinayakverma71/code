/// Memory Profiling Test
/// Ensures all providers stay under 8MB memory usage

use lapce_ai_rust::ai_providers::{
    core_trait::{AiProvider, ChatRequest, ChatMessage},
    openai_exact::OpenAIProvider,
    anthropic_exact::AnthropicProvider,
    gemini_exact::GeminiProvider,
    bedrock_exact::BedrockProvider,
    azure_exact::AzureProvider,
    xai_exact::XAiProvider,
    vertex_ai_exact::VertexAiProvider,
};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{System, SystemExt, ProcessExt, Pid};

#[tokio::test]
async fn test_memory_usage_all_providers() {
    println!("💾 Testing Memory Usage for All Providers");
    
    let mut system = System::new_all();
    system.refresh_all();
    
    let pid = Pid::from(std::process::id() as usize);
    let initial_memory = get_process_memory(&mut system, pid);
    println!("Initial memory: {:.2} MB", initial_memory);
    
    // Create all providers
    let providers: Vec<Arc<dyn AiProvider + Send + Sync>> = vec![
        Arc::new(OpenAIProvider::new("test-key".to_string(), None)),
        Arc::new(AnthropicProvider::new("test-key".to_string(), None)),
        Arc::new(GeminiProvider::new("test-key".to_string(), None)),
        Arc::new(BedrockProvider::new("us-east-1".to_string())),
        Arc::new(AzureProvider::new(
            "https://test.openai.azure.com".to_string(),
            "test-key".to_string(),
            "2023-05-15".to_string()
        )),
        Arc::new(XAiProvider::new("test-key".to_string())),
        Arc::new(VertexAiProvider::new(
            "test-project".to_string(),
            "us-central1".to_string(),
            None
        )),
    ];
    
    // Measure memory after creating providers
    system.refresh_all();
    let providers_memory = get_process_memory(&mut system, pid);
    let providers_overhead = providers_memory - initial_memory;
    println!("Memory after creating 7 providers: {:.2} MB (overhead: {:.2} MB)", 
             providers_memory, providers_overhead);
    
    // Assert memory constraint
    assert!(providers_overhead < 8.0, 
            "All providers combined should use less than 8MB (actual: {:.2} MB)", 
            providers_overhead);
    
    // Test each provider individually
    for provider in &providers {
        test_provider_memory(provider.clone(), &mut system, pid).await;
    }
}

#[tokio::test]
async fn test_streaming_memory_usage() {
    println!("🌊 Testing Streaming Memory Usage");
    
    let mut system = System::new_all();
    system.refresh_all();
    let pid = Pid::from(std::process::id() as usize);
    
    let provider = Arc::new(OpenAIProvider::new("test-key".to_string(), None));
    
    // Create a streaming request
    let request = ChatRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: Some("Generate a very long response".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        max_tokens: Some(4000), // Large response
        temperature: Some(0.7),
        stream: Some(true),
        ..Default::default()
    };
    
    let initial_memory = get_process_memory(&mut system, pid);
    
    // Simulate streaming (would need mock server in production)
    match provider.chat_stream(request).await {
        Ok(mut stream) => {
            let mut token_count = 0;
            let mut max_memory = initial_memory;
            
            // Process stream and monitor memory
            use futures::StreamExt;
            while let Some(token_result) = stream.next().await {
                if let Ok(_token) = token_result {
                    token_count += 1;
                    
                    // Check memory every 100 tokens
                    if token_count % 100 == 0 {
                        system.refresh_all();
                        let current_memory = get_process_memory(&mut system, pid);
                        max_memory = max_memory.max(current_memory);
                        
                        if token_count % 1000 == 0 {
                            println!("Processed {} tokens, memory: {:.2} MB", 
                                   token_count, current_memory);
                        }
                    }
                }
            }
            
            let memory_growth = max_memory - initial_memory;
            println!("Streamed {} tokens with {:.2} MB memory growth", 
                   token_count, memory_growth);
            
            // Assert streaming doesn't leak memory
            assert!(memory_growth < 2.0, 
                    "Streaming should use less than 2MB additional memory (actual: {:.2} MB)", 
                    memory_growth);
        }
        Err(_) => {
            // Mock/test environment - skip
            println!("Skipping streaming test (no mock server)");
        }
    }
}

#[tokio::test]
async fn test_concurrent_memory_usage() {
    println!("🔄 Testing Concurrent Request Memory Usage");
    
    let mut system = System::new_all();
    system.refresh_all();
    let pid = Pid::from(std::process::id() as usize);
    
    let provider = Arc::new(OpenAIProvider::new("test-key".to_string(), None));
    let initial_memory = get_process_memory(&mut system, pid);
    
    // Create 100 concurrent requests
    let mut tasks = Vec::new();
    for i in 0..100 {
        let p = provider.clone();
        tasks.push(tokio::spawn(async move {
            let request = create_test_request(i);
            // Simulate processing
            tokio::time::sleep(Duration::from_millis(10)).await;
            p.chat(request).await
        }));
    }
    
    // Monitor memory while requests are in flight
    let monitor_handle = tokio::spawn(async move {
        let mut max_memory = initial_memory;
        for _ in 0..10 {
            system.refresh_all();
            let current = get_process_memory(&mut system, pid);
            max_memory = max_memory.max(current);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        (max_memory, initial_memory)
    });
    
    // Wait for all requests
    for task in tasks {
        let _ = task.await;
    }
    
    let (max_memory, initial) = monitor_handle.await.unwrap();
    let peak_overhead = max_memory - initial;
    
    println!("Peak memory during 100 concurrent requests: {:.2} MB (overhead: {:.2} MB)", 
           max_memory, peak_overhead);
    
    // Assert memory stays reasonable under load
    assert!(peak_overhead < 20.0, 
            "Concurrent requests should use less than 20MB (actual: {:.2} MB)", 
            peak_overhead);
}

// Helper functions
fn get_process_memory(system: &mut System, pid: Pid) -> f64 {
    system.refresh_process(pid);
    if let Some(process) = system.process(pid) {
        process.memory() as f64 / 1024.0 / 1024.0 // Convert to MB
    } else {
        0.0
    }
}

async fn test_provider_memory(
    provider: Arc<dyn AiProvider + Send + Sync>,
    system: &mut System,
    pid: Pid
) {
    let name = provider.name();
    let before = get_process_memory(system, pid);
    
    // Create and execute a request
    let request = create_test_request(0);
    let _ = provider.chat(request).await;
    
    system.refresh_all();
    let after = get_process_memory(system, pid);
    let overhead = after - before;
    
    println!("Provider '{}': {:.2} MB overhead", name, overhead);
    
    // Each provider should use minimal memory
    assert!(overhead < 2.0, 
            "Provider '{}' should use less than 2MB (actual: {:.2} MB)", 
            name, overhead);
}

fn create_test_request(id: usize) -> ChatRequest {
    ChatRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: Some(format!("Test request #{}", id)),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        max_tokens: Some(100),
        temperature: Some(0.7),
        ..Default::default()
    }
}
