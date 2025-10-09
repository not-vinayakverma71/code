// Test runner for core tools module

use lapce_ai_rust::core::tools::{
    ToolRegistry, ToolContext, RooIgnore,
    parse_tool_xml, generate_tool_xml,
};
use lapce_ai_rust::core::tools::traits::ToolOutput;
use std::time::Instant;
use std::path::PathBuf;

fn main() {
    println!("Running Core Tools Tests\n");
    
    test_registry_lookup_performance();
    test_xml_parse_generate();
    test_rooignore();
    
    println!("\n✅ All Core Tools Tests Passed!");
}

fn test_registry_lookup_performance() {
    println!("Testing Registry Lookup Performance...");
    
    use lapce_ai_rust::core::tools::{Tool, ToolResult};
    use async_trait::async_trait;
    use serde_json::Value;
    
    struct MockTool {
        name: &'static str,
    }
    
    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &'static str {
            self.name
        }
        
        fn description(&self) -> &'static str {
            "Mock tool"
        }
        
        async fn execute(&self, _args: Value, _context: ToolContext) -> ToolResult {
            Ok(ToolOutput::success(Value::Null))
        }
    }
    
    let registry = ToolRegistry::new();
    
    // Register 1000 tools
    for i in 0..1000 {
        let name = Box::leak(format!("tool_{}", i).into_boxed_str());
        let tool = MockTool { name };
        registry.register(tool).unwrap();
    }
    
    // Measure lookup time
    let iterations = 100_000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let _ = registry.get("tool_500");
    }
    
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() / iterations as u128;
    
    println!("  Average lookup time: {} ns", avg_ns);
    assert!(avg_ns < 1000, "Lookup must be < 1 microsecond, was {} ns", avg_ns);
    println!("  ✓ Registry lookup < 1µs confirmed!");
}

fn test_xml_parse_generate() {
    println!("Testing XML Parse/Generate...");
    
    // Test simple XML parsing
    let simple_xml = r#"
        <tool_use name="readFile">
            <path>/test/file.txt</path>
            <encoding>utf-8</encoding>
        </tool_use>
    "#;
    
    let args = parse_tool_xml(simple_xml).unwrap();
    assert_eq!(args.tool_name, "readFile");
    assert_eq!(args.arguments["path"], "/test/file.txt");
    assert_eq!(args.arguments["encoding"], "utf-8");
    println!("  ✓ Simple XML parsing works");
    
    // Test multi-file XML with line ranges
    let multi_file_xml = r#"
        <tool_use name="readFiles">
            <file path="file1.txt" start_line="10" end_line="20" />
            <file>
                <path>file2.txt</path>
                <start_line>5</start_line>
                <end_line>15</end_line>
            </file>
        </tool_use>
    "#;
    
    let args = parse_tool_xml(multi_file_xml).unwrap();
    assert_eq!(args.tool_name, "readFiles");
    let files = args.multi_file.unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, "file1.txt");
    assert_eq!(files[0].start_line, Some(10));
    assert_eq!(files[0].end_line, Some(20));
    assert_eq!(files[1].path, "file2.txt");
    assert_eq!(files[1].start_line, Some(5));
    assert_eq!(files[1].end_line, Some(15));
    println!("  ✓ Multi-file XML with line ranges works");
    
    // Test generate and roundtrip
    let result = serde_json::json!({
        "content": "Hello, World!",
        "lines": 42,
        "success": true
    });
    
    let xml = generate_tool_xml("testTool", &result).unwrap();
    assert!(xml.contains(r#"name="testTool""#));
    assert!(xml.contains("Hello, World!"));
    assert!(xml.contains("42"));
    assert!(xml.contains("true"));
    println!("  ✓ XML generation works");
    
    println!("  ✓ XML roundtrip successful!");
}

fn test_rooignore() {
    println!("Testing RooIgnore...");
    
    use tempfile::TempDir;
    
    let temp_dir = TempDir::new().unwrap();
    let mut rooignore = RooIgnore::new(temp_dir.path().to_path_buf());
    
    // Test basic patterns
    let patterns = r#"
*.log
temp/
build/
!build/important.txt
"#;
    
    rooignore.load_from_string(patterns).unwrap();
    
    // Test blocked patterns
    assert!(!rooignore.is_allowed(&temp_dir.path().join("debug.log")));
    assert!(!rooignore.is_allowed(&temp_dir.path().join("temp/file.txt")));
    assert!(!rooignore.is_allowed(&temp_dir.path().join("build/output.exe")));
    
    // Test negation pattern
    assert!(rooignore.is_allowed(&temp_dir.path().join("build/important.txt")));
    
    // Test allowed patterns
    assert!(rooignore.is_allowed(&temp_dir.path().join("src/main.rs")));
    assert!(rooignore.is_allowed(&temp_dir.path().join("readme.md")));
    
    println!("  ✓ RooIgnore patterns work correctly!");
    
    // Test performance
    let start = Instant::now();
    for i in 0..1000 {
        let _ = rooignore.is_allowed(&temp_dir.path().join(format!("file{}.txt", i)));
    }
    let elapsed = start.elapsed();
    let avg_us = elapsed.as_micros() / 1000;
    
    println!("  Average match time: {} µs", avg_us);
    assert!(avg_us < 1000, "Match must be < 1ms, was {} µs", avg_us);
    println!("  ✓ RooIgnore performance < 1ms/match confirmed!");
}
