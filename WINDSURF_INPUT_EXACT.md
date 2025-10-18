# Windsurf Input Box - EXACT Structure 🎨

> **Documentation Version:** 1.1  
> **Last Updated:** 2025-10-12  
> **Source:** outerhtml.html (Real Windsurf IDE extraction)

## 🎯 Overview

This document contains the **exact** Windsurf input structure extracted from the real application, not guessed or approximated. Use this as the source of truth for implementing a pixel-perfect Windsurf-style input component.

### Key Features
- ✅ Tiny 20×20px send button
- ✅ Right-aligned action buttons
- ✅ Contenteditable input area
- ✅ Opacity-based hover states
- ✅ Theme-aware color system

---

## Main Structure

```html
<div class="flex flex-row">
  <div class="flex w-full flex-col items-stretch">
    
    <!-- BOTTOM BAR: Buttons (left side) -->
    <div class="flex items-center justify-start gap-1.5">
      
      <!-- 1. Add Files Button (+ icon) -->
      <div class="relative flex items-center -mr-0.5">
        <input type="file" accept=".png,.jpg,..." class="hidden">
        <button class="relative m-0 -my-px flex cursor-pointer flex-row items-center justify-start gap-0.5 overflow-hidden rounded border-0 p-0.5 pl-1 text-[12px] text-ide-text-color opacity-70 outline-none hover:bg-neutral-500/10 hover:opacity-100 pr-1 !p-0.5">
          <svg><!-- plus icon --></svg>
        </button>
      </div>
      
      <!-- 2. Code Button -->
      <div class="relative">
        <button class="relative m-0 -my-px flex cursor-pointer... text-[12px] leading-[12px]">
          <div class="flex items-center gap-1">
            <svg><!-- code icon --></svg>
            <span>Code</span>
          </div>
        </button>
      </div>
      
      <!-- 3. Model Selector Button -->
      <button class="m-0 -my-px cursor-pointer... text-[12px] leading-[12px]">
        <span>Claude Sonnet 4.5 Thinking (promo)</span>
      </button>
      
      <!-- SPACER: Push right buttons to the right -->
      <div class="flex-grow"></div>
      
      <!-- 4. Microphone Button (right side) -->
      <button data-state="closed" class="content">
        <div class="relative flex shrink-0 cursor-pointer flex-row items-center gap-1 overflow-hidden rounded p-0.5 opacity-70 hover:bg-neutral-500/20 hover:opacity-100">
          <svg><!-- mic icon 3.5x3.5 --></svg>
        </div>
      </button>
      
      <!-- 5. Send Button (FAR RIGHT, TINY) -->
      <button class="group flex h-[20px] w-[20px] flex-shrink-0 items-center justify-center rounded-full bg-ide-input-color text-ide-input-background cursor-not-allowed opacity-50" type="submit">
        <svg class="lucide lucide-arrow-up h-3 w-3">
          <!-- arrow up icon -->
        </svg>
      </button>
      
    </div>
  </div>
</div>
```

---

## Input Area (Above Buttons)

```html
<!-- Contenteditable div (acts as textarea) -->
<div contenteditor="true" style="user-select: text; white-space: pre-wrap; word-break: break-word;">
  <p class=""><br></p>
</div>

<!-- Placeholder (overlay) -->
<p class="pointer-events-none absolute left-0 right-0 top-0 z-0 flex select-none items-center overflow-hidden text-ellipsis whitespace-nowrap text-ide-input-color opacity-50 !text-ide-input-color">
  <span class="truncate">Ask anything (Ctrl+L)</span>
</p>
```

---

## Key CSS Classes

### Send Button (Most Important!)
```
h-[20px]              // Exactly 20px height
w-[20px]              // Exactly 20px width  
flex-shrink-0         // Don't shrink
items-center          // Center content vertically
justify-center        // Center content horizontally
rounded-full          // Perfect circle
bg-ide-input-color    // Use theme color
cursor-not-allowed    // When disabled
opacity-50            // When disabled
```

### Bottom Bar Container
```
flex items-center justify-start gap-1.5
```

### Button Common Pattern
```
relative m-0 -my-px flex cursor-pointer flex-row items-center justify-start gap-0.5 overflow-hidden rounded border-0 p-0.5 pl-1 text-[12px] text-ide-text-color opacity-70 outline-none hover:bg-neutral-500/10 hover:opacity-100
```

### Spacer (Push Right)
```
flex-grow
```

---

## Windsurf Design Principles

1. **Tiny send button**: 20x20px, not 32px or 40px
2. **Far right alignment**: Spacer pushes mic + send to the right
3. **Minimal opacity**: 70% → 100% on hover
4. **Small text**: 12px for buttons
5. **Tight spacing**: gap-1.5 (6px)
6. **Subtle hover**: bg-neutral-500/10
7. **Icon sizing**: 3x3 (12px) or 3.5x3.5 (14px)

---

## Colors from Theme

- `text-ide-text-color` → `--vscode-foreground`
- `text-ide-input-color` → `--vscode-input-foreground`
- `bg-ide-input-color` → `--vscode-input-background`
- `text-ide-input-background` → button text on colored bg

---

## Translation to Floem/Rust

### Complete Implementation Example

```rust
use floem::views::*;
use floem::style::*;

fn windsurf_input_bar() -> impl View {
    h_stack((
        // Left buttons group
        h_stack((
            add_files_button(),
            code_button(), 
            model_selector(),
        ))
        .style(|s| s.gap(6.0)),  // gap-1.5 = 6px
        
        // Spacer (pushes right content to the right)
        empty().style(|s| s.flex_grow(1.0)),
        
        // Right buttons group
        h_stack((
            mic_button(),
            send_button(),
        ))
        .style(|s| s.gap(6.0)),
    ))
    .style(|s| {
        s.width_full()
            .items_center()
            .gap(6.0)
            .padding(4.0)
    })
}

// Send button implementation
fn send_button() -> impl View {
    button(|| {
        svg(ARROW_UP_ICON)
            .style(|s| s.size(12.0, 12.0))  // 3x3 = 12px
    })
    .style(|s| {
        s.size(20.0, 20.0)  // Exact 20x20px
            .border_radius(50.percent())  // Perfect circle
            .flex_shrink(0.0)
            .items_center()
            .justify_center()
            .background(Color::from(theme.input_background))
            .color(Color::from(theme.input_foreground))
            .cursor(CursorStyle::NotAllowed)
            .apply_if(disabled, |s| s.opacity(50.0))
    })
}
```

### Helper Button Pattern

```rust
fn base_button(content: impl View) -> impl View {
    button(|| content)
        .style(|s| {
            s.margin(0.0)
                .flex_row()
                .items_center()
                .gap(2.0)
                .padding_horiz(4.0)
                .padding_vert(2.0)
                .font_size(12.0)
                .border_radius(4.0)
                .opacity(70.0)
                .hover(|s| {
                    s.background(Color::rgba8(128, 128, 128, 25))  // neutral-500/10
                        .opacity(100.0)
                })
        })
}
```

---

## Implementation Checklist

### Phase 1: Core Components ✅
- [x] Build the send button (20x20px circular)
- [x] Build the button row with spacer
- [x] Build the input area (contenteditable-like)
- [x] Add placeholder overlay
- [x] Wire up keyboard shortcuts

### Phase 2: Button Components
- [ ] Add Files button with file picker
- [ ] Code toggle button with state
- [ ] Model selector dropdown
- [ ] Microphone button with recording state

### Phase 3: Polish & UX
- [ ] Hover state animations
- [ ] Focus management
- [ ] Keyboard navigation (Tab, Arrow keys)
- [ ] Accessibility (ARIA labels)
- [ ] Theme color integration

### Phase 4: Testing
- [ ] Unit tests for button states
- [ ] Integration tests for keyboard shortcuts
- [ ] Visual regression tests
- [ ] Theme compatibility tests

---

## Troubleshooting

### Send Button Not Circular
**Problem:** Button appears oval or square.  
**Solution:** Ensure `flex-shrink-0` is set and both width/height are exactly 20px.

### Buttons Not Right-Aligned
**Problem:** Right buttons appear in center or left.  
**Solution:** Add a `flex-grow` spacer between left and right button groups.

### Hover State Not Working
**Problem:** Opacity doesn't change on hover.  
**Solution:** Check base opacity is 70% and hover changes it to 100%.

### Theme Colors Not Applied
**Problem:** Colors don't match VS Code theme.  
**Solution:** Verify CSS variables map correctly:
- `--vscode-input-background`
- `--vscode-input-foreground`
- `--vscode-foreground`

---

## Performance Considerations

1. **Icon Rendering:** Use SVG for scalability, cache rendered icons
2. **Hover States:** Use CSS transitions for smooth opacity changes
3. **Contenteditable:** Debounce input events to avoid excessive updates
4. **Theme Updates:** Subscribe to theme changes efficiently

---

## References

- [Floem Documentation](https://github.com/lapce/floem)
- [TailwindCSS Sizing](https://tailwindcss.com/docs/width)
- [VS Code Theme API](https://code.visualstudio.com/api/references/theme-color)
- Related files:
  - `WINDSURF_FULL_UI_STRUCTURE.md`
  - `WINDSURF_UI_COMPONENTS.md`
  - `VSCODE_INPUT_STYLING.md`

---

## Version History

- **v1.1** (2025-10-12): Added implementation examples, troubleshooting, and testing checklist
- **v1.0** (2025-10-12): Initial extraction from outerhtml.html

---

**Ready to implement!** 🚀

*For questions or improvements, update this document and increment the version number.*
