use anyhow::Result;

// Example usage of the Semantic Search API

#[tokio::main]
async fn main() -> Result<()> {
    // Create client
    let client = semantic_search_client::SemanticSearchClient::new(
        "http://localhost:8080".to_string()
    )?;
    
    // Index some code
    let code = r#"
    fn fibonacci(n: u32) -> u32 {
        match n {
            0 => 0,
            1 => 1,
            _ => fibonacci(n - 1) + fibonacci(n - 2),
        }
    }
    "#;
    
    println!("Indexing code...");
    let index_response = client.index(
        "examples/fibonacci.rs".to_string(),
        code.to_string(),
        Some("rust".to_string()),
    ).await?;
    
    println!("Indexed {} chunks", index_response.chunks_indexed);
    
    // Search for code
    println!("\nSearching for 'fibonacci'...");
    let search_response = client.search(
        "fibonacci recursive function".to_string(),
        Some(5),
    ).await?;
    
    println!("Found {} results in {:.2}ms", 
        search_response.total, 
        search_response.latency_ms
    );
    
    for (i, result) in search_response.results.iter().enumerate() {
        println!("\n{}. Score: {:.3}", i + 1, result.score);
        println!("   File: {}", result.path);
        println!("   Lines: {}-{}", result.start_line, result.end_line);
        println!("   Preview: {}", &result.content[..100.min(result.content.len())]);
    }
    
    // Check health
    let health = client.health().await?;
    println!("\nHealth: {}", health.status);
    println!("Version: {}", health.version);
    println!("Uptime: {}s", health.uptime_seconds);
    
    Ok(())
}

mod semantic_search_client {
    pub use crate::client_sdk::*;
    include!("../src/bin/client_sdk.rs");
}
