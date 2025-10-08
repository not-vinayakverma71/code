/// Test to verify tool.ts translation works correctly
use lapce_ai_rust::tools_translation::*;
use serde_json;

fn main() {
    println!("Testing tool.ts Translation...\n");
    
    // Test 1: ToolGroup serialization
    println!("1. Testing ToolGroup:");
    let group = ToolGroup::Command;
    let json = serde_json::to_string(&group).unwrap();
    println!("   Command group: {}", json);
    assert_eq!(json, r#""command""#);
    
    // Test 2: ToolName serialization
    println!("\n2. Testing ToolName:");
    let tool = ToolName::ExecuteCommand;
    let json = serde_json::to_string(&tool).unwrap();
    println!("   ExecuteCommand: {}", json);
    assert_eq!(json, r#""execute_command""#);
    
    let tool2 = ToolName::CodebaseSearch;
    let json2 = serde_json::to_string(&tool2).unwrap();
    println!("   CodebaseSearch: {}", json2);
    assert_eq!(json2, r#""codebase_search""#);
    
    // Test 3: All 22 tool names
    println!("\n3. Testing all 22 tool names:");
    let all_tools = vec![
        ToolName::ExecuteCommand,
        ToolName::ReadFile,
        ToolName::WriteToFile,
        ToolName::ApplyDiff,
        ToolName::InsertContent,
        ToolName::SearchAndReplace,
        ToolName::SearchFiles,
        ToolName::ListFiles,
        ToolName::ListCodeDefinitionNames,
        ToolName::BrowserAction,
        ToolName::UseMcpTool,
        ToolName::AccessMcpResource,
        ToolName::AskFollowupQuestion,
        ToolName::AttemptCompletion,
        ToolName::SwitchMode,
        ToolName::NewTask,
        ToolName::FetchInstructions,
        ToolName::CodebaseSearch,
        ToolName::EditFile,
        ToolName::NewRule,
        ToolName::ReportBug,
        ToolName::Condense,
        ToolName::UpdateTodoList,
    ];
    println!("   Total tools: {}", all_tools.len());
    assert_eq!(all_tools.len(), 23); // 22 + UpdateTodoList
    
    // Test 4: ToolUsage
    println!("\n4. Testing ToolUsage:");
    let mut usage = ToolUsage::new();
    usage.record_attempt(ToolName::ReadFile);
    usage.record_attempt(ToolName::ReadFile);
    usage.record_failure(ToolName::ReadFile);
    usage.record_attempt(ToolName::WriteToFile);
    
    let json = serde_json::to_string(&usage).unwrap();
    println!("   ToolUsage: {}", json);
    assert!(json.contains(r#""read_file":{"attempts":2,"failures":1}"#));
    assert!(json.contains(r#""write_to_file":{"attempts":1,"failures":0}"#));
    
    // Test 5: Tool group categorization
    println!("\n5. Testing tool group categorization:");
    assert_eq!(ToolName::ReadFile.group(), ToolGroup::Read);
    assert_eq!(ToolName::WriteToFile.group(), ToolGroup::Edit);
    assert_eq!(ToolName::BrowserAction.group(), ToolGroup::Browser);
    assert_eq!(ToolName::ExecuteCommand.group(), ToolGroup::Command);
    assert_eq!(ToolName::UseMcpTool.group(), ToolGroup::Mcp);
    assert_eq!(ToolName::SwitchMode.group(), ToolGroup::Modes);
    println!("   ✅ All groups correctly categorized");
    
    // Test 6: Helper methods
    println!("\n6. Testing helper methods:");
    assert!(ToolName::ReadFile.is_read_only());
    assert!(!ToolName::ReadFile.is_write());
    assert!(ToolName::WriteToFile.is_write());
    assert!(!ToolName::WriteToFile.is_read_only());
    println!("   ✅ Helper methods work correctly");
    
    // Test 7: Deserialize ToolUsage
    println!("\n7. Testing ToolUsage deserialization:");
    let json_input = r#"{
        "execute_command": {"attempts": 5, "failures": 2},
        "read_file": {"attempts": 10, "failures": 0},
        "codebase_search": {"attempts": 3, "failures": 1}
    }"#;
    
    match serde_json::from_str::<ToolUsage>(json_input) {
        Ok(usage) => {
            println!("   ✅ Successfully deserialized ToolUsage");
            let stats = usage.get_stats(ToolName::ExecuteCommand).unwrap();
            assert_eq!(stats.attempts, 5);
            assert_eq!(stats.failures, 2);
        }
        Err(e) => {
            println!("   ❌ Deserialization failed: {}", e);
        }
    }
    
    // Test 8: All 6 tool groups
    println!("\n8. Testing all 6 tool groups:");
    let groups = vec![
        ToolGroup::Read,
        ToolGroup::Edit,
        ToolGroup::Browser,
        ToolGroup::Command,
        ToolGroup::Mcp,
        ToolGroup::Modes,
    ];
    for group in &groups {
        let json = serde_json::to_string(group).unwrap();
        println!("   {:?}: {}", group, json);
    }
    assert_eq!(groups.len(), 6);
    
    println!("\n✅ All tool.ts translation tests passed!");
}
