# VSCode Dark Modern Theme for Lapce 🎨

## What Changed

Converted Lapce's default dark theme to **VSCode Dark Modern** colors!

### Before vs After

| Element | Old (Lapce Grey) | New (VSCode Modern) |
|---------|------------------|---------------------|
| **Editor Background** | `#282C34` (grey) | `#1F1F1F` (deep black) |
| **Panel Background** | `#21252B` (grey) | `#181818` (darker) |
| **Current Line** | `#2C313C` | `#2A2D2E` |
| **Selection** | `#3E4451` | `#264F78` (blue highlight) |
| **Text** | `#ABB2BF` (dim) | `#CCCCCC` (brighter) |
| **Borders** | `#000000` (black) | `#454545` (visible grey) |
| **Scrollbar** | `#3E4451BB` | `#424242` |

---

## Key Improvements

### 🖤 **Deeper Blacks**
- Main background: `#1F1F1F` (almost pure black)
- Panels: `#181818` (even darker)
- Much better contrast than the old grey theme

### 🎨 **Better Syntax Colors**
- Blue: `#569CD6` (VSCode's signature blue)
- Green: `#6A9955` (for strings)
- Yellow: `#DCDCAA` (for types/constants)
- Cyan: `#4EC9B0` (for built-ins)
- Orange: `#CE9178` (for numbers/strings)
- Purple: `#C586C0` (for keywords)

### ✨ **Improved UI Elements**
- Selection: `#264F78` (blue highlight like VSCode)
- Current line: `#2A2D2E` (subtle highlight)
- Indent guides: `#404040` (visible but not distracting)
- Caret: `#AEAFAD` (bright and visible)
- Buttons: `#0E639C` (VSCode blue)

---

## How to Apply

### Option 1: Restart Lapce (Automatic)
```bash
pkill lapce
./target/release/lapce &
```

The theme will load automatically as it's the default "Lapce Dark".

### Option 2: Switch Theme Manually
1. Open Lapce Settings (Ctrl+,)
2. Search for "color theme"
3. Select "Lapce Dark (VSCode Modern)"

---

## Color Palette Reference

### Base Colors
```toml
black   = "#1F1F1F"  # Deep black background
white   = "#CCCCCC"  # Bright text
blue    = "#569CD6"  # Functions, tags
cyan    = "#4EC9B0"  # Built-in types
green   = "#6A9955"  # Strings
yellow  = "#DCDCAA"  # Types, constants
orange  = "#CE9178"  # Numbers, strings
red     = "#F48771"  # Errors, fields
purple  = "#C586C0"  # Keywords
grey    = "#3E3E42"  # Borders, UI elements
```

### Background Hierarchy
```
#181818  → Panels (darkest)
#1F1F1F  → Editor background
#252526  → Secondary elements
#2A2D2E  → Hover/selected states
#454545  → Borders
```

---

## What This Fixes

✅ **No more dim grey everywhere**  
✅ **Deep black background like VSCode**  
✅ **Better text contrast (brighter white)**  
✅ **Visible borders (`#454545` instead of black)**  
✅ **Blue selection highlight**  
✅ **VSCode-style button colors**  
✅ **Improved syntax highlighting**

---

## Screenshots Reference

### VSCode Dark Modern Features:
- Pure black sidebar and panels
- Deep dark editor area
- Blue selection highlights
- Subtle line highlighting
- Clear borders between sections
- High contrast text

### Now in Lapce:
All the above features are now in Lapce's default dark theme!

---

## Customization

If you want to tweak colors further, edit:
```
/home/verma/lapce/defaults/dark-theme.toml
```

Then rebuild:
```bash
cargo build --release --package lapce-app
```

---

## Theme Name

The theme is now called: **"Lapce Dark (VSCode Modern)"**

You'll see this name in:
- Settings → Color Theme dropdown
- Theme selector in Lapce

---

## Summary

✅ **Converted** default Lapce Dark to VSCode Dark Modern  
✅ **Deeper blacks** for better contrast  
✅ **VSCode-style colors** throughout  
✅ **Better visibility** for all UI elements  
✅ **Rebuilt** successfully  

**Enjoy your VSCode-style Lapce experience!** 🚀
