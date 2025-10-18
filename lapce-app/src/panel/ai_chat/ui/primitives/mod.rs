// UI Primitives - Radix UI equivalent components for Floem
// Week 1, Day 1-2: Foundation components
//
// These primitives are used by all other UI components

pub mod dialog;
pub mod dropdown;
pub mod popover;

// Re-exports for convenience
pub use dialog::{dialog, confirm_dialog, DialogProps};
pub use dropdown::{dropdown_menu, dropdown_item, dropdown_checkbox, DropdownItem, DropdownProps};
pub use popover::{popover, simple_popover, PopoverProps, PopoverPosition};
