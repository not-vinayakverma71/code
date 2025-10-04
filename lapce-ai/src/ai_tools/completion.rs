use crate::types_tool::ToolParameter;
use crate::mcp_tools::{
    core::{Tool, ToolContext, ToolResult, ResourceLimits},
    permissions::Permission,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::Result;
use chrono::Utc;

pub struct CompletionHandler;
impl CompletionHandler {
    pub fn new() -> Self { Self }
}

pub struct AttemptCompletionTool;
impl AttemptCompletionTool {
    pub fn new() -> Self { Self }
}
#[async_trait]
impl Tool for AttemptCompletionTool {
    fn name(&self) -> &str { "attemptCompletion" }
    fn description(&self) -> &str { "Attempt to complete a task and verify success criteria" }
    fn parameters(&self) -> Vec<crate::mcp_tools::core::ToolParameter> { vec![] }
    fn input_schema(&self) -> Value { 
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Task description"
                },
                "result": {
                    "type": "string",
                    "description": "Result of the task"
                },
                "success_criteria": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "criterion": {"type": "string"},
                            "met": {"type": "boolean"}
                        }
                    },
                    "description": "Success criteria and whether they were met"
                },
                "confidence": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 1,
                    "description": "Confidence level (0-1)"
                }
            },
            "required": ["task", "result"]
        })
    }
    
    async fn validate(&self, args: &Value) -> Result<()> {
        if !args.is_object() || args.get("task").is_none() || args.get("result").is_none() {
            anyhow::bail!("Missing required parameters: task and result");
        }
        Ok(())
    }
    
    async fn execute(&self, args: Value, _context: ToolContext) -> Result<ToolResult> {
        let (task, result, criteria, confidence) = if let Some(xml_str) = args.as_str() {
            let tool_use = crate::mcp_tools::xml::parse_tool_use(xml_str)?;
            let task = tool_use.params.get("task")
                .ok_or_else(|| anyhow::anyhow!("Missing task in XML"))?
                .clone();
            let result = tool_use.params.get("result")
                .ok_or_else(|| anyhow::anyhow!("Missing result in XML"))?
                .clone();
            let criteria: Vec<(String, bool)> = tool_use.params.get("success_criteria")
                .and_then(|s| serde_json::from_str::<Vec<Value>>(s).ok())
                .map(|arr| {
                    arr.iter().filter_map(|v| {
                        let criterion = v.get("criterion")?.as_str()?;
                        let met = v.get("met")?.as_bool()?;
                        Some((criterion.to_string(), met))
                    }).collect()
                })
                .unwrap_or_default();
            let confidence = tool_use.params.get("confidence")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.8);
            (task, result, criteria, confidence)
        } else {
            let task = args.get("task")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid task"))?
                .to_string();
            let result = args.get("result")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid result"))?
                .to_string();
            let criteria: Vec<(String, bool)> = args.get("success_criteria")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter().filter_map(|v| {
                        let criterion = v.get("criterion")?.as_str()?;
                        let met = v.get("met")?.as_bool()?;
                        Some((criterion.to_string(), met))
                    }).collect()
                })
                .unwrap_or_default();
            let confidence = args.get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.8);
            (task, result, criteria, confidence)
        };
        
        // Calculate completion status
        let criteria_met = if criteria.is_empty() {
            confidence > 0.7
        } else {
            let met_count = criteria.iter().filter(|(_, met)| *met).count();
            let total = criteria.len();
            (met_count as f64 / total as f64) >= 0.8
        };
        
        let completion_status = if criteria_met && confidence >= 0.8 {
            "completed"
        } else if criteria_met && confidence >= 0.6 {
            "partially_completed"
        } else {
            "incomplete"
        };
        // Build criteria report
        let mut criteria_xml = String::new();
        if !criteria.is_empty() {
            criteria_xml.push_str("<criteria>");
            for (criterion, met) in criteria {
                criteria_xml.push_str(&format!(
                    "<criterion><description>{}</description><met>{}</met></criterion>",
                    html_escape::encode_text(&criterion),
                    met
                ));
            }
            criteria_xml.push_str("</criteria>");
        }
        
        let timestamp = Utc::now().to_rfc3339();
        let xml_response = format!(
            "<tool_result><success>true</success><task>{}</task><result>{}</result><status>{}</status><confidence>{}</confidence>{}<timestamp>{}</timestamp></tool_result>",
            html_escape::encode_text(&task),
            html_escape::encode_text(&result),
            completion_status,
            confidence,
            criteria_xml,
            timestamp
        );
        Ok(ToolResult::from_xml(xml_response))
    }
    
    fn required_permissions(&self) -> Vec<Permission> {
        vec![Permission::FileWrite("*".to_string()), Permission::FileWrite("*".to_string())]
    }
}
