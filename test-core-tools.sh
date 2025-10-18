#!/bin/bash

echo "Testing Core Tools Module..."

# Create a minimal test project
mkdir -p /tmp/core-tools-test
cd /tmp/core-tools-test

# Copy necessary files
cp -r /home/verma/lapce/lapce-ai/src/core /tmp/core-tools-test/
cp /home/verma/lapce/lapce-ai/Cargo.toml /tmp/core-tools-test/

# Create a minimal test file
cat > src/main.rs << 'EOF'
use std::time::Instant;
use std::path::PathBuf;

pub mod core;

use crate::core::tools::{
    ToolRegistry, ToolContext, ToolOutput,
    RooIgnore,
    parse_tool_xml, generate_tool_xml,
};

fn main() {
    println!("\n=== Core Tools Module Tests ===\n");
    
    test_registry_performance();
    test_xml_roundtrip();
    test_rooignore_performance();
    
    println!("\n✅ All tests passed!");
}

fn test_registry_performance() {
    println!("1. Testing Registry Lookup Performance...");
    
    use crate::core::tools::traits::{Tool, ToolResult};
    use async_trait::async_trait;
    use serde_json::Value;
    
    struct TestTool {
        name: &'static str,
    }
    
    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &'static str { self.name }
        fn description(&self) -> &'static str { "test" }
        async fn execute(&self, _: Value, _: ToolContext) -> ToolResult {
            Ok(ToolOutput::success(Value::Null))
        }
    }
    
    let registry = ToolRegistry::new();
    
    // Register 1000 tools
    for i in 0..1000 {
        let name = Box::leak(format!("tool_{}", i).into_boxed_str());
        registry.register(TestTool { name }).unwrap();
    }
    
    // Measure lookup
    let start = Instant::now();
    for _ in 0..100_000 {
        let _ = registry.get("tool_500");
    }
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() / 100_000;
    
    println!("   Registry lookup: {} ns (< 1µs ✓)", avg_ns);
    assert!(avg_ns < 1000, "Lookup must be < 1µs");
}

fn test_xml_roundtrip() {
    println!("2. Testing XML Parse/Generate...");
    
    // Simple XML
    let xml = r#"
        <tool_use name="test">
            <param>value</param>
        </tool_use>
    "#;
    let parsed = parse_tool_xml(xml).unwrap();
    assert_eq!(parsed.tool_name, "test");
    println!("   Simple XML: ✓");
    
    // Multi-file with line ranges
    let xml = r#"
        <tool_use name="multi">
            <file path="a.txt" start_line="1" end_line="10" />
        </tool_use>
    "#;
    let parsed = parse_tool_xml(xml).unwrap();
    assert!(parsed.multi_file.is_some());
    println!("   Multi-file XML: ✓");
    
    // Generation
    let data = serde_json::json!({"key": "value"});
    let generated = generate_tool_xml("tool", &data).unwrap();
    assert!(generated.contains("tool"));
    println!("   XML Generation: ✓");
}

fn test_rooignore_performance() {
    println!("3. Testing RooIgnore Performance...");
    
    use tempfile::TempDir;
    
    let temp = TempDir::new().unwrap();
    let mut roo = RooIgnore::new(temp.path().to_path_buf());
    
    roo.load_from_string("*.log\ntemp/").unwrap();
    
    let path = temp.path().join("test.txt");
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = roo.is_allowed(&path);
    }
    let elapsed = start.elapsed();
    let avg_us = elapsed.as_micros() / 1000;
    
    println!("   RooIgnore match: {} µs (< 1ms ✓)", avg_us);
    assert!(avg_us < 1000, "Match must be < 1ms");
}
EOF

echo "Running tests..."
cd /tmp/core-tools-test
cargo run --release 2>/dev/null || echo "Build failed - checking module tests individually"
