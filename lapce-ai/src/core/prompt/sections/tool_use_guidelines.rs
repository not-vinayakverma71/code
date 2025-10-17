//! Tool Use Guidelines Section
//!
//! 1:1 Translation from Codex `src/core/prompts/sections/tool-use-guidelines.ts`
//!
//! Reference: /home/verma/lapce/Codex/src/core/prompts/sections/tool-use-guidelines.ts (lines 1-60)

/// Generate tool use guidelines section
///
/// Translation of getToolUseGuidelinesSection() from tool-use-guidelines.ts (lines 3-59)
///
/// # Arguments
///
/// * `codebase_search_available` - Whether codebase_search tool is available
pub fn tool_use_guidelines_section(codebase_search_available: bool) -> String {
    let mut item_number = 1;
    let mut guidelines = Vec::new();
    
    // First guideline is always the same
    guidelines.push(format!(
        "{}. Assess what information you already have and what information you need to proceed with the task.",
        item_number
    ));
    item_number += 1;
    
    // Conditional codebase search guideline
    if codebase_search_available {
        guidelines.push(format!(
            "{}. **CRITICAL: For ANY exploration of code you haven't examined yet in this conversation, you MUST use the `codebase_search` tool FIRST before any other search or file exploration tools.** This applies throughout the entire conversation, not just at the beginning. The codebase_search tool uses semantic search to find relevant code based on meaning rather than just keywords, making it far more effective than regex-based search_files for understanding implementations. Even if you've already explored some code, any new area of exploration requires codebase_search first.",
            item_number
        ));
        item_number += 1;
        
        guidelines.push(format!(
            "{}. Choose the most appropriate tool based on the task and the tool descriptions provided. After using codebase_search for initial exploration of any new code area, you may then use more specific tools like search_files (for regex patterns), list_files, or read_file for detailed examination. For example, using the list_files tool is more effective than running a command like `ls` in the terminal. It's critical that you think about each available tool and use the one that best fits the current step in the task.",
            item_number
        ));
        item_number += 1;
    } else {
        guidelines.push(format!(
            "{}. Choose the most appropriate tool based on the task and the tool descriptions provided. Assess if you need additional information to proceed, and which of the available tools would be most effective for gathering this information. For example using the list_files tool is more effective than running a command like `ls` in the terminal. It's critical that you think about each available tool and use the one that best fits the current step in the task.",
            item_number
        ));
        item_number += 1;
    }
    
    // Remaining guidelines
    guidelines.push(format!(
        "{}. If multiple actions are needed, use one tool at a time per message to accomplish the task iteratively, with each tool use being informed by the result of the previous tool use. Do not assume the outcome of any tool use. Each step must be informed by the previous step's result.",
        item_number
    ));
    item_number += 1;
    
    guidelines.push(format!(
        "{}. Formulate your tool use using the XML format specified for each tool.",
        item_number
    ));
    item_number += 1;
    
    guidelines.push(format!(
        "{}. After each tool use, the user will respond with the result of that tool use. This result will provide you with the necessary information to continue your task or make further decisions. This response may include:
  - Information about whether the tool succeeded or failed, along with any reasons for failure.
  - Linter errors that may have arisen due to the changes you made, which you'll need to address.
  - New terminal output in reaction to the changes, which you may need to consider or act upon.
  - Any other relevant feedback or information related to the tool use.",
        item_number
    ));
    item_number += 1;
    
    guidelines.push(format!(
        "{}. ALWAYS wait for user confirmation after each tool use before proceeding. Never assume the success of a tool use without explicit confirmation of the result from the user.",
        item_number
    ));
    
    format!(
        r#"# Tool Use Guidelines

{}

It is crucial to proceed step-by-step, waiting for the user's message after each tool use before moving forward with the task. This approach allows you to:
1. Confirm the success of each step before proceeding.
2. Address any issues or errors that arise immediately.
3. Adapt your approach based on new information or unexpected results.
4. Ensure that each action builds correctly on the previous ones.

By waiting for and carefully considering the user's response after each tool use, you can react accordingly and make informed decisions about how to proceed with the task. This iterative process helps ensure the overall success and accuracy of your work."#,
        guidelines.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_use_guidelines_without_codebase_search() {
        let section = tool_use_guidelines_section(false);
        
        assert!(section.contains("# Tool Use Guidelines"));
        assert!(section.contains("1. Assess what information"));
        assert!(section.contains("2. Choose the most appropriate"));
        assert!(section.contains("3. If multiple actions"));
        assert!(!section.contains("codebase_search"));
        assert!(section.contains("step-by-step"));
    }
    
    #[test]
    fn test_tool_use_guidelines_with_codebase_search() {
        let section = tool_use_guidelines_section(true);
        
        assert!(section.contains("# Tool Use Guidelines"));
        assert!(section.contains("1. Assess what information"));
        assert!(section.contains("2. **CRITICAL"));
        assert!(section.contains("codebase_search"));
        assert!(section.contains("MUST use the `codebase_search` tool FIRST"));
        assert!(section.contains("3. Choose the most appropriate"));
        assert!(section.contains("After using codebase_search"));
    }
    
    #[test]
    fn test_numbering_consistency() {
        let section_without = tool_use_guidelines_section(false);
        let section_with = tool_use_guidelines_section(true);
        
        // Without codebase_search should have fewer numbered items
        assert!(section_without.contains("6. ALWAYS wait"));
        
        // With codebase_search should have more items
        assert!(section_with.contains("7. ALWAYS wait"));
    }
    
    #[test]
    fn test_footer_present() {
        let section = tool_use_guidelines_section(false);
        
        assert!(section.contains("It is crucial to proceed step-by-step"));
        assert!(section.contains("1. Confirm the success"));
        assert!(section.contains("2. Address any issues"));
        assert!(section.contains("3. Adapt your approach"));
        assert!(section.contains("4. Ensure that each action"));
    }
}
