//! Tool Use Section
//!
//! 1:1 Translation from Codex `src/core/prompts/sections/tool-use.ts`
//!
//! Reference: /home/verma/lapce/Codex/src/core/prompts/sections/tool-use.ts (lines 1-20)

/// Generate shared tool use section
///
/// Translation of getSharedToolUseSection() from tool-use.ts
pub fn shared_tool_use_section() -> String {
    r#"====

TOOL USE

You have access to a set of tools that are executed upon the user's approval. You must use exactly one tool per message, and every assistant message must include a tool call. You use tools step-by-step to accomplish a given task, with each tool use informed by the result of the previous tool use.

# Tool Use Formatting

Tool uses are formatted using XML-style tags. The tool name itself becomes the XML tag name. Each parameter is enclosed within its own set of tags. Here's the structure:

<actual_tool_name>
<parameter1_name>value1</parameter1_name>
<parameter2_name>value2</parameter2_name>
...
</actual_tool_name>

Always use the actual tool name as the XML tag name for proper parsing and execution."#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_shared_tool_use_section() {
        let section = shared_tool_use_section();
        
        assert!(section.contains("===="));
        assert!(section.contains("TOOL USE"));
        assert!(section.contains("exactly one tool per message"));
        assert!(section.contains("XML-style tags"));
        assert!(section.contains("<actual_tool_name>"));
        assert!(section.contains("</actual_tool_name>"));
    }
}
