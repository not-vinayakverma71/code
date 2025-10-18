// UI Primitives - Phase 1.1
// Ported from Codex components/ui/

pub mod badge;
pub mod button;

// COMMENTED OUT: Using Floem's built-in components instead
// These primitive ports had compilation errors and Floem already provides:
// - Dropdown (dropdown::Dropdown)
// - Tooltip (tooltip())
// - Dialog (can build with container + overlay)
// - Checkbox (checkbox())
// - Button (button())
// See FLOEM_FINDINGS.md for details
// pub mod primitives;

// TODO: Port remaining primitives
// pub mod dialog;
// pub mod select;
// pub mod checkbox;
// pub mod tooltip;
// pub mod progress;
// pub mod separator;
