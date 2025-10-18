# Windsurf SVG Icons - EXACT from HTML

Extracted from `small.html` (real Windsurf chat)

---

## 1. Plus Icon (+) - Add Files
**Size:** `h-3 w-3` (12px), `stroke-[2]`

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-plus h-3 w-3 stroke-[2]">
  <path d="M5 12h14"></path>
  <path d="M12 5v14"></path>
</svg>
```

---

## 2. Code Icon (</>)
**Size:** `size-[12px]` (12x12px), `stroke-[2.5px]`

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-code -mr-px size-[12px] stroke-[2.5px]">
  <path d="m16 18 6-6-6-6"></path>
  <path d="m8 6-6 6 6 6"></path>
</svg>
```

---

## 3. Microphone Icon (🎤)
**Size:** `h-3.5 w-3.5` (14px)

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-mic h-3.5 w-3.5">
  <path d="M12 19v3"></path>
  <path d="M19 10v2a7 7 0 0 1-14 0v-2"></path>
  <rect x="9" y="2" width="6" height="13" rx="3"></rect>
</svg>
```

---

## 4. Arrow Up Icon (↑) - Send Button
**Size:** `h-3 w-3` (12px)

```svg
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-arrow-up h-3 w-3">
  <path d="m5 12 7-7 7 7"></path>
  <path d="M12 19V5"></path>
</svg>
```

---

## Usage in Floem

Floem has an `svg()` view function. Here's how to use these:

```rust
use floem::views::svg;

// Plus icon
svg(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
  <path d="M5 12h14"/>
  <path d="M12 5v14"/>
</svg>"#)
.style(|s| s.width(12.0).height(12.0))

// Code icon
svg(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
  <path d="m16 18 6-6-6-6"/>
  <path d="m8 6-6 6 6 6"/>
</svg>"#)
.style(|s| s.width(12.0).height(12.0))

// Mic icon
svg(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
  <path d="M12 19v3"/>
  <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
  <rect x="9" y="2" width="6" height="13" rx="3"/>
</svg>"#)
.style(|s| s.width(14.0).height(14.0))

// Arrow up
svg(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
  <path d="m5 12 7-7 7 7"/>
  <path d="M12 19V5"/>
</svg>"#)
.style(|s| s.width(12.0).height(12.0))
```

---

## Next Steps

1. Replace emoji/text in `chat_text_area.rs` with these SVGs
2. Add color theming to SVG `stroke` attribute
3. Ensure proper scaling at different DPI settings
