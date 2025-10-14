# 🎨 Themes & Icons in Your Lapce Fork

## 📊 What Users Will Get From Your Repo

When someone clones and builds your Lapce fork, they will get:

---

## 🎨 **Color Themes: 5 Built-in**

Your repo includes **5 color themes** compiled into the binary:

| # | Theme Name | Type | Default |
|---|------------|------|---------|
| 1 | **Lapce Dark Modern** | Dark | ✅ YES |
| 2 | Lapce Light | Light | |
| 3 | Lapce Light Modern | Light | |
| 4 | Lapce Dark High Contrast | Dark HC | |
| 5 | Lapce Light High Contrast | Light HC | |

### Default Theme
**Location:** `defaults/settings.toml` line 5
```toml
color-theme = "Lapce Dark"
```

**Note:** This is actually "Lapce Dark Modern" in the actual theme file (`defaults/dark-theme.toml`).

### Your Current Theme
You're using: **"Lapce Dark Modern"** (in `~/.config/lapce-nightly/settings.toml`)

---

## 🎯 **Icon Theme: 1 Built-in**

Your repo includes **1 icon theme** compiled into the binary:

| # | Icon Theme Name | Description | Default |
|---|----------------|-------------|---------|
| 1 | **Lapce Codicons** | VS Code Codicons style icons | ✅ YES |

### Default Icon Theme
**Location:** `defaults/settings.toml` line 6
```toml
icon-theme = "Lapce Codicons"
```

### Icon Sources
**Location:** `/home/verma/lapce/icons/`
- `codicons/` - 941 icon files (UI icons, symbols, etc.)
- `file-types/` - 18 file-type specific icons
- `lapce/` - 2 Lapce-specific icons

---

## 📁 **File & Directory Icons**

### Directory Icons
**Closed folder:** `folder.svg` (24x24 pixels)
```svg
<svg width="24" height="24" viewBox="0 0 24 24">
```

**Opened folder:** `folder-opened.svg` (24x24 pixels)

### File Icons

**Generic file:** `file.svg` (16x16 pixels)
```svg
<svg width="16" height="16" viewBox="0 0 16 16">
```

**File-type specific icons** (18 types):
1. `c.svg` - C files
2. `cpp.svg` - C++ files
3. `css.svg` - CSS files
4. `go.svg` - Go files
5. `html.svg` - HTML files
6. `java.svg` - Java files
7. `javascript.svg` - JavaScript files
8. `json.svg` - JSON files
9. `markdown.svg` - Markdown files
10. `php.svg` - PHP files
11. `python.svg` - Python files
12. `ruby.svg` - Ruby files
13. `rust.svg` - Rust files (3.4 KB - detailed!)
14. `shell.svg` - Shell scripts
15. `toml.svg` - TOML files
16. `typescript.svg` - TypeScript files
17. `xml.svg` - XML files
18. `yaml.svg` - YAML files

---

## 📏 **Icon Sizes**

### Default Icon Size
**Location:** `defaults/settings.toml` line 92
```toml
[ui]
icon-size = 0  # 0 means use default size
```

**Default sizes:**
- **Folder icons:** 24x24 pixels
- **File icons:** 16x16 pixels
- **UI icons:** Varies (16x16, 20x20, 24x24)

### Customizable Icon Size
Users can change icon size in their settings:
```toml
[ui]
icon-size = 14  # Custom size (e.g., 14, 16, 18, 20, 24)
```

Your current icon size: **0** (default, not overridden)

---

## 🔍 **Detailed Icon Breakdown**

### UI Icons (941 files in `icons/codicons/`)
These include:
- File explorer icons
- Git status icons
- Debug icons
- Terminal icons
- Search icons
- Settings icons
- Panel icons
- And many more...

**Example UI icons:**
- `menu.svg` - Menu button
- `close.svg` - Close button
- `settings-gear.svg` - Settings
- `terminal.svg` - Terminal
- `extensions.svg` - Extensions
- `search.svg` - Search
- `error.svg` - Error indicator
- `warning.svg` - Warning indicator
- `debug.svg` - Debug icon
- `source-control.svg` - Git icon

---

## 📦 **What's Compiled Into Binary**

When users build your fork, these files are **embedded** into the binary:

### Theme Files (6 files):
1. `defaults/dark-theme.toml` (5.7 KB)
2. `defaults/light-theme.toml` (5.7 KB)
3. `defaults/light-modern-theme.toml` (5.7 KB)
4. `defaults/dark-highcontrast-theme.toml` (5.8 KB)
5. `defaults/light-highcontrast-theme.toml` (5.8 KB)
6. `defaults/icon-theme.toml` (12.5 KB)

**Total themes size:** ~35 KB

### Icon Files (961 files):
- 941 codicon SVG files
- 18 file-type SVG files
- 2 Lapce logo files

**Total icons size:** ~2-3 MB (estimated)

---

## 🎯 **What Users See By Default**

When someone builds and runs your Lapce fork:

1. **Theme:** Lapce Dark Modern (dark theme) ✅
2. **Icon Theme:** Lapce Codicons ✅
3. **Folder Icon:** Yellow/blue folder (24x24) ✅
4. **File Icons:** 
   - Generic: White document icon (16x16) ✅
   - Rust files: Rust logo icon ✅
   - Python files: Python logo icon ✅
   - JavaScript files: JS logo icon ✅
   - etc. (18 file types have custom icons)
5. **Icon Size:** Default (0 = auto-sized based on UI scale)

---

## 🎨 **Switching Themes**

Users can switch themes in two ways:

### Method 1: Settings UI
1. Open settings: `Ctrl+,`
2. Search for "theme"
3. Select from dropdown:
   - Lapce Dark Modern ⭐
   - Lapce Light
   - Lapce Light Modern
   - Lapce Dark High Contrast
   - Lapce Light High Contrast

### Method 2: Edit Config
Edit `~/.config/lapce-nightly/settings.toml`:
```toml
[core]
color-theme = "Lapce Light Modern"  # Change theme
icon-theme = "Lapce Codicons"       # Icon theme (only 1 available)
```

---

## 📐 **Changing Icon Size**

Edit `~/.config/lapce-nightly/settings.toml`:
```toml
[ui]
icon-size = 18  # Options: 12, 14, 16, 18, 20, 24 (or 0 for default)
```

**Common sizes:**
- `12` - Very small (compact)
- `14` - Small
- `16` - Default for files
- `18` - Medium
- `20` - Large
- `24` - Extra large (default for folders)

---

## 🆚 **Comparison: You vs. Users**

| Feature | Your Machine | Users Building From Repo |
|---------|--------------|--------------------------|
| **Themes Available** | 5 built-in | 5 built-in ✅ Same |
| **Default Theme** | Lapce Dark Modern | Lapce Dark Modern ✅ Same |
| **Your Active Theme** | Lapce Dark Modern | Lapce Dark Modern (default) ✅ |
| **Icon Theme** | Lapce Codicons | Lapce Codicons ✅ Same |
| **Icon Size** | 1.7 (70% larger) | 1.7 (70% larger) ✅ Same |
| **Folder Icons** | 24x24 yellow folders | 24x24 yellow folders ✅ Same |
| **File Icons** | 16x16 + 18 custom | 16x16 + 18 custom ✅ Same |

---

## 🎁 **Additional Themes & Icons**

Users can add more themes via:

1. **Plugins/Extensions** - Install theme plugins (if available in Lapce plugin registry)
2. **Custom Theme Files** - Place `.toml` theme files in `~/.config/lapce-nightly/themes/`
3. **Your Repo** - You can add more theme files to `defaults/` and they'll be built-in

---

## 📝 **Summary**

### For Users of Your Fork:

✅ **5 color themes** included  
✅ **1 icon theme** included  
✅ **Lapce Dark Modern** as default  
✅ **961 icon files** embedded  
✅ **18 file-type icons** (Rust, Python, JS, etc.)  
✅ **24x24 folder 
2. **Custom Theme Files** - Place `.toml` theme files in `~/.config/lapce-nightly/themes/`
3. **Your Repo** - You can add more theme files to `defaults/` and they'll be built-in

---

## 📝 **Summary**

### For Users of Your Fork:

✅ **5 color themes** included  
✅ **1 icon theme** inicons**  
✅ **16x16 file icons**  
✅ **Customizable icon size**  
✅ **Same experience as you**  

**Everyone who builds from your repo gets the exact same themes and icons!** 🎨
