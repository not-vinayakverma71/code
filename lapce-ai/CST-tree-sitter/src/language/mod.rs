//! Language registry and utilities for tree-sitter grammars
//! Provides unified access to all 73 configured languages

pub mod registry;

pub use registry::{LanguageRegistry, LanguageInfo, LanguageError};
