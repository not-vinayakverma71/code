// SEMANTIC SEARCH - CORRECT IMPLEMENTATION
// Direct translation from Codex TypeScript using local LanceDB

pub fn init() {
    println!("Lapce AI Rust - Semantic Search initialized");
}

// SEMANTIC SEARCH MODULES - Direct translations from Codex
pub mod tools;     // Translation of Codex/src/core/tools/
pub mod services;  // Translation of Codex/src/services/code-index/

// IPC PROTOCOL MODULES - Complete TypeScript translations
pub mod ipc_messages;  // Core modules
pub mod ai_providers; // Sage protocol
pub mod xml_parsing_utils;
pub mod cross_platform_ipc;
pub mod windows_shared_memory;
pub mod macos_shared_memory;  // Complete ipc.ts translation
pub mod events_exact_translation;  // Complete events.ts translation
pub mod global_settings_exact_translation;  // Complete global-settings.ts translation
pub mod tools_translation;  // Complete tool.ts translation

// Existing modules that we're keeping
pub mod types;
pub mod ai_completion;
pub mod file_watcher;
pub mod multi_language_parser;
pub mod distributed_search;
pub mod lapce_plugin;
pub mod refactoring_engine;
pub mod bug_detection_ai;
pub mod test_generator_complete;
pub mod doc_generator_complete;
pub mod shared_memory_complete;
// pub mod shared_memory_optimized; // TODO: Fix implementation
pub mod shared_memory_nuclear;
pub mod binary_codec;
pub mod ipc_config;
// Provider implementations - NOW IN ai_providers module
// All old provider modules have been reorganized into src/ai_providers/
pub mod working_cache_system;
pub mod optimized_cache;
pub mod connection_pool_complete;
pub mod mock_types;
pub mod optimized_vector_search;
pub mod cache;
