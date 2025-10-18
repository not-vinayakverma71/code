//! Objective Section
//!
//! 1:1 Translation from Codex `src/core/prompts/sections/objective.ts`
//!
//! Reference: /home/verma/lapce/Codex/src/core/prompts/sections/objective.ts (lines 1-29)

/// Generate objective section
///
/// Translation of getObjectiveSection() from objective.ts (lines 3-28)
///
/// # Arguments
///
/// * `codebase_search_available` - Whether codebase_search tool is available (code index enabled)
pub fn objective_section(codebase_search_available: bool) -> String {
    let codebase_search_instruction = if codebase_search_available {
        "First, for ANY exploration of code you haven't examined yet in this conversation, you MUST use the `codebase_search` tool to search for relevant code based on the task's intent BEFORE using any other search or file exploration tools. This applies throughout the entire task, not just at the beginning - whenever you need to explore a new area of code, codebase_search must come first. Then, "
    } else {
        "First, "
    };
    
    format!(
        r#"====

OBJECTIVE

You accomplish a given task iteratively, breaking it down into clear steps and working through them methodically.

1. Analyze the user's task and set clear, achievable goals to accomplish it. Prioritize these goals in a logical order.
2. Work through these goals sequentially, utilizing available tools one at a time as necessary. Each goal should correspond to a distinct step in your problem-solving process. You will be informed on the work completed and what's remaining as you go.
3. Remember, you have extensive capabilities with access to a wide range of tools that can be used in powerful and clever ways as necessary to accomplish each goal. Before calling a tool, do some analysis. {}analyze the file structure provided in environment_details to gain context and insights for proceeding effectively. Next, think about which of the provided tools is the most relevant tool to accomplish the user's task. Go through each of the required parameters of the relevant tool and determine if the user has directly provided or given enough information to infer a value. When deciding if the parameter can be inferred, carefully consider all the context to see if it supports a specific value. If all of the required parameters are present or can be reasonably inferred, proceed with the tool use. BUT, if one of the values for a required parameter is missing, DO NOT invoke the tool (not even with fillers for the missing params) and instead, ask the user to provide the missing parameters using the ask_followup_question tool. DO NOT ask for more information on optional parameters if it is not provided.
4. Once you've completed the user's task, you must use the attempt_completion tool to present the result of the task to the user.
5. The user may provide feedback, which you can use to make improvements and try again. But DO NOT continue in pointless back and forth conversations, i.e. don't end your responses with questions or offers for further assistance."#,
        codebase_search_instruction
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_objective_section_without_codebase_search() {
        let section = objective_section(false);
        
        assert!(section.contains("===="));
        assert!(section.contains("OBJECTIVE"));
        assert!(section.contains("iteratively"));
        assert!(section.contains("First, analyze the file structure"));
        assert!(!section.contains("codebase_search"));
    }
    
    #[test]
    fn test_objective_section_with_codebase_search() {
        let section = objective_section(true);
        
        assert!(section.contains("===="));
        assert!(section.contains("OBJECTIVE"));
        assert!(section.contains("codebase_search"));
        assert!(section.contains("MUST use the `codebase_search` tool"));
        assert!(section.contains("BEFORE using any other search"));
    }
    
    #[test]
    fn test_objective_has_all_steps() {
        let section = objective_section(false);
        
        assert!(section.contains("1. Analyze the user's task"));
        assert!(section.contains("2. Work through these goals"));
        assert!(section.contains("3. Remember, you have extensive"));
        assert!(section.contains("4. Once you've completed"));
        assert!(section.contains("5. The user may provide feedback"));
        assert!(section.contains("attempt_completion"));
    }
}
