/// List available Gemini models

use anyhow::Result;

const API_KEY: &str = "AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU";

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n=== LISTING GEMINI MODELS ===\n");
    
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        API_KEY
    );
    
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    
    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;
        
        if let Some(models) = json["models"].as_array() {
            println!("Available models:");
            for model in models {
                if let Some(name) = model["name"].as_str() {
                    let display_name = model["displayName"].as_str().unwrap_or("");
                    let methods = model["supportedGenerationMethods"]
                        .as_array()
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(", "))
                        .unwrap_or_default();
                    
                    println!("\n  Model: {}", name);
                    println!("    Display: {}", display_name);
                    println!("    Methods: {}", methods);
                }
            }
        }
    } else {
        let error = response.text().await?;
        println!("Error: {}", error);
    }
    
    Ok(())
}
