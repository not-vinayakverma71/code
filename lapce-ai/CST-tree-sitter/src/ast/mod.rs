//! AST utilities for semantic analysis

pub mod kinds;
pub mod delta;

pub use kinds::{CanonicalKind, LanguageMapping, map_kind, map_field};
