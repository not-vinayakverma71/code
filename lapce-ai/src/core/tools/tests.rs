// Comprehensive tests for tool system - P0-tests

#[cfg(test)]
mod permission_tests;

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::core::tools::registry::ToolRegistry;
    use crate::core::tools::xml_util::{XmlParser, XmlGenerator};
    use crate::core::tools::traits::{Tool, ToolResult, ToolError, ToolContext, ToolOutput, ToolPermissions};
    use serde_json::{json, Value};
    use std::sync::Arc;
    use tokio;
    use std::time::Instant;

    // Test tool implementation
    #[derive(Clone)]
    struct TestTool {
        name: String,
        delay_ms: u64,
    }

    #[async_trait::async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &'static str {
            // For tests, leak the string to get a static reference
            Box::leak(self.name.clone().into_boxed_str())
        }

        fn description(&self) -> &'static str {
            "Test tool for unit tests"
        }

        async fn execute(&self, args: Value, _ctx: ToolContext) -> ToolResult {
            if self.delay_ms > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
            }
            
            Ok(ToolOutput {
                success: true,
                result: args,
                error: None,
                metadata: [
                    ("tool".to_string(), json!(self.name)),
                    ("timestamp".to_string(), json!(chrono::Utc::now().to_rfc3339()))
                ].into_iter().collect(),
            })
        }
    }

    #[tokio::test]
    async fn test_registry_register_and_get() {
        let registry = ToolRegistry::new();
        
        let tool = TestTool {
            name: "test_tool".to_string(),
            delay_ms: 0,
        };
        
        registry.register(tool).unwrap();
        
        // Should find the tool
        let found = registry.get("test_tool");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "test_tool");
        
        // Should not find non-existent tool
        let not_found = registry.get("non_existent");
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_registry_list_tools() {
        let registry = ToolRegistry::new();
        
        // Register multiple tools
        for i in 0..5 {
            let tool = TestTool {
                name: format!("tool_{}", i),
                delay_ms: 0,
            };
            registry.register(tool).unwrap();
        }
        
        let tools = registry.list_tools();
        assert_eq!(tools.len(), 5);
        
        // Check all tools are present
        let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
        for i in 0..5 {
            assert!(tool_names.contains(&format!("tool_{}", i)));
        }
    }

    #[tokio::test]
    async fn test_registry_performance() {
        let registry = ToolRegistry::new();
        
        // Register 1000 tools
        for i in 0..1000 {
            let tool = TestTool {
                name: format!("perf_tool_{}", i),
                delay_ms: 0,
            };
            registry.register(tool).unwrap();
        }
        
        // Measure lookup performance
        let start = Instant::now();
        for i in 0..1000 {
            let _ = registry.get(&format!("perf_tool_{}", i));
        }
        let elapsed = start.elapsed();
        
        // Should complete 1000 lookups in < 10ms
        assert!(elapsed.as_millis() < 10, "Registry lookups too slow: {:?}", elapsed);
    }

    #[test]
    fn test_xml_parser_basic() {
        let xml = r#"
            <tool>
                <name>readFile</name>
                <args>
                    <path>/test/file.txt</path>
                    <lineStart>10</lineStart>
                    <lineEnd>20</lineEnd>
                </args>
            </tool>
        "#;
        
        let parser = XmlParser::new();
        let parsed = parser.parse(xml).unwrap();
        
        assert_eq!(parsed["tool"]["name"].as_str().unwrap(), "readFile");
        assert_eq!(parsed["tool"]["args"]["path"].as_str().unwrap(), "/test/file.txt");
        assert_eq!(parsed["tool"]["args"]["lineStart"].as_str().unwrap(), "10");
        assert_eq!(parsed["tool"]["args"]["lineEnd"].as_str().unwrap(), "20");
    }

    #[test]
    fn test_xml_parser_multi_file() {
        let xml = r#"
            <tool>
                <name>multiEdit</name>
                <files>
                    <file>
                        <path>file1.rs</path>
                        <lineStart>1</lineStart>
                        <lineEnd>10</lineEnd>
                    </file>
                    <file>
                        <path>file2.rs</path>
                        <lineStart>20</lineStart>
                        <lineEnd>30</lineEnd>
                    </file>
                </files>
            </tool>
        "#;
        
        let parser = XmlParser::new();
        let parsed = parser.parse(xml).unwrap();
        
        assert_eq!(parsed["tool"]["name"].as_str().unwrap(), "multiEdit");
        
        // The parser converts multiple <file> elements to an array
        let files = parsed["tool"]["files"]["file"].as_array().unwrap();
        assert_eq!(files.len(), 2);
        
        assert_eq!(files[0]["path"].as_str().unwrap(), "file1.rs");
        assert_eq!(files[0]["lineStart"].as_str().unwrap(), "1");
        assert_eq!(files[0]["lineEnd"].as_str().unwrap(), "10");
        
        assert_eq!(files[1]["path"].as_str().unwrap(), "file2.rs");
        assert_eq!(files[1]["lineStart"].as_str().unwrap(), "20");
        assert_eq!(files[1]["lineEnd"].as_str().unwrap(), "30");
    }

    #[test]
    fn test_xml_generator_basic() {
        let data = json!({
            "status": "success",
            "content": "File content here",
            "metadata": {
                "size": 1024,
                "modified": "2024-01-01T00:00:00Z"
            }
        });
        
        let generator = XmlGenerator::new();
        let xml = generator.generate(&data).unwrap();
        
        assert!(xml.contains("<status>success</status>"));
        assert!(xml.contains("<content>File content here</content>"));
        assert!(xml.contains("<size>1024</size>"));
        assert!(xml.contains("<modified>2024-01-01T00:00:00Z</modified>"));
    }

    #[test]
    fn test_xml_roundtrip() {
        let original = json!({
            "tool": "readFile",
            "args": {
                "path": "/test/file.txt",
                "encoding": "utf-8",
                "lineRange": {
                    "start": 10,
                    "end": 20
                }
            },
            "metadata": {
                "requestId": "abc123",
                "timestamp": "2024-01-01T00:00:00Z"
            }
        });
        
        let generator = XmlGenerator::new();
        let xml = generator.generate(&original).unwrap();
        
        let parser = XmlParser::new();
        let parsed = parser.parse(&xml).unwrap();
        
        // Check key fields survived roundtrip - the parser wraps in a root element
        assert_eq!(parsed["root"]["tool"].as_str().unwrap(), "readFile");
        assert_eq!(parsed["root"]["args"]["path"].as_str().unwrap(), "/test/file.txt");
        assert_eq!(parsed["root"]["args"]["encoding"].as_str().unwrap(), "utf-8");
        assert_eq!(
            parsed["root"]["args"]["lineRange"]["start"].as_str().unwrap(), 
            "10"
        );
        assert_eq!(
            parsed["root"]["args"]["lineRange"]["end"].as_str().unwrap(),
            "20"
        );
    }

    #[test]
    fn test_xml_parser_error_handling() {
        let parser = XmlParser::new();
        
        // Invalid XML with actual parsing error (unclosed tag with content)
        let result = parser.parse("<tool><name>test</name>");
        assert!(result.is_err());
        
        // Empty input
        let result = parser.parse("");
        assert!(result.is_err());
        
        // Malformed XML
        let result = parser.parse("<tool><name>test</tool></name>");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let tool = TestTool {
            name: "exec_test".to_string(),
            delay_ms: 10,
        };
        
        let args = json!({
            "input": "test data",
            "count": 42
        });
        
        let ctx = ToolContext::default();
        
        let start = Instant::now();
        let result = tool.execute(args.clone(), ctx).await.unwrap();
        let elapsed = start.elapsed();
        
        // Should have the delay
        assert!(elapsed.as_millis() >= 10);
        
        // Should return success
        assert!(result.success);
        assert_eq!(result.result, args);
        assert_eq!(result.metadata["tool"], "exec_test");
    }

    #[tokio::test]
    async fn test_concurrent_tool_execution() {
        let tool = Arc::new(TestTool {
            name: "concurrent_test".to_string(),
            delay_ms: 10,
        });
        
        let mut handles = vec![];
        
        // Launch 10 concurrent executions
        for i in 0..10 {
            let tool_clone = tool.clone();
            let handle = tokio::spawn(async move {
                let args = json!({ "id": i });
                let ctx = ToolContext::default();
                tool_clone.execute(args, ctx).await
            });
            handles.push(handle);
        }
        
        // Wait for all to complete
        let results: Vec<_> = futures::future::join_all(handles).await;
        
        // All should succeed
        for result in results {
            let tool_result = result.unwrap().unwrap();
            assert!(tool_result.success);
        }
    }

    #[test]
    fn test_tool_error_creation() {
        let error = ToolError::NotFound("missing_tool".to_string());
        assert_eq!(error.to_string(), "Tool not found: missing_tool");
        
        let error = ToolError::InvalidArgs("bad format".to_string());
        assert_eq!(error.to_string(), "Invalid arguments: bad format");
        
        let error = ToolError::ExecutionFailed("failed to run".to_string());
        assert_eq!(error.to_string(), "Execution failed: failed to run");
    }

    #[test]
    fn test_tool_context_workspace() {
        let ctx = ToolContext {
            workspace: std::path::PathBuf::from("/workspace"),
            user_id: "user123".to_string(),
            session_id: "session456".to_string(),
            execution_id: "exec789".to_string(),
            require_approval: true,
            dry_run: false,
            metadata: std::collections::HashMap::new(),
            permissions: ToolPermissions {
                read: true,
                write: false,
                execute: true,
                file_read: true,
                file_write: false,
                network: false,
            },
            rooignore: None,
        };
        
        assert_eq!(ctx.workspace.to_str().unwrap(), "/workspace");
        assert_eq!(ctx.user_id, "user123");
        assert!(ctx.permissions.read);
        assert!(!ctx.permissions.write);
        assert!(ctx.permissions.execute);
    }
}
