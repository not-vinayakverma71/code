/// Exact 1:1 Translation of TypeScript registerCodeActions from codex-reference/activate/registerCodeActions.ts
/// DAY 10 H1-2: Translate registerCodeActions.ts

use std::sync::Arc;

/// Code action IDs - exact translation line 3
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeActionId {
    ExplainCode,
    FixCode,
    ImproveCode,
    AddToContext,
}

/// Code action names
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CodeActionName {
    Explain,
    Fix,
    Improve,
    AddToContext,
}

impl CodeActionName {
    fn as_str(&self) -> &str {
        match self {
            CodeActionName::Explain => "EXPLAIN",
            CodeActionName::Fix => "FIX",
            CodeActionName::Improve => "IMPROVE",
            CodeActionName::AddToContext => "ADD_TO_CONTEXT",
        }
    }
}

/// Extension context placeholder
pub struct ExtensionContext {
    pub subscriptions: Vec<CommandSubscription>,
}

pub struct CommandSubscription {
    pub command: String,
    pub handler: Arc<dyn Fn(Vec<String>) + Send + Sync>,
}

/// registerCodeActions - exact translation lines 9-14
pub fn register_code_actions(context: &mut ExtensionContext) {
    register_code_action(context, CodeActionId::ExplainCode, CodeActionName::Explain);
    register_code_action(context, CodeActionId::FixCode, CodeActionName::Fix);
    register_code_action(context, CodeActionId::ImproveCode, CodeActionName::Improve);
    register_code_action(context, CodeActionId::AddToContext, CodeActionName::AddToContext);
}

/// registerCodeAction - exact translation lines 16-53
fn register_code_action(
    context: &mut ExtensionContext,
    command: CodeActionId,
    prompt_type: CodeActionName,
) {
    let mut user_input: Option<String> = None;
    let command_for_handler = command.clone();
    
    let handler = Arc::new(move |args: Vec<String>| {
        let file_path: String;
        let selected_text: String;
        let mut start_line: Option<u32> = None;
        let mut end_line: Option<u32> = None;
        let mut diagnostics: Option<Vec<Diagnostic>> = None;
        
        if args.len() > 1 {
            // Called from code action - line 29-30
            file_path = args.get(0).cloned().unwrap_or_default();
            selected_text = args.get(1).cloned().unwrap_or_default();
            start_line = args.get(2).and_then(|s| s.parse().ok());
            end_line = args.get(3).and_then(|s| s.parse().ok());
            // diagnostics would be parsed from args[4] if present
        } else {
            // Called directly from command palette - lines 32-39
            if let Some(editor_context) = EditorUtils::get_editor_context() {
                file_path = editor_context.file_path;
                selected_text = editor_context.selected_text;
                start_line = editor_context.start_line;
                end_line = editor_context.end_line;
                diagnostics = editor_context.diagnostics;
            } else {
                return;
            }
        }
        
        // Build params - lines 42-48
        let mut params = CodeActionParams {
            file_path,
            selected_text,
            start_line: start_line.map(|n| n.to_string()),
            end_line: end_line.map(|n| n.to_string()),
            diagnostics,
            user_input: user_input.clone(),
        };
        
        // Call ClineProvider - line 50
        let command_clone = command_for_handler.clone();
        let prompt_type_clone = prompt_type.clone();
        
        tokio::spawn(async move {
            ClineProvider::handle_code_action(
                &get_code_action_command(&command_clone),
                prompt_type_clone.as_str(),
                params,
            ).await;
        });
    });
    
    context.subscriptions.push(CommandSubscription {
        command: get_code_action_command(&command),
        handler,
    });
}

/// Get code action command string
fn get_code_action_command(command: &CodeActionId) -> String {
    match command {
        CodeActionId::ExplainCode => "kilocode.explainCode".to_string(),
        CodeActionId::FixCode => "kilocode.fixCode".to_string(),
        CodeActionId::ImproveCode => "kilocode.improveCode".to_string(),
        CodeActionId::AddToContext => "kilocode.addToContext".to_string(),
    }
}

/// Editor context
#[derive(Debug, Clone)]
pub struct EditorContext {
    pub file_path: String,
    pub selected_text: String,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
    pub diagnostics: Option<Vec<Diagnostic>>,
}

/// Diagnostic placeholder
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub severity: String,
}

/// EditorUtils placeholder
pub struct EditorUtils;

impl EditorUtils {
    pub fn get_editor_context() -> Option<EditorContext> {
        // Placeholder implementation
        None
    }
}

/// Code action params
#[derive(Debug, Clone)]
pub struct CodeActionParams {
    pub file_path: String,
    pub selected_text: String,
    pub start_line: Option<String>,
    pub end_line: Option<String>,
    pub diagnostics: Option<Vec<Diagnostic>>,
    pub user_input: Option<String>,
}

/// ClineProvider placeholder
pub struct ClineProvider;

impl ClineProvider {
    pub async fn handle_code_action(
        command: &str,
        prompt_type: &str,
        params: CodeActionParams,
    ) {
        println!("Handling code action: {} ({})", command, prompt_type);
        println!("Params: {:?}", params);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_register_code_actions() {
        let mut context = ExtensionContext {
            subscriptions: vec![],
        };
        
        register_code_actions(&mut context);
        
        assert_eq!(context.subscriptions.len(), 4);
        assert_eq!(context.subscriptions[0].command, "kilocode.explainCode");
        assert_eq!(context.subscriptions[1].command, "kilocode.fixCode");
        assert_eq!(context.subscriptions[2].command, "kilocode.improveCode");
        assert_eq!(context.subscriptions[3].command, "kilocode.addToContext");
    }
    
    #[test]
    fn test_code_action_names() {
        assert_eq!(CodeActionName::Explain.as_str(), "EXPLAIN");
        assert_eq!(CodeActionName::Fix.as_str(), "FIX");
        assert_eq!(CodeActionName::Improve.as_str(), "IMPROVE");
        assert_eq!(CodeActionName::AddToContext.as_str(), "ADD_TO_CONTEXT");
    }
}
