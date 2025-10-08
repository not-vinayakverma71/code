/// Simple AWS Bedrock Titan test with provided credentials
use anyhow::Result;
use colored::Colorize;
use futures::stream::StreamExt;
use std::time::Instant;

use lapce_ai_rust::ai_providers::{
    core_trait::{AiProvider, ChatRequest, ChatMessage, StreamToken},
    bedrock_exact::{BedrockProvider, BedrockConfig},
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    
    println!("\n{}", "🔍 AWS BEDROCK TITAN TEST".bright_cyan().bold());
    println!("{}", "=".repeat(60).bright_cyan());
    
    // Use provided AWS credentials
    let access_key = std::env::var("AWS_ACCESS_KEY_ID")
        .unwrap_or_else(|_| "AKIA2RCKMSFVZ72HLCXD".to_string());
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
        .unwrap_or_else(|_| "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".to_string());
    let region = std::env::var("AWS_REGION")
        .unwrap_or_else(|_| "us-east-1".to_string());
    
    println!("📌 Configuration:");
    println!("  • Region: {}", region);
    println!("  • Access Key: {}...", &access_key[..10]);
    
    // Create Bedrock provider
    let config = BedrockConfig {
        region: region.clone(),
        access_key_id: access_key,
        secret_access_key: secret_key,
        session_token: None,
        base_url: None,
        default_model: Some("amazon.titan-text-express-v1".to_string()),
        timeout_ms: Some(30000),
    };
    
    println!("\n⚙️ Initializing Bedrock provider...");
    let provider = BedrockProvider::new(config).await?;
    
    // Test 1: Simple request
    println!("\n{}", "TEST 1: Simple Request".bright_yellow().bold());
    println!("{}", "-".repeat(40).yellow());
    
    let request = ChatRequest {
        model: "amazon.titan-text-express-v1".to_string(),
        messages: vec![
            ChatMessage {
                role: "user".to_string(),
                content: Some("Say 'Hello from AWS Titan' and nothing else.".to_string()),
                name: None,
                function_call: None,
                tool_calls: None,
            }
        ],
        temperature: Some(0.0),
        max_tokens: Some(20),
        stream: Some(true),
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
    
    let start = Instant::now();
    println!("📤 Sending request to Titan...");
    
    match provider.chat_stream(request.clone()).await {
        Ok(mut stream) => {
            let mut response_text = String::new();
            let mut token_count = 0;
            
            while let Some(result) = stream.next().await {
                match result {
                    Ok(token) => {
                        match token {
                            StreamToken::Text(text) => {
                                response_text.push_str(&text);
                                token_count += 1;
                                print!("{}", text.green());
                            }
                            StreamToken::Delta { content } => {
                                response_text.push_str(&content);
                                token_count += 1;
                                print!("{}", content.green());
                            }
                            StreamToken::Done => {
                                println!("\n✅ Stream completed");
                                break;
                            }
                            StreamToken::Error(e) => {
                                println!("\n❌ Stream error: {}", e.red());
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        println!("\n❌ Error: {}", e.to_string().red());
                        break;
                    }
                }
            }
            
            let duration = start.elapsed();
            println!("\n📊 Stats:");
            println!("  • Response time: {:.2}s", duration.as_secs_f64());
            println!("  • Tokens received: {}", token_count);
            println!("  • Response: {}", response_text.trim());
        }
        Err(e) => {
            println!("❌ Failed to create stream: {}", e.to_string().red());
            println!("\n🔍 Debugging info:");
            println!("  • Error type: {:?}", e);
            
            // Try to provide more context
            if e.to_string().contains("403") {
                println!("  ⚠️ Access denied - check AWS credentials");
            } else if e.to_string().contains("404") {
                println!("  ⚠️ Model not found - check model ID");
            } else if e.to_string().contains("connection") {
                println!("  ⚠️ Connection issue - check network/region");
            }
        }
    }
    
    // Test 2: Available models
    println!("\n{}", "TEST 2: List Available Models".bright_yellow().bold());
    println!("{}", "-".repeat(40).yellow());
    
    match provider.list_models().await {
        Ok(models) => {
            println!("✅ Available models:");
            for model in models {
                println!("  • {}", model.id.cyan());
            }
        }
        Err(e) => {
            println!("⚠️ Could not list models: {}", e.to_string().yellow());
            println!("  Note: Bedrock may not support model listing");
        }
    }
    
    // Test 3: Multiple quick requests
    println!("\n{}", "TEST 3: 10 Quick Requests".bright_yellow().bold());
    println!("{}", "-".repeat(40).yellow());
    
    let mut success_count = 0;
    let mut fail_count = 0;
    
    for i in 1..=10 {
        print!("  Request {}/10: ", i);
        
        let test_request = ChatRequest {
            model: "amazon.titan-text-express-v1".to_string(),
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: Some(format!("Say 'Response {}' and nothing else.", i)),
                    name: None,
                    function_call: None,
                    tool_calls: None,
                }
            ],
            temperature: Some(0.0),
            max_tokens: Some(10),
            stream: Some(false), // Non-streaming for speed
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
        
        match provider.chat(test_request).await {
            Ok(response) => {
                success_count += 1;
                println!("✅ Success");
                if let Some(choice) = response.choices.first() {
                    if let Some(content) = &choice.message.content {
                        println!("    Response: {}", content.trim().green());
                    }
                }
            }
            Err(e) => {
                fail_count += 1;
                println!("❌ Failed: {}", e.to_string().red());
            }
        }
        
        // Small delay to avoid rate limiting
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    
    println!("\n📊 Test 3 Results:");
    println!("  • Successful: {}/{}", success_count.to_string().green(), 10);
    println!("  • Failed: {}/{}", fail_count.to_string().red(), 10);
    
    // Summary
    println!("\n{}", "📝 SUMMARY".bright_green().bold());
    println!("{}", "=".repeat(60).bright_green());
    
    if success_count > 0 {
        println!("✅ AWS Bedrock Titan is working!");
        println!("   Ready for larger tests (1000 concurrent, 1M tokens)");
    } else {
        println!("❌ AWS Bedrock Titan connection failed");
        println!("   Please check:");
        println!("   • AWS credentials are valid");
        println!("   • Region is correct ({}", region);
        println!("   • Titan model is available in this region");
    }
    
    Ok(())
}
