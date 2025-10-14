# Windsurf Chat UI Analysis for Floem Conversion

## Overview
This document analyzes the Windsurf chat UI from `windsurf.css` and `small.html` to identify:
1. **Static styles** - hardcoded colors/values to convert to Floem
2. **Dynamic styles** - VS Code theme-dependent variables to SKIP
3. **Layout structure** - component hierarchy and spacing

## Key Findings

### 1. CSS Variables Analysis

#### STATIC Chat Colors (Convert to Floem)
These are hardcoded color values not dependent on VS Code theme:

```css
/* Chat Container */
--vscode-inlineChat-background: #202020;
--vscode-inlineChat-border: #454545;
--vscode-inlineChat-shadow: rgba(0, 0, 0, 0.36);

/* Chat Input */
--vscode-inlineChatInput-background: #313131;
--vscode-inlineChatInput-border: #454545;
--vscode-inlineChatInput-focusBorder: #0078d4;  /* Blue accent */
--vscode-inlineChatInput-placeholderForeground: #989898;

/* Message Blocks */
--vscode-chat-requestBackground: rgba(31, 31, 31, 0.62);
--vscode-chat-requestBorder: rgba(255, 255, 255, 0.1);
--vscode-chat-slashCommandBackground: #34414b;
--vscode-chat-slashCommandForeground: #40a6ff;  /* Blue for commands */
--vscode-chat-avatarBackground: #1f1f1f;

/* General Input Styles */
--vscode-input-background: #313131;
--vscode-input-foreground: #cccccc;
--vscode-input-border: #3c3c3c;
--vscode-input-placeholderForeground: #989898;

/* Button Styles */
--vscode-button-background: #0078d4;  /* Primary blue */
--vscode-button-hoverBackground: #026ec1;  /* Darker blue on hover */
--vscode-button-foreground: #ffffff;
--vscode-button-secondaryBackground: #313131;
--vscode-button-secondaryHoverBackground: #3c3c3c;
```

#### DYNAMIC Variables (SKIP - VS Code theme-dependent)
These reference VS Code theme variables:

```css
/* These use var(--vscode-*) and adapt to theme */
--codeium-chat-background: var(--vscode-sideBar-background);
--codeium-message-block-user-background: var(--vscode-list-activeSelectionBackground);
--codeium-message-block-user-color: var(--vscode-list-activeSelectionForeground);
--codeium-message-block-bot-background: var(--vscode-list-inactiveSelectionBackground);
--codeium-message-block-bot-color: var(--vscode-foreground);
--codeium-input-color: var(--vscode-input-foreground);
```

### 2. Floem Color Palette

Based on the static values, here's the Floem palette:

```rust
// Dark theme colors for AI Chat (static, not theme-dependent)
pub struct AiChatTheme {
    // Container
    pub chat_background: Color,           // #202020 (dark)
    pub chat_border: Color,               // #454545 (medium gray)
    pub chat_shadow: Color,               // rgba(0,0,0,0.36)
    
    // Input box
    pub input_background: Color,          // #313131 (dark gray)
    pub input_border: Color,              // #3c3c3c
    pub input_focus_border: Color,        // #0078d4 (blue accent)
    pub input_placeholder: Color,         // #989898 (light gray)
    pub input_foreground: Color,          // #cccccc (off-white)
    
    // Messages
    pub message_user_background: Color,   // rgba(31,31,31,0.62) semi-transparent
    pub message_bot_background: Color,    // #1f1f1f (dark)
    pub message_border: Color,            // rgba(255,255,255,0.1) subtle white
    
    // Special elements
    pub command_background: Color,        // #34414b (blue-gray)
    pub command_foreground: Color,        // #40a6ff (bright blue)
    pub avatar_background: Color,         // #1f1f1f
    
    // Buttons
    pub button_primary: Color,            // #0078d4 (blue)
    pub button_primary_hover: Color,      // #026ec1 (darker blue)
    pub button_secondary: Color,          // #313131
    pub button_secondary_hover: Color,    // #3c3c3c
    pub button_foreground: Color,         // #ffffff
}
```

### 3. Spacing & Layout Constants

From CSS analysis:

```rust
pub struct AiChatSpacing {
    pub border_radius: f64,          // 6.0 (rounded corners)
    pub input_padding_x: f64,        // 12.0
    pub input_padding_y: f64,        // 8.0
    pub message_padding: f64,        // 12.0
    pub panel_padding: f64,          // 16.0
    pub gap_small: f64,              // 4.0
    pub gap_medium: f64,             // 8.0
    pub gap_large: f64,              // 16.0
    pub input_height: f64,           // 36.0 (minimum)
    pub button_height: f64,          // 28.0
    pub avatar_size: f64,            // 24.0
}
```

### 4. Component Structure

Based on HTML patterns (from small.html):

```
┌─ Chat Panel Container ────────────────────────────┐
│  background: #202020                              │
│  border: #454545                                  │
│  shadow: rgba(0,0,0,0.36)                        │
│                                                   │
│  ┌─ Header ─────────────────────────────────┐   │
│  │  Title + Model Selector                  │   │
│  │  height: 40px, padding: 12px             │   │
│  └──────────────────────────────────────────┘   │
│                                                   │
│  ┌─ Message List (Scrollable) ─────────────┐   │
│  │                                          │   │
│  │  ┌─ User Message ────────────────┐      │   │
│  │  │  background: rgba(31,31,31,.62)      │   │
│  │  │  border: rgba(255,255,255,0.1)       │   │
│  │  │  border-radius: 6px                  │   │
│  │  │  padding: 12px                       │   │
│  │  └──────────────────────────────┘      │   │
│  │                                          │   │
│  │  ┌─ AI Message ──────────────────┐      │   │
│  │  │  background: #1f1f1f           │      │   │
│  │  │  border: rgba(255,255,255,0.1) │      │   │
│  │  │  Inline code: #313131          │      │   │
│  │  │  Code block: #202020           │      │   │
│  │  └──────────────────────────────┘      │   │
│  │                                          │   │
│  └──────────────────────────────────────────┘   │
│                                                   │
│  ┌─ Input Area ──────────────────────────────┐  │
│  │  ┌─ Text Input ────────────────────────┐ │  │
│  │  │  background: #313131              │ │  │
│  │  │  border: #3c3c3c                  │ │  │
│  │  │  focus-border: #0078d4 (blue)     │ │  │
│  │  │  placeholder: #989898             │ │  │
│  │  │  foreground: #cccccc              │ │  │
│  │  │  padding: 8px 12px                │ │  │
│  │  │  border-radius: 6px               │ │  │
│  │  └──────────────────────────────────┘ │  │
│  │  ┌─ Send Button ────────────────────┐ │  │
│  │  │  background: #0078d4 (blue)      │ │  │
│  │  │  hover: #026ec1                  │ │  │
│  │  │  foreground: #ffffff             │ │  │
│  │  │  height: 28px                    │ │  │
│  │  │  border-radius: 4px              │ │  │
│  │  └──────────────────────────────────┘ │  │
│  └──────────────────────────────────────────┘  │
└───────────────────────────────────────────────────┘
```

### 5. Typography

```rust
pub struct AiChatTypography {
    pub body_font_size: f64,        // 13.0
    pub code_font_size: f64,        // 12.0
    pub small_font_size: f64,       // 11.0
    pub body_font_family: &'static str,  // "Segoe UI", system-ui
    pub code_font_family: &'static str,  // "Consolas", "Courier New", monospace
    pub line_height: f64,           // 1.5
}
```

## Next Steps for Floem Implementation

### 1. Update `ai_theme.rs`
- Add all static color constants
- Create `AiChatTheme`, `AiChatSpacing`, `AiChatTypography` structs
- Keep existing dark/light theme methods

### 2. Update `ai_chat_widgets.rs`
- Apply exact color values from theme
- Use correct spacing constants
- Match border-radius, padding, shadows

### 3. Input Component Enhancements
- Multi-line text input with auto-resize
- Focus border color change (#3c3c3c → #0078d4)
- Placeholder styling (#989898)
- Send button with hover state

### 4. Message Styling
- User messages: semi-transparent background
- AI messages: darker background with dimmed text
- Code blocks with proper background/padding
- Inline code with distinct background

### 5. Avoid These VS Code Variables
❌ `var(--vscode-sideBar-background)`
❌ `var(--vscode-list-activeSelectionBackground)`
❌ `var(--vscode-foreground)`
❌ Any `var(--vscode-*)` reference

✅ Use hardcoded hex/rgba values only

## Summary

**Static colors identified:** 20+  
**Layout constants:** 12+  
**Components to implement:** 6 (container, header, message-list, user-msg, ai-msg, input-area)

The key insight is that Windsurf uses **hardcoded colors** for the chat UI core, while only the **surrounding IDE elements** reference VS Code theme variables. This means we can safely convert all chat-specific colors to Floem without worrying about theme adaptation.
