# Windsurf Cascade UI Components - Implementation Guide

Based on extracted ui.json data from Windsurf DevTools.

## 1. Text Content (`prose` styling)

```
Classes: prose prose-sm leading-[1.6]
Font-size: 14px
Line-height: 22.4px (1.6)
Color: rgb(204, 204, 204)
```

## 2. Code Blocks

```html
<pre class="mr-1 whitespace-pre-wrap break-all font-mono text-xs">
  <span class="opacity-50">~/lapce$ </span>
  cargo build...
</pre>
```

Styles:
- Font: monospace
- Size: 12px (text-xs)
- Whitespace: pre-wrap
- Word-break: break-all

## 3. Terminal Command Decoration

```html
<div class="terminal-command-decoration codicon">
  <!-- Icon decoration -->
</div>
```

Styles:
- Width: 8px
- Height: 17.37px
- Font-size: 16px
- Margin-left: -17px
- Position: absolute

## 4. Message Components

### User Message
- TODO: Need to extract from ui.json

### AI Message  
- TODO: Need to extract from ui.json

## 5. Tool Execution Display
- TODO: Need HTML structure

## Next Steps:
1. Build markdown/prose renderer
2. Build code block component with syntax highlighting
3. Build terminal output component
4. Build tool execution UI
5. Build thinking/streaming indicators
