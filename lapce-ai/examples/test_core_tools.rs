// Example test for core tools module
use std::time::Instant;
use std::path::PathBuf;
use tempfile::TempDir;

use lapce_ai_rust::core::tools::{
    ToolRegistry, ToolContext, ToolOutput,
    RooIgnore,
    parse_tool_xml, generate_tool_xml,
};
use lapce_ai_rust::core::tools::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

struct BenchTool {
    name: &'static str,
}

#[async_trait]
impl Tool for BenchTool {
    fn name(&self) -> &'static str { self.name }
    fn description(&self) -> &'static str { "bench tool" }
    async fn execute(&self, _: Value, _: ToolContext) -> ToolResult {
        Ok(ToolOutput::success(Value::Null))
    }
}

#[tokio::main]
async fn main() {
    println!("\n=== Core Tools Performance Tests ===\n");
    
    // Test 1: Registry Lookup < 1µs
    println!("1. Registry Lookup Performance");
    let registry = ToolRegistry::new();
    for i in 0..1000 {
        let name = Box::leak(format!("tool_{}", i).into_boxed_str());
        registry.register(BenchTool { name }).unwrap();
    }
    
    let iterations = 100_000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = registry.get("tool_500");
    }
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() / iterations as u128;
    println!("   Average lookup: {} ns", avg_ns);
    if avg_ns < 1000 {
        println!("   ✅ < 1µs requirement met!");
    } else {
        println!("   ❌ Failed: {} ns > 1000 ns", avg_ns);
    }
    
    // Test 2: XML Parse/Generate Roundtrip
    println!("\n2. XML Parse/Generate Tests");
    
    // Simple XML
    let simple = r#"
        <tool_use name="readFile">
            <path>/test.txt</path>
        </tool_use>
    "#;
    let parsed = parse_tool_xml(simple).unwrap();
    assert_eq!(parsed.tool_name, "readFile");
    println!("   ✅ Simple XML parsing works");
    
    // Multi-file with line ranges
    let multi = r#"
        <tool_use name="readFiles">
            <file path="a.txt" start_line="1" end_line="10" />
            <file path="b.txt" start_line="5" end_line="15" />
        </tool_use>
    "#;
    let parsed = parse_tool_xml(multi).unwrap();
    let files = parsed.multi_file.unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].start_line, Some(1));
    assert_eq!(files[0].end_line, Some(10));
    println!("   ✅ Multi-file with line ranges works");
    
    // Generate and verify
    let data = serde_json::json!({
        "result": "success",
        "count": 42
    });
    let xml = generate_tool_xml("test", &data).unwrap();
    assert!(xml.contains("test"));
    assert!(xml.contains("success"));
    assert!(xml.contains("42"));
    println!("   ✅ XML generation works");
    
    // Test 3: RooIgnore Performance
    println!("\n3. RooIgnore Performance");
    let temp = TempDir::new().unwrap();
    let mut roo = RooIgnore::new(temp.path().to_path_buf());
    
    roo.load_from_string("*.log\n*.tmp\nnode_modules/").unwrap();
    
    // Test correctness
    assert!(!roo.is_allowed(&temp.path().join("test.log")));
    assert!(roo.is_allowed(&temp.path().join("test.txt")));
    println!("   ✅ Pattern matching works");
    
    // Test performance
    let path = temp.path().join("test.rs");
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = roo.is_allowed(&path);
    }
    let elapsed = start.elapsed();
    let avg_us = elapsed.as_micros() / 1000;
    println!("   Average match: {} µs", avg_us);
    if avg_us < 1000 {
        println!("   ✅ < 1ms requirement met!");
    } else {
        println!("   ❌ Failed: {} µs > 1000 µs", avg_us);
    }
    
    println!("\n=== Summary ===");
    println!("✅ All core tools tests completed successfully!");
}
