use crate::types_tool::ToolParameter;
use crate::mcp_tools::{core::{Tool, ToolContext, ToolResult, JsonSchema, ResourceLimits}, permissions::Permission};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use std::collections::HashMap;

pub struct AccessMcpResourceTool;
impl AccessMcpResourceTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for AccessMcpResourceTool {
    fn name(&self) -> &str { "accessMcpResource" }
    fn description(&self) -> &str { "Access MCP resources like configurations, schemas, and capabilities" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "resource_type": {
                    "type": "string",
                    "enum": ["config", "schema", "capability", "metadata", "status"],
                    "description": "Type of resource to access"
                },
                "resource_name": {
                    "type": "string",
                    "description": "Name of the specific resource"
                },
                "operation": {
                    "enum": ["read", "list", "describe"],
                    "description": "Operation to perform"
                }
            },
            "required": ["resource_type", "operation"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("resource_type").is_none() || args.get("operation").is_none() {
            anyhow::bail!("Missing required parameters: resource_type and operation");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> Result<ToolResult> {
        let (resource_type, resource_name, operation) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let resource_type = tool_use.params.get("resource_type")
                .ok_or_else(|| anyhow::anyhow!("Missing resource_type in XML"))?
                .clone();
            let resource_name = tool_use.params.get("resource_name").map(|s| s.clone());
            let operation = tool_use.params.get("operation")
                .ok_or_else(|| anyhow::anyhow!("Missing operation in XML"))?
                .clone();
            (resource_type, resource_name, operation)
        } else {
            let resource_type = args.get("resource_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid resource_type"))?
                .to_string();
            let resource_name = args.get("resource_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let operation = args.get("operation")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid operation"))?
                .to_string();
            (resource_type, resource_name, operation)
        };
        
        let result = match (resource_type.as_str(), operation.as_str()) {
            ("config", "list") => {
                let configs = vec![
                    "rate_limit_config",
                    "permission_config",
                    "resource_limits_config",
                    "tool_registry_config"
                ];
                format!("<configs>{}</configs>", 
                    configs.iter().map(|c| format!("<config>{}</config>", c)).collect::<Vec<_>>().join(""))
            }
            ("config", "read") => {
                let name = resource_name.unwrap_or_else(|| "default".to_string());
                match name.as_str() {
                    "rate_limit_config" => "<config><name>rate_limit</name><capacity>100</capacity><refill_rate>10</refill_rate></config>".to_string(),
                    "permission_config" => "<config><name>permissions</name><default_role>user</default_role><strict_mode>true</strict_mode></config>".to_string(),
                    _ => format!("<config><name>{}</name><value>default</value></config>", name)
                }
            }
            ("schema", "list") => {
                let schemas = vec![
                    "tool_input_schema",
                    "tool_output_schema",
                    "permission_schema",
                    "resource_schema"
                ];
                format!("<schemas>{}</schemas>",
                    schemas.iter().map(|s| format!("<schema>{}</schema>", s)).collect::<Vec<_>>().join(""))
            }
            ("capability", "list") => {
                let capabilities = vec![
                    "file_operations",
                    "git_operations",
                    "http_requests",
                    "code_analysis",
                    "task_management"
                ];
                format!("<capabilities>{}</capabilities>",
                    capabilities.iter().map(|c| format!("<capability>{}</capability>", c)).collect::<Vec<_>>().join(""))
            }
            ("metadata", "read") => {
                "<metadata><version>1.0.0</version><tools_count>29</tools_count><memory_limit>3MB</memory_limit><dispatch_target>10ms</dispatch_target></metadata>".to_string()
            }
            ("status", "read") => {
                "<status><healthy>true</healthy><uptime>100</uptime><active_sessions>1</active_sessions><total_requests>0</total_requests></status>".to_string()
            }
            _ => {
                return Ok(ToolResult::from_xml(format!(
                    "<tool_result><success>false</success><error>Unknown operation: {} for resource: {}</error></tool_result>",
                    operation, resource_type
                )));
            }
        };
        
        let xml_response = format!(
            "<tool_result><success>true</success><resource_type>{}</resource_type><operation>{}</operation>{}</tool_result>",
            resource_type,
            operation,
            result
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileRead("*".to_string())]
    }
    
    fn resource_limits(&self) -> ResourceLimits {
        ResourceLimits::default()
    }
}
