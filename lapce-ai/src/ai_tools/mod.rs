// AI-specific tools that use native modules for better performance
// These tools leverage native filesystem, git, and terminal operations

// pub mod semantic_search;    // Semantic search using embeddings (from codebase_search.rs)
// pub mod code_analysis;      // Code analysis with tree-sitter (from list_code_definitions.rs)
// pub mod task_management;    // Task orchestration (from update_todo_list.rs)
// pub mod new_task;          // New task creation
// pub mod completion;        // AI completion logic (from attempt_completion.rs)
// pub mod conversation;      // Conversation management (from condense.rs)
// pub mod interaction;       // User interaction (from ask_followup_question.rs)
pub mod rules;            // Rule management (from new_rule.rs)
pub mod mode;             // Mode switching (from switch_mode.rs)
pub mod feedback;         // Bug reporting (from report_bug.rs)
pub mod instructions;     // Instruction fetching (from fetch_instructions.rs)

// Re-export commonly used items (after fixing modules)
