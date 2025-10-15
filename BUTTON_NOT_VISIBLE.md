# Code Mode Button Not Visible

## Issue
User reports "nothing appearing" after clicking code button - button may not be rendering.

## What I Did
1. Created `code_mode_selector()` function with simple toggle button
2. Added it to `header_bar()` in h_stack next to title
3. Made button BRIGHT GREEN (0x00, 0xff, 0x00) border to test visibility
4. Added dark gray background (0x40, 0x40, 0x40)

## Current State
```rust
// In header_bar():
h_stack((
    label(|| "Windsurf Chat - Real Conversation".to_string())
        .style(|s| { s.flex_grow(1.0) }),
    code_mode_selector(),  // <-- Should appear on right
))
```

## Button Styling
- Font: 12px, color #cccccc
- Padding: 8px vertical, 16px horizontal
- Border: 2px **BRIGHT GREEN** (#00ff00)
- Background: #404040
- Border radius: 6px
- Cursor: pointer
- Shows "Code" text initially

## Next Steps if Still Not Visible
1. Check if header_bar itself renders (title should show)
2. Try moving button to main chat area instead of header
3. Test with even simpler element (just a colored box)
4. Check floem h_stack layout behavior with flex_grow

## Demo runs successfully
- No compilation errors
- Binary starts
- But button visibility unclear
