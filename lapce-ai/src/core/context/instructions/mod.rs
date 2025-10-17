//! Instructions Management
//!
//! Port from Codex/src/core/context/instructions/
//! Handles kilo-rules, workflows, and rule-helpers.

pub mod kilo_rules;
pub mod rule_helpers;
pub mod workflows;

pub use kilo_rules::*;
pub use rule_helpers::*;
pub use workflows::*;
