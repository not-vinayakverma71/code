//! Capabilities Section
//!
//! 1:1 Translation from Codex `src/core/prompts/sections/capabilities.ts`
//!
//! Reference: /home/verma/lapce/Codex/src/core/prompts/sections/capabilities.ts (lines 1-50)

use std::path::Path;

/// Diff strategy for editing files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffStrategy {
    Unified,
    Wholefile,
}

/// Generate capabilities section
///
/// Translation of getCapabilitiesSection() from capabilities.ts (lines 10-49)
///
/// # Arguments
///
/// * `workspace` - Current workspace directory
/// * `supports_browser` - Whether browser_action tool is available
/// * `has_mcp` - Whether MCP servers are configured
/// * `diff_strategy` - Diff strategy for editing (None means write_to_file only)
/// * `codebase_search_available` - Whether codebase_search is available
/// * `fast_apply_available` - Whether edit_file (Morph) is available
pub fn capabilities_section(
    workspace: &Path,
    supports_browser: bool,
    has_mcp: bool,
    diff_strategy: Option<DiffStrategy>,
    codebase_search_available: bool,
    fast_apply_available: bool,
) -> String {
    let cwd = workspace.display().to_string();
    
    let browser_capability = if supports_browser {
        ", use the browser"
    } else {
        ""
    };
    
    let codebase_search_capability = if codebase_search_available {
        r#"
- You can use the `codebase_search` tool to perform semantic searches across your entire codebase. This tool is powerful for finding functionally relevant code, even if you don't know the exact keywords or file names. It's particularly useful for understanding how features are implemented across multiple files, discovering usages of a particular API, or finding code examples related to a concept. This capability relies on a pre-built index of your code."#
    } else {
        ""
    };
    
    let edit_tool = if fast_apply_available {
        "the edit_file"
    } else if diff_strategy.is_some() {
        "the apply_diff or write_to_file"
    } else {
        "the write_to_file"
    };
    
    let browser_tool_capability = if supports_browser {
        r#"
- You can use the browser_action tool to interact with websites (including html files and locally running development servers) through a Puppeteer-controlled browser when you feel it is necessary in accomplishing the user's task. This tool is particularly useful for web development tasks as it allows you to launch a browser, navigate to pages, interact with elements through clicks and keyboard input, and capture the results through screenshots and console logs. This tool may be useful at key stages of web development tasks-such as after implementing new features, making substantial changes, when troubleshooting issues, or to verify the result of your work. You can analyze the provided screenshots to ensure correct rendering or identify errors, and review console logs for runtime issues.
  - For example, if asked to add a component to a react website, you might create the necessary files, use execute_command to run the site locally, then use browser_action to launch the browser, navigate to the local server, and verify the component renders & functions correctly before closing the browser."#
    } else {
        ""
    };
    
    let mcp_capability = if has_mcp {
        r#"
- You have access to MCP servers that may provide additional tools and resources. Each server may provide different capabilities that you can use to accomplish tasks more effectively.
"#
    } else {
        ""
    };
    
    format!(
        r#"====

CAPABILITIES

- You have access to tools that let you execute CLI commands on the user's computer, list files, view source code definitions, regex search{}, read and write files, and ask follow-up questions. These tools help you effectively accomplish a wide range of tasks, such as writing code, making edits or improvements to existing files, understanding the current state of a project, performing system operations, and much more.
- When the user initially gives you a task, a recursive list of all filepaths in the current workspace directory ('{}') will be included in environment_details. This provides an overview of the project's file structure, offering key insights into the project from directory/file names (how developers conceptualize and organize their code) and file extensions (the language used). This can also guide decision-making on which files to explore further. If you need to further explore directories such as outside the current workspace directory, you can use the list_files tool. If you pass 'true' for the recursive parameter, it will list files recursively. Otherwise, it will list files at the top level, which is better suited for generic directories where you don't necessarily need the nested structure, like the Desktop.{}
- You can use search_files to perform regex searches across files in a specified directory, outputting context-rich results that include surrounding lines. This is particularly useful for understanding code patterns, finding specific implementations, or identifying areas that need refactoring.
- You can use the list_code_definition_names tool to get an overview of source code definitions for all files at the top level of a specified directory. This can be particularly useful when you need to understand the broader context and relationships between certain parts of the code. You may need to call this tool multiple times to understand various parts of the codebase related to the task.
    - For example, when asked to make edits or improvements you might analyze the file structure in the initial environment_details to get an overview of the project, then use list_code_definition_names to get further insight using source code definitions for files located in relevant directories, then read_file to examine the contents of relevant files, analyze the code and suggest improvements or make necessary edits, then use {} tool to apply the changes. If you refactored code that could affect other parts of the codebase, you could use search_files to ensure you update other files as needed.
- You can use the execute_command tool to run commands on the user's computer whenever you feel it can help accomplish the user's task. When you need to execute a CLI command, you must provide a clear explanation of what the command does. Prefer to execute complex CLI commands over creating executable scripts, since they are more flexible and easier to run. Interactive and long-running commands are allowed, since the commands are run in the user's VSCode terminal. The user may keep commands running in the background and you will be kept updated on their status along the way. Each command you execute is run in a new terminal instance.{}{}"#,
        browser_capability,
        cwd,
        codebase_search_capability,
        edit_tool,
        browser_tool_capability,
        mcp_capability
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_capabilities_basic() {
        let workspace = PathBuf::from("/home/user/project");
        let section = capabilities_section(&workspace, false, false, None, false, false);
        
        assert!(section.contains("===="));
        assert!(section.contains("CAPABILITIES"));
        assert!(section.contains("execute CLI commands"));
        assert!(section.contains("/home/user/project"));
        assert!(!section.contains("browser"));
        assert!(!section.contains("MCP"));
        assert!(!section.contains("codebase_search"));
    }
    
    #[test]
    fn test_capabilities_with_browser() {
        let workspace = PathBuf::from("/test");
        let section = capabilities_section(&workspace, true, false, None, false, false);
        
        assert!(section.contains("use the browser"));
        assert!(section.contains("browser_action"));
        assert!(section.contains("Puppeteer-controlled browser"));
    }
    
    #[test]
    fn test_capabilities_with_mcp() {
        let workspace = PathBuf::from("/test");
        let section = capabilities_section(&workspace, false, true, None, false, false);
        
        assert!(section.contains("MCP servers"));
        assert!(section.contains("additional tools and resources"));
    }
    
    #[test]
    fn test_capabilities_with_codebase_search() {
        let workspace = PathBuf::from("/test");
        let section = capabilities_section(&workspace, false, false, None, true, false);
        
        assert!(section.contains("codebase_search"));
        assert!(section.contains("semantic searches"));
        assert!(section.contains("pre-built index"));
    }
    
    #[test]
    fn test_capabilities_edit_tool_variants() {
        let workspace = PathBuf::from("/test");
        
        // No diff strategy - write_to_file only
        let section1 = capabilities_section(&workspace, false, false, None, false, false);
        assert!(section1.contains("use the write_to_file tool"));
        
        // With diff strategy
        let section2 = capabilities_section(&workspace, false, false, Some(DiffStrategy::Unified), false, false);
        assert!(section2.contains("use the apply_diff or write_to_file tool"));
        
        // With fast apply (Morph)
        let section3 = capabilities_section(&workspace, false, false, None, false, true);
        assert!(section3.contains("use the edit_file tool"));
    }
}
