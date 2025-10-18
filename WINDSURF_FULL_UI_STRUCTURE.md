# Windsurf Chat Panel - Complete UI/UX Structure

## Main Layout Structure

From the actual Windsurf HTML (`outerhtml.html`), here's the complete UI hierarchy:

```
<div id="chat" class="flex h-full flex-col items-center justify-center outline-none">
  <div class="relative flex h-screen w-full flex-col items-center justify-between">
    
    <!-- TOP: Header/Navigation Bar -->
    <div class="z-40 flex flex-none flex-col gap-px focus:outline-none">
      <div class="flex flex-none items-center justify-between gap-1 px-2 pb-[5px] pt-1.5">
        <!-- Thread selector button -->
        <button class="mr-1 flex min-w-0 flex-1 items-center gap-1 opacity-70 transition-opacity hover:opacity-100">
          <div class="mr-1 flex min-w-0 flex-row items-center gap-1">
            <span class="truncate">{{Thread Title}}</span>
          </div>
          <svg><!-- search icon --></svg>
        </button>
        
        <!-- Action buttons (+ and settings) -->
        <div class="flex flex-row items-center justify-center gap-x-2">
          <button class="-m-1 flex cursor-pointer... p-1 hover:bg-neutral-500/10">
            <svg><!-- plus icon --></svg>
          </button>
          <button class="relative m-0 -my-px flex cursor-pointer...">
            <!-- settings button -->
          </button>
        </div>
      </div>
    </div>
    
    <!-- MIDDLE: Messages Area (scrollable) -->
    <div class="relative flex h-full w-full flex-col">
      <div class="relative flex h-full w-full flex-col" tabindex="-1">
        <!-- Messages container (scroll here) -->
        <!-- Welcome screen shown when empty -->
        <!-- Individual message rows -->
      </div>
    </div>
    
    <!-- BOTTOM: Input Area -->
    <div class="flex-none w-full">
      <!-- THIS IS WHERE THE INPUT BOX GOES -->
      <!-- Need to find this exact structure -->
    </div>
    
  </div>
</div>
```

---

## Key Tailwind Classes Used

### 1. **Main Container**
```
flex h-full flex-col items-center justify-center outline-none
```
- Vertical flex layout
- Full height
- Center items
- No outline

### 2. **Inner Container**
```
relative flex h-screen w-full flex-col items-center justify-between
```
- Relative positioning
- Full screen height
- Full width
- Vertical flex
- Space-between layout

### 3. **Header**
```
z-40 flex flex-none flex-col gap-px focus:outline-none
```
- Z-index 40 (above content)
- No flex grow/shrink
- Small gap between elements

### 4. **Messages Area**
```
relative flex h-full w-full flex-col
```
- Relative positioning
- Full height (grows to fill)
- Full width
- Vertical layout

---

## Colors from windsurf.css

```css
--vscode-input-background: #313131
--vscode-input-foreground: #cccccc
--vscode-input-border: #3c3c3c
--vscode-input-placeholderForeground: #989898

--vscode-inlineChatInput-background: #313131
--vscode-inlineChatInput-border: #454545
--vscode-inlineChatInput-focusBorder: #0078d4

--vscode-chat-requestBorder: rgba(255, 255, 255, 0.1)
--vscode-chat-requestBackground: rgba(31, 31, 31, 0.62)
```

---

## What We Need to Find

To complete this, I need the **exact HTML/classes for**:

1. **Input box container** at the bottom
2. **Message row** structure
3. **Welcome screen** layout
4. **Send button** exact styling

---

## Next Step

Since the file is 9269 lines, I can extract specific sections once you tell me which part you need most:

1. **Input box only** - Small, fast implementation
2. **Complete panel** - Full UI with messages, welcome screen, etc.

Which do you want to build first?
