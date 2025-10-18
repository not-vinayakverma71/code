# Windsurf → Floem UI/UX Conversion Analysis

## Files Analyzed
- **windsurf.css**: 2,657 lines - Complete VSCode/Windsurf CSS variables & Tailwind classes
- **small.html**: 8,990 lines (579KB) - Full AI Chat UI HTML structure

## Key Findings

### 1. Color System (CSS Variables → Floem Colors)

#### Core Colors
```css
--vscode-editor-background: #1f1f1f          → editor_background
--vscode-editor-foreground: #cccccc          → foreground
--vscode-textLink-foreground: #4daafc        → link_color
--vscode-input-background: #313131           → input_background
--vscode-input-border: #3c3c3c               → input_border
--vscode-widget-border: #313131              → panel_border
```

#### Chat-Specific Colors
```css
--vscode-chat-requestBorder: rgba(255, 255, 255, 0.1)
--vscode-chat-requestBackground: rgba(31, 31, 31, 0.62)
--vscode-chat-slashCommandBackground: #34414b
--vscode-chat-slashCommandForeground: #40a6ff
--vscode-chat-avatarBackground: #1f1f1f
--vscode-chat-avatarForeground: #cccccc
```

#### Code Blocks
```css
--vscode-textCodeBlock-background: #2b2b2b   → code_block_bg
--vscode-textPreformat-background: #3c3c3c
--vscode-textPreformat-foreground: #d0d0d0
```

#### AI Text Dimming
- Used extensively: `opacity-70`, `opacity-50` for AI responses
- codeium-text-medium class for 55% opacity

### 2. Typography System

```css
--vscode-font-family: system-ui, "Ubuntu", "Droid Sans", sans-serif
--vscode-editor-font-family: 'Droid Sans Mono', 'monospace', monospace
--vscode-font-size: 13px
--vscode-editor-font-size: 14px
```

**Tailwind Text Classes Found:**
- `text-[13px]` - Base text (most common)
- `text-xs` - 12px (inline code)
- `text-sm` - 14px
- `leading-[1.5]`, `leading-[1.6]` - Line heights

### 3. Spacing System (Tailwind → Floem)

**Most Common Spacing Patterns:**
```
gap-0.5  → 2px   (0.125rem)
gap-1    → 4px   (0.25rem) 
gap-1.5  → 6px   (0.375rem) ✅ EXACT WINDSURF
gap-2    → 8px   (0.5rem)
gap-3    → 12px  (0.75rem)
gap-4    → 16px  (1rem)

px-1     → 4px horizontal padding
py-0.5   → 2px vertical padding
```

### 4. Border Radius (Tailwind → Floem)

```
rounded           → 4px    (border-radius: 0.25rem)
rounded-[6px]     → 6px    ✅ EXACT (panel/steps)
rounded-[12px]    → 12px   ✅ EXACT (buttons)
rounded-full      → 9999px (circular avatars)
```

### 5. Component Breakdown

#### A. **Inline Code** (Most Used: 19 occurrences)
```html
class="max-w-full whitespace-pre-wrap break-all 
       rounded bg-neutral-500/20 
       px-1 py-0.5 
       font-mono text-xs font-medium 
       text-ide-text-color underline-offset-2 inline"
```
**Floem Translation:**
- Background: `neutral-500/20` = rgba(115, 115, 115, 0.2)
- Padding: 4px horiz, 2px vert
- Font: monospace, 12px, medium weight
- Border radius: 4px

#### B. **Message Container**
```html
class="flex flex-row items-start gap-2"
```
- Horizontal flex layout
- Items align to top
- 8px gap between elements

#### C. **Chat Row Structure**
```html
class="group top-0 flex w-full flex-row items-start gap-2 
       relative after:absolute after:block after:h-[calc(100cqh-1px)] 
       after:w-full after:content-[''] after:pointer-events-none"
```
- Full width row
- Group hover support
- Pseudo-element for decoration

#### D. **Message Bubble Content**
```html
class="codeium-text-medium flex min-w-0 flex-col gap-1"
```
- Vertical flex column
- 4px gap between paragraphs
- Dimmed text (text-medium = 55% opacity)

#### E. **Prose/Markdown Container** (26 occurrences)
```html
class="leading-[1.6] prose prose-sm max-w-none break-words 
       prose-headings:mb-1 prose-headings:text-base prose-headings:font-semibold
       prose-p:mb-3 prose-p:block prose-p:leading-[1.6]
       prose-code:before:content-none prose-code:after:content-none
       prose-pre:my-2 prose-pre:bg-transparent prose-pre:p-0
       prose-ol:my-0 prose-ol:flex prose-ol:list-outside prose-ol:list-decimal
       prose-ul:my-0 prose-ul:flex prose-ul:list-outside prose-ul:list-disc"
```
**Key Requirements:**
- Line height: 1.6
- Headings: 1rem (16px), semibold, 4px bottom margin
- Paragraphs: 12px bottom margin, block display
- Code: no pseudo-elements (` ` markers)
- Lists: outside markers, flex layout, decimal/disc

#### F. **Input Button** (Panel Border Style)
```html
class="panel-border panel-bg text-ide-text-color shadow-menu 
       rounded-[12px] relative px-[9px] py-[6px] 
       cursor-pointer 
       hover:bg-[color-mix(in_srgb,var(--codeium-input-background)_70%,var(--codeium-chat-background)_30%)]"
```
**Floem Requirements:**
- 12px border radius
- 9px horizontal, 6px vertical padding
- Hover: color-mix of input (70%) + chat (30%) backgrounds

#### G. **File Icons** (11 occurrences)
```html
class="show-file-icons"
```
- VSCode file icon integration
- Codicons font family

#### H. **Action Buttons** (Copy, Thumbs, etc.)
```html
class="lucide lucide-copy h-3 w-3"
class="lucide lucide-thumbs-up h-3 w-3 
       transition-[opacity,transform] duration-200 
       cursor-pointer opacity-70 hover:opacity-100"
```
- 12px × 12px icons (h-3 w-3)
- 70% opacity → 100% on hover
- 200ms transition

#### I. **Avatar Circle**
```html
class="relative mt-[2px] flex size-[13px] flex-shrink-0 
       items-center justify-center 
       rounded-full bg-[var(--codeium-text-color)] 
       opacity-50"
```
- 13px circular avatar
- 50% opacity
- Centered content

#### J. **Step Panel** (Shadow-step style)
```html
class="overflow-hidden panel-bg panel-border 
       rounded-[6px] shadow-step"
```
- 6px border radius
- Panel background + border
- Custom shadow

### 6. Layout Patterns

**Most Common Flex Patterns:**
1. `flex flex-row items-start gap-2` (7×) - Message rows
2. `flex min-w-0 flex-col gap-1` (19×) - Content columns
3. `inline-flex items-baseline gap-0.5` (10×) - Inline elements
4. `flex w-full flex-row` (10×) - Full-width rows

**Group Hover:**
```html
class="group/exp relative"
class="group-hover:opacity-50" (child)
```
- Parent-child hover relationships

### 7. Key Measurements

#### Exact Windsurf Values
- **Send button**: 20×20px (from previous analysis)
- **Gap**: 6px (gap-1.5) between input elements
- **Text size**: 12px (text-[12px]) for UI buttons
- **Icon size**: 12px (h-3 w-3) for action icons
- **Avatar size**: 13px circular
- **Panel radius**: 6px
- **Button radius**: 12px
- **Input padding**: 9px × 6px

### 8. Shadows & Effects

```css
shadow-menu    → Box shadow for dropdowns
shadow-step    → Box shadow for step panels
transition-[opacity,transform] duration-200
```

### 9. Cursor & Interaction

```
cursor-pointer
hover:underline
hover:bg-[...]
hover:opacity-100
select-none
```

## Floem Conversion Strategy

### Phase 1: Enhanced Theme System
**File:** `lapce-app/src/ai_theme_v2.rs`

```rust
pub struct WindsurfTheme {
    // Base colors (from CSS vars)
    pub editor_background: Color,      // #1f1f1f
    pub foreground: Color,             // #cccccc
    pub link_color: Color,             // #4daafc
    pub link_hover_color: Color,
    
    // Input colors
    pub input_background: Color,       // #313131
    pub input_border: Color,           // #3c3c3c
    pub input_placeholder: Color,      // #989898
    
    // Chat colors
    pub chat_background: Color,        // rgba(31, 31, 31, 0.62)
    pub chat_border: Color,            // rgba(255, 255, 255, 0.1)
    pub chat_avatar_bg: Color,
    
    // Code colors
    pub code_block_bg: Color,          // #2b2b2b
    pub inline_code_bg: Color,         // rgba(115, 115, 115, 0.2)
    pub code_text: Color,              // #d0d0d0
    
    // UI feedback
    pub hover_background: Color,
    pub panel_border: Color,           // #313131
    pub shadow_color: Color,
    
    // Typography
    pub font_family: &'static str,     // system-ui
    pub mono_family: &'static str,     // Droid Sans Mono
    pub font_size_base: f32,           // 13px
    pub font_size_code: f32,           // 12px
    pub line_height: f32,              // 1.6
    
    // Spacing (exact Tailwind values)
    pub gap_0_5: f32,   // 2px
    pub gap_1: f32,     // 4px
    pub gap_1_5: f32,   // 6px ✅
    pub gap_2: f32,     // 8px
    pub gap_3: f32,     // 12px
    pub gap_4: f32,     // 16px
    
    // Radius
    pub radius_sm: f32,      // 4px
    pub radius_md: f32,      // 6px ✅
    pub radius_lg: f32,      // 12px ✅
    pub radius_full: f32,    // 9999px
    
    // Sizes
    pub icon_size_sm: f32,   // 12px (h-3)
    pub avatar_size: f32,    // 13px
    pub button_size: f32,    // 20px
}
```

### Phase 2: Core Components
**Files:** `lapce-app/src/panel/ai_chat/windsurf_components/`

#### 2.1 `inline_code_v2.rs`
- Exact: `bg-neutral-500/20`, `rounded`, `px-1 py-0.5`
- Font: mono, 12px, medium weight

#### 2.2 `code_block_v2.rs`
- Background: `#2b2b2b`
- Pre: `my-2`, `bg-transparent`, `p-0`

#### 2.3 `message_bubble.rs`
- Container: `flex flex-col gap-1`
- Text: 55% opacity for AI

#### 2.4 `markdown_prose.rs`
- Headings: 16px, semibold, mb-1
- Paragraphs: leading-1.6, mb-3
- Lists: outside markers, flex, gap-1

#### 2.5 `action_buttons.rs`
- Icons: 12×12px
- Opacity: 70% → 100% hover
- Transition: 200ms

#### 2.6 `avatar.rs`
- Circular: 13px
- 50% opacity
- Centered

#### 2.7 `input_button.rs`
- Radius: 12px
- Padding: 9px × 6px
- Hover: color-mix

### Phase 3: Layout System
**File:** `lapce-app/src/panel/ai_chat/windsurf_layout.rs`

- Message rows: `flex-row gap-2`
- Content columns: `flex-col gap-1`
- Group hover support

### Phase 4: Integration
- Replace `ai_chat_widgets.rs` with Windsurf components
- Update `ai_theme.rs` → `ai_theme_v2.rs`
- Wire to existing IPC bridge

## Action Items

1. ✅ **Extract exact CSS variables** (DONE - above)
2. ⏳ **Create WindsurfTheme struct** with all colors/sizes
3. ⏳ **Build component library** (9 components)
4. ⏳ **Implement layout patterns** (flex, group-hover)
5. ⏳ **Test against real HTML** structure
6. ⏳ **Wire to ai_bridge** for real data

## Success Criteria

- [ ] Inline code matches: `bg-neutral-500/20 px-1 py-0.5 rounded text-xs`
- [ ] Message spacing: `gap-1` (4px) between paragraphs
- [ ] Action icons: 12×12px, 70% opacity
- [ ] Input button: 12px radius, 9×6px padding
- [ ] Avatar: 13px circle, 50% opacity
- [ ] Text: leading-1.6 (line height)
- [ ] AI dimming: 55% opacity
- [ ] Hover states: 200ms transitions
- [ ] Panel radius: 6px
- [ ] Button radius: 12px

## References

- windsurf.css: Lines 2400-2657 (chat colors)
- small.html: Contains all UI patterns
- Tailwind classes: Exact pixel values documented above
