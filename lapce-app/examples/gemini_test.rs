use serde_json::{json, Value};
use futures_util::StreamExt;

const GEMINI_API_KEY: &str = "AIzaSyCr-9z7jQ-co7IvuvxCWvhvitxMOINIXcU";
const GEMINI_MODEL: &str = "gemini-2.5-flash";

#[tokio::main]
async fn main() {
    println!("\n🤖 Gemini API Test - Auto-sending 'hi'\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📤 YOU: hi");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🌐 Calling Gemini API...\n");
    
    match call_gemini_streaming("hi").await {
        Ok(_) => {
            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("✅ Success! Gemini is responding with streaming!");
        }
        Err(e) => {
            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("❌ ERROR: {}", e);
        }
    }
}

async fn call_gemini_streaming(prompt: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
        GEMINI_MODEL, GEMINI_API_KEY
    );
    
    let payload = json!({
        "contents": [{
            "parts": [{
                "text": prompt
            }]
        }],
        "generationConfig": {
            "temperature": 0.9,
            "topK": 40,
            "topP": 0.95,
            "maxOutputTokens": 2048,
        }
    });
    
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("API error {}: {}", status, error_text));
    }
    
    print!("🤖 GEMINI: ");
    std::io::Write::flush(&mut std::io::stdout()).ok();
    
    // TRUE STREAMING: Process bytes as they arrive from network
    use futures_util::stream::StreamExt;
    let mut stream = response.bytes_stream();
    let mut line_buffer = Vec::new();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream error: {}", e))?;
        
        // Process each byte
        for &byte in chunk.as_ref() {
            if byte == b'\n' {
                // Parse complete line
                if let Ok(line) = String::from_utf8(line_buffer.clone()) {
                    if line.starts_with("data: ") {
                        let json_str = &line[6..];
                        if let Ok(json) = serde_json::from_str::<Value>(json_str) {
                            if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                                // Typewriter effect: print character-by-character
                                for ch in text.chars() {
                                    print!("{}", ch);
                                    std::io::Write::flush(&mut std::io::stdout()).ok();
                                    // Small delay for typewriter effect
                                    std::thread::sleep(std::time::Duration::from_millis(20));
                                }
                            }
                        }
                    }
                }
                line_buffer.clear();
            } else if byte != b'\r' {
                line_buffer.push(byte);
            }
        }
    }
    
    Ok(())
}
