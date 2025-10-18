use std::io::{BufRead, BufReader, Cursor};

// Test the SSE parsing logic in isolation
fn main() {
    let test_sse_data = r#"data: {"candidates": [{"content": {"parts": [{"text": "Hello"}],"role": "model"},"index": 0}]}

data: {"candidates": [{"content": {"parts": [{"text": " world! This is"}],"role": "model"},"index": 0}]}

data: {"candidates": [{"content": {"parts": [{"text": " a test message."}],"role": "model"},"index": 0}]}

data: [DONE]
"#;

    let cursor = Cursor::new(test_sse_data);
    let mut reader = BufReader::new(cursor);
    let mut line = String::new();
    let mut result = String::new();

    println!("Testing SSE parsing:");
    
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let line = line.trim_end();
                if !line.is_empty() {
                    println!("Raw line: {}", line);
                    
                    if let Some(json_str) = line.strip_prefix("data: ") {
                        let trimmed = json_str.trim();
                        if trimmed == "[DONE]" {
                            println!("Stream done");
                            break;
                        } else if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                            if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                                println!("Parsed text: '{}'", text);
                                result.push_str(text);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }
    
    println!("\nFinal result: '{}'", result);
    println!("Expected: 'Hello world! This is a test message.'");
    println!("Match: {}", result == "Hello world! This is a test message.");
}
