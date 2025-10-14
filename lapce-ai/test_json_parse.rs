fn main() {
    let json_str = r#"{"path":"test.txt","content":"Hello"}"#;
    println!("JSON string: {}", json_str);
    
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(value) => {
            println!("Parsed successfully: {:?}", value);
            println!("Path: {:?}", value["path"]);
            println!("Content: {:?}", value["content"]);
        }
        Err(e) => {
            println!("Failed to parse: {}", e);
        }
    }
}
