/// Exact 1:1 Translation of TypeScript registerTerminalActions from codex-reference/activate/registerTerminalActions.ts
/// DAY 10 H1-2: Translate registerTerminalActions.ts

use std::sync::Arc;

/// Terminal action IDs - exact translation line 3
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalActionId {
    TerminalAddToContext,
    TerminalFixCommand,
    TerminalExplainCommand,
}

/// Terminal action prompt types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalActionPromptType {
    TerminalAddToContext,
    TerminalFix,
    TerminalExplain,
}

impl TerminalActionPromptType {
    fn as_str(&self) -> &str {
        match self {
            TerminalActionPromptType::TerminalAddToContext => "TERMINAL_ADD_TO_CONTEXT",
            TerminalActionPromptType::TerminalFix => "TERMINAL_FIX",
            TerminalActionPromptType::TerminalExplain => "TERMINAL_EXPLAIN",
        }
    }
}

/// Extension context placeholder
pub struct ExtensionContext {
    pub subscriptions: Vec<CommandSubscription>,
}

pub struct CommandSubscription {
    pub command: String,
    pub handler: Arc<dyn Fn(TerminalArgs) + Send + Sync>,
}

#[derive(Debug, Clone)]
pub struct TerminalArgs {
    pub selection: Option<String>,
}

/// registerTerminalActions - exact translation lines 10-14
pub fn register_terminal_actions(context: &mut ExtensionContext) {
    register_terminal_action(
        context,
        TerminalActionId::TerminalAddToContext,
        TerminalActionPromptType::TerminalAddToContext,
    );
    register_terminal_action(
        context,
        TerminalActionId::TerminalFixCommand,
        TerminalActionPromptType::TerminalFix,
    );
    register_terminal_action(
        context,
        TerminalActionId::TerminalExplainCommand,
        TerminalActionPromptType::TerminalExplain,
    );
}

/// registerTerminalAction - exact translation lines 16-39
fn register_terminal_action(
    context: &mut ExtensionContext,
    command: TerminalActionId,
    prompt_type: TerminalActionPromptType,
) {
    let command_for_handler = command.clone();
    let handler = Arc::new(move |args: TerminalArgs| {
        let command_clone = command_for_handler.clone();
        let prompt_type_clone = prompt_type.clone();
        
        tokio::spawn(async move {
            let mut content = args.selection;
            
            // If no content provided, get from terminal - lines 25-27
            if content.is_none() || content.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                let lines_to_get = if prompt_type_clone == TerminalActionPromptType::TerminalAddToContext {
                    -1
                } else {
                    1
                };
                content = Terminal::get_terminal_contents(lines_to_get).await;
            }
            
            // If still no content, show warning - lines 29-32
            if content.is_none() {
                show_warning_message("No terminal content to process"); // t("common:warnings.no_terminal_content")
                return;
            }
            
            // Handle terminal action - lines 34-36
            ClineProvider::handle_terminal_action(
                &get_terminal_command(&command_clone),
                prompt_type_clone.as_str(),
                TerminalActionParams {
                    terminal_content: content.unwrap(),
                },
            ).await;
        });
    });
    
    context.subscriptions.push(CommandSubscription {
        command: get_terminal_command(&command),
        handler,
    });
}

/// Get terminal command string
fn get_terminal_command(command: &TerminalActionId) -> String {
    match command {
        TerminalActionId::TerminalAddToContext => "kilocode.terminalAddToContext".to_string(),
        TerminalActionId::TerminalFixCommand => "kilocode.terminalFixCommand".to_string(),
        TerminalActionId::TerminalExplainCommand => "kilocode.terminalExplainCommand".to_string(),
    }
}

/// Terminal placeholder
pub struct Terminal;

impl Terminal {
    pub async fn get_terminal_contents(lines: i32) -> Option<String> {
        // Placeholder implementation
        // -1 means all content, positive number means last N lines
        if lines == -1 {
            Some("Full terminal content placeholder".to_string())
        } else {
            Some(format!("Last {} line(s) of terminal", lines))
        }
    }
}

/// Terminal action params
#[derive(Debug, Clone)]
pub struct TerminalActionParams {
    pub terminal_content: String,
}

/// ClineProvider placeholder
pub struct ClineProvider;

impl ClineProvider {
    pub async fn handle_terminal_action(
        command: &str,
        prompt_type: &str,
        params: TerminalActionParams,
    ) {
        println!("Handling terminal action: {} ({})", command, prompt_type);
        println!("Terminal content: {}", params.terminal_content);
    }
}

/// Show warning message (placeholder for VSCode API)
fn show_warning_message(message: &str) {
    eprintln!("Warning: {}", message);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_register_terminal_actions() {
        let mut context = ExtensionContext {
            subscriptions: vec![],
        };
        
        register_terminal_actions(&mut context);
        
        assert_eq!(context.subscriptions.len(), 3);
        assert_eq!(context.subscriptions[0].command, "kilocode.terminalAddToContext");
        assert_eq!(context.subscriptions[1].command, "kilocode.terminalFixCommand");
        assert_eq!(context.subscriptions[2].command, "kilocode.terminalExplainCommand");
    }
    
    #[test]
    fn test_terminal_action_prompt_types() {
        assert_eq!(TerminalActionPromptType::TerminalAddToContext.as_str(), "TERMINAL_ADD_TO_CONTEXT");
        assert_eq!(TerminalActionPromptType::TerminalFix.as_str(), "TERMINAL_FIX");
        assert_eq!(TerminalActionPromptType::TerminalExplain.as_str(), "TERMINAL_EXPLAIN");
    }
    
    #[tokio::test]
    async fn test_terminal_get_contents() {
        let content = Terminal::get_terminal_contents(-1).await;
        assert!(content.is_some());
        assert!(content.unwrap().contains("terminal"));
        
        let content = Terminal::get_terminal_contents(1).await;
        assert!(content.is_some());
        assert!(content.unwrap().contains("1 line"));
    }
}
