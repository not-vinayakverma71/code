# Dropdown Fix - Root Cause Analysis

## Problem
Dropdown toggle works, but clicking items does nothing.

## Root Cause
**floem doesn't register event handlers on `Display::None` elements.**

When `is_open = false`:
- Original code: `.display(Display::None)` 
- Result: Element removed from render tree
- Event handlers: **NEVER REGISTERED**
- Click result: Nothing happens

## Failed Attempts
1. **height(0)**: Handlers registered but no clickable surface area
2. **inset_top(-9999)**: Still has positioning issues  
3. **dyn_container**: Syntax complexity

## Working Solution
**Keep element always rendered, hide with visibility or clip**

```rust
.style(move |s| {
    if is_open.get() {
        s.position(Position::Absolute)
            .inset_bottom_pct(100.0)
            .margin_bottom(8.0)
            .z_index(9999)
    } else {
        s.position(Position::Absolute)
            .inset_bottom_pct(100.0) 
            .margin_bottom(8.0)
            .z_index(-1)  // Behind everything
            .width(0.0)   // No width
            .height(0.0)  // No height
            .overflow_hidden() // Clip content
    }
})
```

This keeps the element in DOM (handlers work) but makes it invisible and non-interactive.
