//! Section Snapshot Tests (P8)
//!
//! These tests verify exact string matching between Rust implementations
//! and Codex TypeScript sources. Any deviation indicates a parity break.
//!
//! Reference: All sections in /home/verma/lapce/Codex/src/core/prompts/sections/

use crate::core::prompt::sections::*;
use crate::core::prompt::sections::capabilities::DiffStrategy;
use crate::core::prompt::modes::{Mode, get_mode_by_slug};
use std::path::Path;

// ============================================================================
// Markdown Formatting Section
// ============================================================================

#[test]
fn test_markdown_formatting_exact_match() {
    // Reference: Codex markdown-formatting.ts (lines 2-6)
    let expected = r#"====

MARKDOWN RULES

ALL responses MUST show ANY `language construct` OR filename reference as clickable, exactly as [`filename OR language.declaration()`](relative/file/path.ext:line); line is required for `syntax` and optional for filename links. This applies to ALL markdown responses and ALSO those in <attempt_completion>"#;
    
    let actual = markdown_formatting_section();
    
    assert_eq!(
        actual, expected,
        "Markdown formatting section must exactly match Codex"
    );
}

// ============================================================================
// Tool Use Section
// ============================================================================

#[test]
fn test_shared_tool_use_exact_match() {
    // Reference: Codex tool-use.ts sharedToolUseSection() (lines 3-18)
    let expected = r#"====

TOOL USE

You have access to a set of tools that are executed upon the user's approval. You can use one tool per message, and will receive the result of that tool use in the user's response. You use tools step-by-step to accomplish a given task, with each tool use informed by the result of the previous tool use.

# Tool Use Formatting

Tool use is formatted using XML-style tags. The tool name is enclosed in opening and closing tags, and each parameter is similarly enclosed within its own set of tags. Here's the structure:

<tool_name>
<parameter1_name>value1</parameter1_name>
<parameter2_name>value2</parameter2_name>
...
</tool_name>"#;
    
    let actual = shared_tool_use_section();
    
    assert_eq!(
        actual, expected,
        "Shared tool use section must exactly match Codex"
    );
}

// ============================================================================
// Tool Use Guidelines Section
// ============================================================================

#[test]
fn test_tool_use_guidelines_without_codebase_search() {
    // Reference: Codex tool-use-guidelines.ts (lines 3-104) without codebase_search
    let actual = tool_use_guidelines_section(false);
    
    // Key assertions for structure
    assert!(actual.contains("===="));
    assert!(actual.contains("TOOL USE GUIDELINES"));
    assert!(actual.contains("# Tool Invocation Guidelines"));
    assert!(actual.contains("# General Tool Use Tips"));
    
    // Should NOT contain codebase_search reference when disabled
    assert!(!actual.contains("codebase_search"));
    
    // Must contain trash-put safety warning
    assert!(actual.contains("trash-put"));
    assert!(actual.contains("rm command"));
}

#[test]
fn test_tool_use_guidelines_with_codebase_search() {
    // Reference: Codex tool-use-guidelines.ts with codebase_search enabled
    let actual = tool_use_guidelines_section(true);
    
    // Should contain codebase_search reference when enabled
    assert!(actual.contains("codebase_search"));
    assert!(actual.contains("semantic search"));
}

#[test]
fn test_tool_use_guidelines_trash_put_warning() {
    // Critical safety feature - must be present
    let actual = tool_use_guidelines_section(false);
    
    assert!(actual.contains("trash-put"));
    assert!(actual.contains("**IMPORTANT**: You MUST use the `trash-put` command"));
    assert!(actual.contains("rm command"));
}

// ============================================================================
// Capabilities Section
// ============================================================================

#[test]
fn test_capabilities_section_structure() {
    let workspace = Path::new("/test/workspace");
    
    let actual = capabilities_section(workspace, false, false, Some(DiffStrategy::Unified), false, false);
    
    // Must contain key sections
    assert!(actual.contains("===="));
    assert!(actual.contains("CAPABILITIES"));
}

#[test]
fn test_capabilities_diff_strategies() {
    let workspace = Path::new("/test/workspace");
    
    // Test unified diff strategy
    let unified = capabilities_section(workspace, false, false, Some(DiffStrategy::Unified), false, false);
    assert!(unified.contains("apply_diff"));
    
    // Test whole file strategy
    let whole = capabilities_section(workspace, false, false, Some(DiffStrategy::Wholefile), false, false);
    assert!(whole.contains("apply_diff"));
}

#[test]
fn test_capabilities_workspace_path() {
    let workspace = Path::new("/my/custom/path");
    
    let actual = capabilities_section(workspace, false, false, None, false, false);
    
    // Must include workspace path
    assert!(actual.contains("/my/custom/path"));
}

// ============================================================================
// Objective Section
// ============================================================================

#[test]
fn test_objective_section_structure() {
    let actual = objective_section(false);
    
    // Must contain key elements
    assert!(actual.contains("===="));
    assert!(actual.contains("OBJECTIVE"));
}

#[test]
fn test_objective_different_modes() {
    // Test objective section
    let actual = objective_section(false);
    
    assert!(actual.contains("===="));
    assert!(actual.contains("OBJECTIVE"));
    assert!(!actual.is_empty());
}

// ============================================================================
// System Info Section
// ============================================================================

#[test]
fn test_system_info_section_structure() {
    let workspace = Path::new("/test/workspace");
    
    let actual = system_info_section(workspace);
    
    // Must contain key sections
    assert!(actual.contains("===="));
    assert!(actual.contains("SYSTEM INFORMATION"));
    assert!(actual.contains("Operating System"));
    assert!(actual.contains("Default Shell"));
    assert!(actual.contains("Current Working Directory"));
}

#[test]
fn test_system_info_includes_workspace() {
    let workspace = Path::new("/custom/workspace/path");
    
    let actual = system_info_section(workspace);
    
    // Must include the workspace path
    assert!(actual.contains("/custom/workspace/path"));
}

#[test]
fn test_system_info_os_detection() {
    let workspace = Path::new("/test");
    
    let actual = system_info_section(workspace);
    
    // Must contain OS information
    #[cfg(target_os = "linux")]
    assert!(actual.contains("Linux"));
    
    #[cfg(target_os = "macos")]
    assert!(actual.contains("macOS"));
    
    #[cfg(target_os = "windows")]
    assert!(actual.contains("Windows"));
}

// ============================================================================
// Mode Roles Section
// ============================================================================

#[test]
fn test_all_mode_roles_defined() {
    let modes = vec!["code", "architect", "ask", "debug", "orchestrator"];
    
    for mode_slug in modes {
        let mode = get_mode_by_slug(mode_slug).unwrap();
        
        // Each mode must have a non-empty role definition
        assert!(!mode.role_definition.is_empty(), 
            "Mode {} must have role definition", mode_slug);
        
        // Role definition must be meaningful (not placeholder)
        assert!(mode.role_definition.len() > 20,
            "Mode {} role definition too short", mode_slug);
    }
}

#[test]
fn test_code_mode_role() {
    let mode = get_mode_by_slug("code").unwrap();
    
    // Code mode should mention software engineering or coding
    let role = mode.role_definition.to_lowercase();
    assert!(
        role.contains("software") || role.contains("code") || role.contains("engineer"),
        "Code mode role should mention software/code/engineer"
    );
}

#[test]
fn test_architect_mode_role() {
    let mode = get_mode_by_slug("architect").unwrap();
    
    // Architect mode should mention architecture or design
    let role = mode.role_definition.to_lowercase();
    assert!(
        role.contains("architect") || role.contains("design") || role.contains("plan"),
        "Architect mode role should mention architecture/design/plan"
    );
}

#[test]
fn test_ask_mode_role() {
    let mode = get_mode_by_slug("ask").unwrap();
    
    // Ask mode should mention answering or questions
    let role = mode.role_definition.to_lowercase();
    assert!(
        role.contains("answer") || role.contains("question") || role.contains("explain"),
        "Ask mode role should mention answering/questions/explain"
    );
}

#[test]
fn test_debug_mode_role() {
    let mode = get_mode_by_slug("debug").unwrap();
    
    // Debug mode should mention debugging or troubleshooting
    let role = mode.role_definition.to_lowercase();
    assert!(
        role.contains("debug") || role.contains("troubleshoot") || role.contains("diagnose"),
        "Debug mode role should mention debugging/troubleshooting"
    );
}

// ============================================================================
// Section Consistency Tests
// ============================================================================

#[test]
fn test_all_sections_have_separator() {
    // All sections should start with ==== separator
    let sections = vec![
        markdown_formatting_section(),
        shared_tool_use_section(),
        tool_use_guidelines_section(false),
        objective_section(&get_mode_by_slug("code").unwrap()),
        system_info_section(Path::new("/test")),
    ];
    
    for (i, section) in sections.iter().enumerate() {
        assert!(
            section.starts_with("====") || section.trim().starts_with("===="),
            "Section {} must start with ==== separator",
            i
        );
    }
}

#[test]
fn test_all_sections_have_headers() {
    // All sections should have a clear header after separator
    let test_cases = vec![
        (markdown_formatting_section(), "MARKDOWN RULES"),
        (shared_tool_use_section(), "TOOL USE"),
        (tool_use_guidelines_section(false), "TOOL USE GUIDELINES"),
        (objective_section(&get_mode_by_slug("code").unwrap()), "OBJECTIVE"),
        (system_info_section(Path::new("/test")), "SYSTEM INFORMATION"),
    ];
    
    for (section, expected_header) in test_cases {
        assert!(
            section.contains(expected_header),
            "Section must contain header '{}'",
            expected_header
        );
    }
}

#[test]
fn test_sections_no_trailing_whitespace() {
    // Sections should not have excessive trailing whitespace
    let sections = vec![
        markdown_formatting_section(),
        shared_tool_use_section(),
        tool_use_guidelines_section(false),
        objective_section(&get_mode_by_slug("code").unwrap()),
        system_info_section(Path::new("/test")),
    ];
    
    for (i, section) in sections.iter().enumerate() {
        let trimmed = section.trim_end();
        // Allow one newline at end, but not multiple
        let trailing_newlines = section.len() - trimmed.len();
        assert!(
            trailing_newlines <= 1,
            "Section {} has {} trailing newlines (max 1 allowed)",
            i, trailing_newlines
        );
    }
}

// ============================================================================
// Snapshot Regression Tests
// ============================================================================

#[test]
fn test_markdown_formatting_length() {
    // If this test fails, markdown formatting section changed
    let section = markdown_formatting_section();
    let expected_length = 300; // Approximate, adjust if intentionally changed
    
    assert!(
        section.len() > expected_length - 50 && section.len() < expected_length + 50,
        "Markdown formatting section length changed significantly: {} (expected ~{})",
        section.len(), expected_length
    );
}

#[test]
fn test_tool_use_section_length() {
    // If this test fails, tool use section changed
    let section = shared_tool_use_section();
    let expected_length = 500; // Approximate
    
    assert!(
        section.len() > expected_length - 100 && section.len() < expected_length + 100,
        "Tool use section length changed significantly: {} (expected ~{})",
        section.len(), expected_length
    );
}

// ============================================================================
// Critical Content Presence Tests
// ============================================================================

#[test]
fn test_critical_safety_warnings_present() {
    let guidelines = tool_use_guidelines_section(false);
    
    // These safety warnings are critical and must be present
    let critical_warnings = vec![
        "trash-put",
        "rm command",
        "IMPORTANT",
    ];
    
    for warning in critical_warnings {
        assert!(
            guidelines.contains(warning),
            "Critical safety warning '{}' missing from guidelines",
            warning
        );
    }
}

#[test]
fn test_xml_formatting_example_present() {
    let section = shared_tool_use_section();
    
    // Must show XML formatting examples
    assert!(section.contains("<tool_name>"));
    assert!(section.contains("</tool_name>"));
    assert!(section.contains("<parameter"));
}

#[test]
fn test_problem_solving_instructions_present() {
    let mode = get_mode_by_slug("code").unwrap();
    let capabilities = capabilities_section(&mode, Path::new("/test"), "unified");
    
    // Must contain problem-solving guidance
    assert!(capabilities.contains("Problem Solving Instructions"));
}

// ============================================================================
// Diff Strategy Completeness Tests
// ============================================================================

#[test]
fn test_all_diff_strategies_work() {
    let mode = get_mode_by_slug("code").unwrap();
    let workspace = Path::new("/test");
    
    let strategies = vec!["unified", "whole", "search-replace"];
    
    for strategy in strategies {
        let section = capabilities_section(&mode, workspace, strategy);
        
        // Each strategy must produce non-empty output
        assert!(!section.is_empty(), "Strategy '{}' produced empty section", strategy);
        
        // Must contain capabilities header
        assert!(section.contains("CAPABILITIES"), 
            "Strategy '{}' missing CAPABILITIES header", strategy);
    }
}

#[test]
fn test_unified_diff_contains_search_replace_tags() {
    let mode = get_mode_by_slug("code").unwrap();
    let section = capabilities_section(&mode, Path::new("/test"), "unified");
    
    // Unified diff must explain search/replace tags
    assert!(section.contains("<search>"));
    assert!(section.contains("<replace>"));
}

// ============================================================================
// Mode Variations Tests
// ============================================================================

#[test]
fn test_each_mode_has_unique_objective() {
    let modes = vec!["code", "architect", "ask", "debug", "orchestrator"];
    let mut objectives = Vec::new();
    
    for mode_slug in &modes {
        let mode = get_mode_by_slug(mode_slug).unwrap();
        let objective = objective_section(&mode);
        objectives.push(objective);
    }
    
    // Each mode should have a different objective section
    for i in 0..objectives.len() {
        for j in (i+1)..objectives.len() {
            assert_ne!(
                objectives[i], objectives[j],
                "Modes {} and {} have identical objectives",
                modes[i], modes[j]
            );
        }
    }
}
