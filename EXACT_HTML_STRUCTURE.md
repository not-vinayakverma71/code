# EXACT Windsurf HTML Structure from small.html

## File Link (from line 7643-7651)
```html
<span class="inline-flex items-baseline gap-0.5 break-all text-[0.9em] cursor-pointer decoration-current hover:underline font-mono leading-[1rem] text-ide-link-color hover:text-ide-link-hover-color">
  <span class="flex-shrink-0 [&_div]:inline">
    <div class="show-file-icons">
      <div class="file-icon lapce-name-dir-icon readme.md-name-file-icon name-file-icon md-ext-file-icon ext-file-icon markdown-lang-file-icon monaco-icon-label"></div>
    </div>
  </span>
  <span data-state="closed">README.md</span>
</span>
```

### Tailwind Classes Breakdown:
- `inline-flex` - display: inline-flex
- `items-baseline` - align-items: baseline
- `gap-0.5` - gap: 0.125rem (2px)
- `break-all` - word-break: break-all
- `text-[0.9em]` - font-size: 0.9em
- `cursor-pointer` - cursor: pointer
- `decoration-current` - text-decoration-color: currentColor
- `hover:underline` - on hover, text-decoration-line: underline
- `font-mono` - font-family: monospace
- `leading-[1rem]` - line-height: 1rem
- `text-ide-link-color` - color: var(--vscode-textLink-foreground) = #4daafc
- `hover:text-ide-link-hover-color` - on hover, color: #4daafc

## TODO Section (lines 7665-7690)

### Container:
```html
<div class="leading-[1.6] prose prose-sm max-w-none break-words prose-headings:mb-1 prose-headings:text-base prose-headings:font-semibold prose-headings:text-ide-editor-color prose-h1:mt-[1.15rem] prose-h1:text-[1.15rem] prose-h2:mt-[1rem] prose-h2:text-[.9375rem] prose-h3:mb-2 prose-h3:mt-2 prose-p:mb-3 prose-p:block prose-p:leading-[1.6] prose-p:text-[var(--codeium-text-color)] prose-code:before:content-none prose-code:after:content-none prose-pre:my-2 prose-pre:bg-transparent prose-pre:p-0 prose-strong:font-semibold prose-strong:text-inherit prose-a:underline marker:text-inherit prose-ol:my-0 prose-ol:flex prose-ol:list-outside prose-ol:list-decimal prose-ol:flex-col prose-ol:gap-1 prose-ol:leading-[1.6] prose-ul:my-0 prose-ul:flex prose-ul:list-outside prose-ul:list-disc prose-ul:flex-col prose-ul:gap-1 prose-li:my-0 [&>ol]:mb-[16px] [&>ol]:mt-[12px] [&>ul]:mb-[16px] [&>ul]:mt-[12px] [&_ol]:mb-[8px] [&_ol]:mt-[6px] [&_ul]:mb-[8px] [&_ul]:mt-[6px] text-[color:var(--codeium-text-color)] prose-a:text-[color:var(--codeium-link-color)]">
  <h1>TODO</h1>
</div>
```

### TODO List:
```html
<div class="leading-[1.6] prose prose-sm ... (same classes)">
  <ul>
    <li node="[object Object]" class="leading-[1.5] [&>p]:inline">
      <p class="!mb-0 mt-0" node="[object Object]">
        <strong>Finalize Windsurf Input Implementation</strong><br>
        Build and integrate the Windsurf input bar using <span class="inline-flex items-baseline gap-0.5 break-all text-[0.9em] cursor-pointer decoration-current hover:underline font-mono leading-[1rem] text-ide-link-color hover:text-ide-link-hover-color">
          <span class="flex-shrink-0 [&_div]:inline">
            <div class="show-file-icons">
              <div class="file-icon ..."></div>
            </div>
          </span>
          <span data-state="closed">WINDSURF_INPUT_EXACT.md</span>
        </span> as the canonical reference...
      </p>
    </li>
  </ul>
</div>
```

### Key Points:
1. TODO items are in a `<ul>` with `<li class="leading-[1.5] [&>p]:inline">`
2. Each `<li>` contains `<p class="!mb-0 mt-0">`
3. Inside `<p>`: `<strong>Title</strong><br>Description`
4. File links are INLINE within the description text, not separate
5. Uses prose Tailwind typography plugin extensively

## Status Section (lines 7690-7693)
```html
<div class="leading-[1.6] prose prose-sm ... (same classes)">
  <h1>Status</h1>
</div>
<div class="leading-[1.6] prose prose-sm ... (same classes)">
  <ul>
    <li node="[object Object]" class="leading-[1.5] [&>p]:inline">
      <strong>Project surface analyzed.</strong>
    </li>
    <li node="[object Object]" class="leading-[1.5] [&>p]:inline">
      <strong>Structured TODO drafted for next steps.</strong>
    </li>
  </ul>
</div>
```

## Colors from CSS Variables:
- `--vscode-textLink-foreground: #4daafc`
- `--vscode-textLink-activeForeground: #4daafc`
- `--codeium-text-color` (main text color)
- `--ide-link-color` = #4daafc
- `--ide-link-hover-color` = #4daafc

## Font Sizes:
- File links: `text-[0.9em]` = 0.9em
- Line height: `leading-[1rem]` = 1rem
- TODO items: `leading-[1.5]` = line-height 1.5
- Prose: `leading-[1.6]` = line-height 1.6
