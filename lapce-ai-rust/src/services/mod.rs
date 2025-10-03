// Direct translation of Codex services

// pub mod code_index_manager; // Has compilation issues with LanceDB API
pub mod code_index_manager_simple; // Simplified version that works

// Re-export the simple version as the main one
pub use code_index_manager_simple as code_index_manager;
