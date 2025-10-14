# CSS to Floem Style Mappings

Complete reference for translating Windsurf CSS/Tailwind to Floem styles.

---

## Typography

| Windsurf CSS | Tailwind Class | Floem Equivalent |
|--------------|----------------|------------------|
| `font-family: system-ui, Ubuntu, ...` | - | `.font_family("system-ui, Ubuntu, ...")` |
| `font-family: monospace` | `font-mono` | `.font_family("Droid Sans Mono, monospace")` |
| `font-size: 12px` | `text-xs` | `.font_size(12.0)` |
| `font-size: 13px` | `text-sm` | `.font_size(13.0)` |
| `font-size: 14px` | `text-base` | `.font_size(14.0)` |
| `font-weight: 500` | `font-medium` | `.font_weight(Weight::MEDIUM)` |
| `font-weight: 600` | `font-semibold` | `.font_weight(Weight::SEMIBOLD)` |

---

## Colors & Opacity

| Windsurf CSS | Value | Floem Code |
|--------------|-------|------------|
| `--vscode-foreground` | `#cccccc` | `Color::rgb8(0xcc, 0xcc, 0xcc)` |
| `--vscode-editor-background` | `#1f1f1f` | `Color::rgb8(0x1f, 0x1f, 0x1f)` |
| `--vscode-descriptionForeground` | `#9d9d9d` | `Color::rgb8(0x9d, 0x9d, 0x9d)` |
| `bg-neutral-500/20` | `rgba(115,115,115,0.2)` | `Color::rgba8(0x73, 0x73, 0x73, 51)` |
| `.codeium-text-medium` (55% opacity) | `color-mix(55%, transparent)` | `.with_alpha_factor(0.55)` |
| `.codeium-text-light` (35% opacity) | `color-mix(35%, transparent)` | `.with_alpha_factor(0.35)` |
| `opacity-70` | 70% | `.with_alpha_factor(0.7)` |

---

## Spacing

| Tailwind | Pixels | Floem Constant |
|----------|--------|----------------|
| `px-1` / `py-1` | 4px | `spacing::SPACE_1` (4.0) |
| `py-0.5` | 2px | `spacing::SPACE_05` (2.0) |
| `px-2` / `py-2` | 8px | `spacing::SPACE_2` (8.0) |
| `px-3` / `py-3` | 12px | `spacing::SPACE_3` (12.0) |
| `px-4` / `py-4` | 16px | `spacing::SPACE_4` (16.0) |
| `gap-1.5` | 6px | `spacing::SPACE_15` (6.0) |

**Floem Usage**:
```rust
.padding_horiz(spacing::SPACE_1)    // px-1
.padding_vert(spacing::SPACE_05)     // py-0.5
.padding(spacing::SPACE_3)           // p-3
.gap(spacing::SPACE_15)              // gap-1.5
```

---

## Border Radius

| Tailwind | Pixels | Floem Constant |
|----------|--------|----------------|
| `rounded` | 3px | `spacing::ROUNDED` (3.0) |
| `rounded-lg` | 8px | `spacing::ROUNDED_LG` (8.0) |
| `rounded-xl` | 12px | `spacing::ROUNDED_XL` (12.0) |
| `rounded-[15px]` | 15px | `spacing::ROUNDED_PANEL` (15.0) |
| `rounded-full` | 50% | `100.0` (large value) |

**Floem Usage**:
```rust
.border_radius(spacing::ROUNDED)       // rounded
.border_radius(spacing::ROUNDED_LG)    // rounded-lg
.border_radius(100.0)                  // rounded-full
```

---

## Layout

| Tailwind | Floem Equivalent |
|----------|------------------|
| `flex flex-col` | `v_stack(...)` |
| `flex flex-row` | `h_stack(...)` |
| `gap-2` | `.style(\|s\| s.gap(8.0))` |
| `items-center` | `.style(\|s\| s.items_center())` |
| `justify-center` | `.style(\|s\| s.justify_center())` |
| `w-full` | `.style(\|s\| s.width(PxPctAuto::Pct(100.0)))` |
| `h-full` | `.style(\|s\| s.height(PxPctAuto::Pct(100.0)))` |
| `flex-grow` | `.style(\|s\| s.flex_grow(1.0))` |

---

## Specific Widget Mappings

### Inline Code (`<code>`)

**Windsurf HTML**:
```html
<code class="bg-neutral-500/20 px-1 py-0.5 font-mono text-xs font-medium rounded">
  cargo build
</code>
```

**Floem**:
```rust
label("cargo build")
    .style(|s| s
        .font_family("monospace")
        .font_size(12.0)
        .background(Color::rgba8(0x73, 0x73, 0x73, 51))
        .padding_horiz(4.0)
        .padding_vert(2.0)
        .border_radius(3.0)
        .font_weight(Weight::MEDIUM)
    )
```

### Code Block

**Windsurf CSS**:
```css
background: #2b2b2b;
padding: 12px;
border-radius: 8px;
font-family: monospace;
```

**Floem**:
```rust
container(
    label(code)
        .style(|s| s
            .font_family("monospace")
            .white_space(WhiteSpace::Pre)
        )
)
.style(|s| s
    .background(Color::rgb8(0x2b, 0x2b, 0x2b))
    .padding(12.0)
    .border_radius(8.0)
)
```

### AI Message Text (Dim)

**Windsurf CSS**:
```css
.codeium-text-medium {
  color: color-mix(in srgb, var(--codeium-text-color) 55%, #0000);
}
```

**Floem**:
```rust
label(ai_text)
    .style(|s| s
        .color(theme.foreground.with_alpha_factor(0.55))
    )
```

### Panel Container

**Windsurf CSS**:
```css
.panel-bg {
  background: color-mix(in srgb, var(--panel-bg-base) 96%, white);
  border: 1px solid color-mix(...);
  border-radius: 15px;
  box-shadow: 0 4px 12px 0 rgba(0, 0, 0, 0.15);
}
```

**Floem**:
```rust
container(content)
    .style(|s| s
        .background(theme.panel_bg)
        .border(1.0)
        .border_color(theme.panel_border)
        .border_radius(15.0)
        .box_shadow_blur(12.0)
        .box_shadow_color(Color::rgba8(0, 0, 0, 38))
    )
```

### Hover States

**Windsurf CSS**:
```css
.hover\:bg-neutral-500\/10:hover {
  background: rgba(115, 115, 115, 0.1);
}
```

**Floem**:
```rust
button("Click")
    .style(|s| s
        .background(Color::TRANSPARENT)
        .hover(|s| s.background(Color::rgba8(0x73, 0x73, 0x73, 26)))
    )
```

### Send Button (Circular)

**Windsurf HTML**:
```html
<button class="h-[20px] w-[20px] rounded-full bg-ide-input-color">
  <svg>↑</svg>
</button>
```

**Floem**:
```rust
button("↑")
    .style(|s| s
        .width(20.0)
        .height(20.0)
        .border_radius(100.0)  // Full circle
        .background(theme.foreground)
        .color(theme.background)
        .justify_center()
        .items_center()
    )
```

---

## Advanced Features

### Color Mix (CSS `color-mix()`)

**CSS**:
```css
color: color-mix(in srgb, #cccccc 55%, transparent);
```

**Floem Helper**:
```rust
fn color_mix(a: Color, b: Color, ratio: f32) -> Color {
    let r = (a.r as f32 * ratio + b.r as f32 * (1.0 - ratio)) as u8;
    let g = (a.g as f32 * ratio + b.g as f32 * (1.0 - ratio)) as u8;
    let b_val = (a.b as f32 * ratio + b.b as f32 * (1.0 - ratio)) as u8;
    let a_val = (a.a as f32 * ratio + b.a as f32 * (1.0 - ratio)) as u8;
    Color::rgba8(r, g, b_val, a_val)
}
```

### Box Shadow

**CSS**:
```css
box-shadow: 0 4px 12px 0 rgba(0, 0, 0, 0.15);
```

**Floem**:
```rust
.box_shadow_blur(12.0)
.box_shadow_color(Color::rgba8(0, 0, 0, 38))  // 38 = 0.15 * 255
```

### Scrolling

**CSS**:
```css
overflow-y: auto;
max-height: 300px;
```

**Floem**:
```rust
scroll(content)
    .style(|s| s.max_height(300.0))
```

---

## Complete Example Translation

### Windsurf Chat Message

**HTML + CSS**:
```html
<div class="flex flex-col gap-2 p-3 w-full">
  <p class="text-sm" style="color: color-mix(in srgb, #cccccc 55%, transparent);">
    Use <code class="bg-neutral-500/20 px-1 py-0.5 font-mono text-xs rounded">cargo build</code> to compile.
  </p>
</div>
```

**Floem**:
```rust
v_stack((
    h_stack((
        label("Use ")
            .style(|s| s
                .color(theme.foreground.with_alpha_factor(0.55))
                .font_size(13.0)
            ),
        label("cargo build")
            .style(|s| s
                .font_family("monospace")
                .font_size(12.0)
                .background(Color::rgba8(0x73, 0x73, 0x73, 51))
                .padding_horiz(4.0)
                .padding_vert(2.0)
                .border_radius(3.0)
            ),
        label(" to compile.")
            .style(|s| s
                .color(theme.foreground.with_alpha_factor(0.55))
                .font_size(13.0)
            ),
    )),
))
.style(|s| s
    .flex_col()
    .gap(8.0)
    .padding(12.0)
    .width(PxPctAuto::Pct(100.0))
)
```

---

## Quick Reference Cheat Sheet

```rust
// Colors
.background(Color::rgb8(0x1f, 0x1f, 0x1f))   // Hex
.background(Color::rgba8(115, 115, 115, 51)) // RGBA
.color(theme.foreground.with_alpha_factor(0.55)) // Opacity

// Spacing
.padding(12.0)                  // All sides
.padding_horiz(4.0)             // Left + right
.padding_vert(2.0)              // Top + bottom
.gap(8.0)                       // Between children

// Typography
.font_family("monospace")
.font_size(13.0)
.font_weight(Weight::MEDIUM)

// Borders
.border(1.0)
.border_color(theme.panel_border)
.border_radius(15.0)

// Layout
.width(PxPctAuto::Pct(100.0))   // 100%
.height(200.0)                   // 200px
.flex_grow(1.0)
.items_center()
.justify_center()

// Effects
.box_shadow_blur(12.0)
.box_shadow_color(Color::rgba8(0, 0, 0, 38))

// Interactive
.cursor(CursorStyle::Pointer)
.hover(|s| s.background(theme.hover_background))
```

---

## Notes

- **Alpha calculation**: Tailwind `/20` = `20% opacity` = `51/255` in u8
- **Font families**: Match system fonts for consistency
- **Spacing**: Use constants for maintainability
- **Colors**: Extract to theme for dark/light mode support
