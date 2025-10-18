// Tool registry for O(1) lookup and dispatch - P0-0: Scaffold core layer

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use anyhow::{Result, bail};

use super::traits::{Tool, ToolContext, ToolResult};

/// Metadata about a registered tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub requires_approval: bool,
    pub category: String,
    pub schema: Option<Value>,
}

/// Tool registry for managing and dispatching tools
pub struct ToolRegistry {
    /// HashMap for O(1) lookup by name
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
    
    /// Metadata cache for registered tools
    metadata: Arc<RwLock<HashMap<String, ToolMetadata>>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a tool
    pub fn register<T: Tool + 'static>(&self, tool: T) -> Result<()> {
        let name = tool.name().to_string();
        let metadata = ToolMetadata {
            name: name.clone(),
            description: tool.description().to_string(),
            requires_approval: tool.requires_approval(),
            category: "general".to_string(), // Tools can override this
            schema: None, // Tools can provide parameter schemas
        };
        
        let tool_arc = Arc::new(tool) as Arc<dyn Tool>;
        
        let mut tools = self.tools.write();
        let mut meta = self.metadata.write();
        
        if tools.contains_key(&name) {
            bail!("Tool '{}' already registered", name);
        }
        
        tools.insert(name.clone(), tool_arc);
        meta.insert(name, metadata);
        
        Ok(())
    }
    
    /// Get a tool by name (O(1) lookup)
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().get(name).cloned()
    }
    
    /// Check if a tool exists
    pub fn contains(&self, name: &str) -> bool {
        self.tools.read().contains_key(name)
    }
    
    /// Get metadata for a tool
    pub fn get_metadata(&self, name: &str) -> Option<ToolMetadata> {
        self.metadata.read().get(name).cloned()
    }
    
    /// List all registered tools
    pub fn list_tools(&self) -> Vec<ToolMetadata> {
        self.metadata.read().values().cloned().collect()
    }
    
    /// Execute a tool by name
    pub async fn execute(
        &self,
        name: &str,
        args: Value,
        context: ToolContext
    ) -> Result<ToolResult> {
        let tool = self.get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", name))?;
        
        // Validate arguments
        tool.validate_args(&args)?;
        
        // Execute the tool
        Ok(tool.execute(args, context).await)
    }
    
    /// Get count of registered tools
    pub fn count(&self) -> usize {
        self.tools.read().len()
    }
    
    /// Clear all registered tools (useful for testing)
    pub fn clear(&self) {
        self.tools.write().clear();
        self.metadata.write().clear();
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::time::Instant;
    use tempfile::TempDir;
    
    struct TestTool {
        name: &'static str,
    }
    
    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &'static str {
            self.name
        }
        
        fn description(&self) -> &'static str {
            "Test tool"
        }
        
        async fn execute(&self, _args: Value, _context: ToolContext) -> ToolResult {
            Ok(crate::core::tools::traits::ToolOutput::success(
                serde_json::json!({"name": self.name})
            ))
        }
    }
    
    #[test]
    fn test_registry_registration() {
        let registry = ToolRegistry::new();
        let tool = TestTool { name: "test_tool" };
        
        registry.register(tool).unwrap();
        assert!(registry.contains("test_tool"));
        assert_eq!(registry.count(), 1);
        
        // Duplicate registration should fail
        let tool2 = TestTool { name: "test_tool" };
        assert!(registry.register(tool2).is_err());
    }
    
    #[test]
    fn test_registry_lookup_performance() {
        let registry = ToolRegistry::new();
        
        // Register 1000 tools
        for i in 0..1000 {
            let tool = TestTool {
                name: Box::leak(format!("tool_{}", i).into_boxed_str()),
            };
            registry.register(tool).unwrap();
        }
        
        // Measure lookup time
        let start = Instant::now();
        for _ in 0..10000 {
            let _ = registry.get("tool_500");
        }
        let elapsed = start.elapsed();
        
        // Average lookup time should be < 2 microseconds
        let avg_lookup_ns = elapsed.as_nanos() / 10000;
        assert!(
            avg_lookup_ns < 2000,
            "Lookup took {} ns, expected < 2000 ns",
            avg_lookup_ns
        );
    }
    
    #[tokio::test]
    async fn test_registry_execution() {
        let registry = ToolRegistry::new();
        let tool = TestTool { name: "exec_test" };
        registry.register(tool).unwrap();
        
        let temp_dir = TempDir::new().unwrap();
        let context = ToolContext::new(
            temp_dir.path().to_path_buf(),
            "test_user".to_string()
        );
        
        let result = registry.execute(
            "exec_test",
            Value::Null,
            context
        ).await.unwrap();
        
        let output = result.unwrap();
        assert!(output.success);
        assert_eq!(output.result.get("name").unwrap(), "exec_test");
    }
    
    #[test]
    fn test_registry_metadata() {
        let registry = ToolRegistry::new();
        let tool = TestTool { name: "meta_test" };
        registry.register(tool).unwrap();
        
        let metadata = registry.get_metadata("meta_test").unwrap();
        assert_eq!(metadata.name, "meta_test");
        assert_eq!(metadata.description, "Test tool");
        assert!(!metadata.requires_approval);
    }
    
    #[test]
    fn test_registry_list_tools() {
        let registry = ToolRegistry::new();
        
        for i in 0..5 {
            let tool = TestTool {
                name: Box::leak(format!("list_tool_{}", i).into_boxed_str()),
            };
            registry.register(tool).unwrap();
        }
        
        let tools = registry.list_tools();
        assert_eq!(tools.len(), 5);
        
        let names: Vec<_> = tools.iter().map(|t| &t.name).collect();
        assert!(names.contains(&&"list_tool_0".to_string()));
        assert!(names.contains(&&"list_tool_4".to_string()));
    }
}
