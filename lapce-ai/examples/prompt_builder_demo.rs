//! Prompt Builder Demo
//!
//! Demonstrates prompt generation for different modes

use lapce_ai_rust::core::prompt::{
    builder::PromptBuilder,
    modes::get_mode_by_slug,
    settings::SystemPromptSettings,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Prompt Builder Demo ===\n");

    // Setup
    let workspace = PathBuf::from("/home/user/project");
    let settings = SystemPromptSettings::default();

    // Demo 1: Code mode with no custom instructions
    println!("1. Code Mode (Default)\n");
    let code_mode = get_mode_by_slug("code")?;
    let builder = PromptBuilder::new(
        workspace.clone(),
        code_mode,
        settings.clone(),
        None,
    );

    match builder.build_and_count().await {
        Ok((prompt, tokens)) => {
            println!("✓ Generated prompt: {} characters", prompt.len());
            println!("✓ Estimated tokens: {}", tokens);
            println!("\n--- First 500 chars ---");
            println!("{}", &prompt[..500.min(prompt.len())]);
            println!("...\n");
        }
        Err(e) => println!("✗ Error: {}\n", e),
    }

    // Demo 2: Architect mode with custom instructions
    println!("2. Architect Mode (With Custom Instructions)\n");
    let architect_mode = get_mode_by_slug("architect")?;
    let builder = PromptBuilder::new(
        workspace.clone(),
        architect_mode,
        settings.clone(),
        Some("Always use Rust for examples. Prefer functional programming style.".to_string()),
    );

    match builder.build_and_count().await {
        Ok((prompt, tokens)) => {
            println!("✓ Generated prompt: {} characters", prompt.len());
            println!("✓ Estimated tokens: {}", tokens);
            
            // Check for custom instructions
            if prompt.contains("Always use Rust") {
                println!("✓ Custom instructions included");
            }
        }
        Err(e) => println!("✗ Error: {}\n", e),
    }

    // Demo 3: Test build_with_retry
    println!("\n3. Build With Retry (Error Recovery)\n");
    let debug_mode = get_mode_by_slug("debug")?;
    let builder = PromptBuilder::new(
        workspace.clone(),
        debug_mode,
        settings.clone(),
        None,
    );

    match builder.build_with_retry().await {
        Ok(prompt) => {
            println!("✓ Built successfully with retry logic");
            println!("✓ Prompt length: {} characters", prompt.len());
        }
        Err(e) => println!("✗ Error: {}", e),
    }

    // Demo 4: All modes comparison
    println!("\n4. All Modes Comparison\n");
    let modes = ["code", "architect", "ask", "debug", "orchestrator"];
    
    for mode_slug in &modes {
        let mode = get_mode_by_slug(mode_slug)?;
        let builder = PromptBuilder::new(
            workspace.clone(),
            mode,
            settings.clone(),
            None,
        );
        
        match builder.build_and_count().await {
            Ok((prompt, tokens)) => {
                println!("  {:<12} - {:>6} chars, ~{:>5} tokens", 
                    mode_slug, prompt.len(), tokens);
            }
            Err(e) => {
                println!("  {:<12} - Error: {}", mode_slug, e);
            }
        }
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}
