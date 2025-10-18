# VS Code to Lapce Keybindings Migration Analysis

## Overview
This document provides a comprehensive analysis of VS Code default keybindings and their mapping to Lapce keybindings for Windows/Linux systems.

## Current Status Analysis

### ✅ Already Aligned with VS Code

| Action | VS Code | Lapce | Status |
|--------|---------|-------|--------|
| Command Palette | `Ctrl+Shift+P` | `Ctrl+Shift+P` | ✅ Match |
| Quick Open | `Ctrl+P` | `Ctrl+P` | ✅ Match |
| Go to Line | `Ctrl+G` | `Ctrl+G` | ✅ Match |
| Save | `Ctrl+S` | `Ctrl+S` | ✅ Match |
| Undo | `Ctrl+Z` | `Ctrl+Z` | ✅ Match |
| Redo | `Ctrl+Y` / `Ctrl+Shift+Z` | Both supported | ✅ Match |
| Cut/Copy/Paste | `Ctrl+X/C/V` | `Ctrl+X/C/V` | ✅ Match |
| Find | `Ctrl+F` | `Ctrl+F` | ✅ Match |
| Toggle Terminal | `Ctrl+`` | `Ctrl+`` | ✅ Match |
| Go to Definition | `F12` | `F12` | ✅ Match |
| Rename Symbol | `F2` | `F2` | ✅ Match |
| New File | `Ctrl+N` | `Ctrl+N` | ✅ Match |
| Open File | `Ctrl+O` | `Ctrl+O` | ✅ Match |
| Close Editor | `Ctrl+W` / `Ctrl+F4` | `Ctrl+W` | ✅ Match |
| Toggle Line Comment | `Ctrl+/` | `Ctrl+/` | ✅ Match |
| Split Editor | `Ctrl+\` | `Ctrl+\` | ✅ Match |
| Next/Previous Tab | `Ctrl+Tab` / `Ctrl+Shift+Tab` | Same | ✅ Match |
| Zoom In/Out | `Ctrl++` / `Ctrl+-` | `Ctrl+=` / `Ctrl+-` | ✅ Match |

---

### ❌ Missing or Different Keybindings (High Priority)

| Action | VS Code | Lapce Current | Gap Analysis |
|--------|---------|---------------|--------------|
| **Replace** | `Ctrl+H` | ❌ Not bound | 🔴 HIGH - Critical editing feature |
| **Go to Symbol in Workspace** | `Ctrl+T` | `Ctrl+T` (different) | 🔴 HIGH - Navigation |
| **Go to Symbol in File** | `Ctrl+Shift+O` | `Ctrl+Shift+O` | ✅ Match |
| **Toggle Sidebar** | `Ctrl+B` | ❌ Not bound | 🔴 HIGH - UI control |
| **Focus Explorer** | `Ctrl+Shift+E` | `Ctrl+Shift+E` | ✅ Match |
| **Focus Search** | `Ctrl+Shift+F` | `Ctrl+Shift+F` | ✅ Match |
| **Focus Problems** | `Ctrl+Shift+M` | `Ctrl+Shift+M` | ✅ Match |
| **Toggle Bottom Panel** | `Ctrl+J` | ❌ Not bound | 🔴 HIGH - UI control |
| **Select All Occurrences** | `Ctrl+Shift+L` | `Ctrl+Shift+L` | ✅ Match |
| **Add Selection to Next Find Match** | `Ctrl+D` | `Ctrl+D` | ✅ Match |
| **Move Line Up/Down** | `Alt+Up/Down` | `Alt+Up/Down` | ✅ Match |
| **Copy Line Up/Down** | `Alt+Shift+Up/Down` | `Alt+Shift+Up/Down` | ✅ Match |
| **Delete Line** | `Ctrl+Shift+K` | ❌ Not bound | 🟡 MEDIUM |
| **Insert Line Below** | `Ctrl+Enter` | `Ctrl+Enter` | ✅ Match |
| **Insert Line Above** | `Ctrl+Shift+Enter` | `Ctrl+Shift+Enter` | ✅ Match |
| **Go to Bracket** | `Ctrl+Shift+\` | `Ctrl+Shift+\` | ✅ Match |
| **Fold/Unfold** | `Ctrl+Shift+[` / `]` | ❌ Not bound | 🟡 MEDIUM |
| **Fold All** | `Ctrl+K Ctrl+0` | ❌ Not bound | 🟢 LOW |
| **Unfold All** | `Ctrl+K Ctrl+J` | ❌ Not bound | 🟢 LOW |
| **Add Cursor Above** | `Ctrl+Alt+Up` | `Ctrl+Alt+Up` | ✅ Match |
| **Add Cursor Below** | `Ctrl+Alt+Down` | `Ctrl+Alt+Down` | ✅ Match |
| **Select Current Line** | `Ctrl+L` | `Ctrl+L` | ✅ Match |
| **Undo Cursor** | `Ctrl+U` | `Ctrl+U` | ✅ Match |
| **Format Document** | `Shift+Alt+F` | ❌ Not bound | 🔴 HIGH |
| **Format Selection** | `Ctrl+K Ctrl+F` | ❌ Not bound | 🟡 MEDIUM |
| **Quick Fix (Code Actions)** | `Ctrl+.` | `Ctrl+.` | ✅ Match |
| **Show Hover** | `Ctrl+K Ctrl+I` | ❌ Not bound | 🟡 MEDIUM |
| **Peek Definition** | `Alt+F12` | ❌ Not bound | 🟡 MEDIUM |
| **Open Definition to Side** | `Ctrl+K F12` | ❌ Not bound | 🟢 LOW |
| **Go Back** | `Alt+Left` | ❌ Not bound | 🔴 HIGH |
| **Go Forward** | `Alt+Right` | ❌ Not bound | 🔴 HIGH |
| **Next/Previous Error** | `F8` / `Shift+F8` | `F8` / `Shift+F8` | ✅ Match |
| **Select to Beginning/End** | `Ctrl+Shift+Home/End` | ❌ Not bound | 🟡 MEDIUM |
| **Delete All Left** | `Ctrl+Backspace` | `Ctrl+Backspace` | ✅ Match |
| **Delete All Right** | `Ctrl+Delete` | `Ctrl+Delete` | ✅ Match |
| **Expand Selection** | `Shift+Alt+Right` | ❌ Not bound | 🟡 MEDIUM |
| **Shrink Selection** | `Shift+Alt+Left` | ❌ Not bound | 🟡 MEDIUM |
| **Column Selection** | `Shift+Alt+I` | `Alt+Shift+I` (different?) | 🟡 Check |
| **New Window** | `Ctrl+Shift+N` | ❌ Not bound | 🟢 LOW |
| **Close Window** | `Alt+F4` | `Alt+F4` | ✅ Match |
| **Split Editor Right** | `Ctrl+\` | ✅ Match | ✅ Match |
| **Close Folder** | `Ctrl+K F` | `Ctrl+K F` | ✅ Match |
| **Toggle Integrated Terminal** | `Ctrl+`` | `Ctrl+`` | ✅ Match |
| **Open Settings** | `Ctrl+,` | `Ctrl+,` | ✅ Match |
| **Open Keyboard Shortcuts** | `Ctrl+K Ctrl+S` | `Ctrl+K Ctrl+S` | ✅ Match |
| **Save All** | `Ctrl+K S` | ❌ Not bound | 🟡 MEDIUM |
| **Close All Editors** | `Ctrl+K Ctrl+W` | ❌ Not bound | 🟡 MEDIUM |
| **Reopen Closed Editor** | `Ctrl+Shift+T` | ❌ Not bound | 🔴 HIGH |
| **Keep Editor** | `Ctrl+K Enter` | ❌ Not bound | 🟢 LOW |
| **Show All Commands** | `Ctrl+Shift+P` | ✅ Match | ✅ Match |
| **Navigate Editor Group** | `Ctrl+1/2/3/4` | ❌ Not bound | 🟡 MEDIUM |
| **Duplicate Lines** | (none in VS Code) | `Alt+Shift+Up/Down` | Different approach |

---

### 🆕 Additional VS Code Keybindings to Consider

| Action | VS Code | Priority | Notes |
|--------|---------|----------|-------|
| Toggle Word Wrap | `Alt+Z` | 🟡 MEDIUM | Useful for long lines |
| Show All Symbols | `Ctrl+T` | 🔴 HIGH | Already conflicts |
| Trigger Suggestion | `Ctrl+Space` | 🔴 HIGH | Already in Lapce |
| Trigger Parameter Hints | `Ctrl+Shift+Space` | 🔴 HIGH | Already in Lapce |
| Replace in Files | `Ctrl+Shift+H` | 🔴 HIGH | Missing |
| Toggle Find Whole Word | `Alt+W` | 🟡 MEDIUM | In search |
| Toggle Find Case Sensitive | `Alt+C` | 🟡 MEDIUM | In search |
| Toggle Find Regex | `Alt+R` | 🟡 MEDIUM | In search |
| Show Workspace Symbols | `Ctrl+T` | 🔴 HIGH | Navigation |
| Transpose Letters | ❌ | 🟢 LOW | Minor feature |

---

## Critical Missing Keybindings (Must Fix)

### Priority 1: 🔴 HIGH (Daily use, muscle memory)

1. **`Ctrl+H`** - Replace (currently unbound, conflicts with `delete_backward` in Lapce's vim mode)
2. **`Ctrl+B`** - Toggle Sidebar Visibility  
3. **`Ctrl+J`** - Toggle Bottom Panel
4. **`Alt+Left`** / **`Alt+Right`** - Navigate Back/Forward in edit history
5. **`Ctrl+Shift+T`** - Reopen Closed Editor
6. **`Shift+Alt+F`** - Format Document
7. **`Ctrl+T`** - Go to Symbol in Workspace (currently used for different command)

### Priority 2: 🟡 MEDIUM (Regular use)

1. **`Ctrl+Shift+K`** - Delete Line
2. **`Ctrl+K Ctrl+F`** - Format Selection
3. **`Alt+F12`** - Peek Definition
4. **`Ctrl+K Ctrl+I`** - Show Hover Info
5. **`Ctrl+K S`** - Save All
6. **`Ctrl+K Ctrl+W`** - Close All Editors
7. **`Ctrl+1/2/3/4`** - Focus Editor Group N
8. **`Shift+Alt+Left/Right`** - Expand/Shrink Selection
9. **`Ctrl+Shift+[/]`** - Fold/Unfold
10. **`Ctrl+Shift+H`** - Replace in Files

### Priority 3: 🟢 LOW (Occasional use)

1. **`Ctrl+K Ctrl+0`** - Fold All
2. **`Ctrl+K Ctrl+J`** - Unfold All
3. **`Ctrl+K F12`** - Open Definition to Side
4. **`Ctrl+K Enter`** - Keep Editor (Pin)
5. **`Ctrl+Shift+N`** - New Window
6. **`Alt+Z`** - Toggle Word Wrap

---

## Recommended Implementation Plan

### Phase 1: Fix Critical Conflicts & Add Essential Keybindings

**File to modify**: `/home/verma/lapce/defaults/keymaps-nonmacos.toml`

```toml
# ===================== PHASE 1: CRITICAL ADDITIONS ====================

# Replace (must-have)
[[keymaps]]
key = "ctrl+h"
command = "replace" # Or "search_replace" depending on command name
mode = "i"

# Toggle Sidebar
[[keymaps]]
key = "ctrl+b"
command = "toggle_sidebar"

# Toggle Bottom Panel  
[[keymaps]]
key = "ctrl+j"
command = "toggle_panel"

# Navigate Back/Forward
[[keymaps]]
key = "alt+left"
command = "jump_location_backward"
mode = "i"

[[keymaps]]
key = "alt+right"
command = "jump_location_forward"
mode = "i"

# Reopen Closed Editor
[[keymaps]]
key = "ctrl+shift+t"
command = "reopen_closed_editor"

# Format Document
[[keymaps]]
key = "shift+alt+f"
command = "format_document"
mode = "i"
```

### Phase 2: Medium Priority Enhancements

```toml
# Delete Line
[[keymaps]]
key = "ctrl+shift+k"
command = "delete_line"
mode = "i"

# Replace in Files
[[keymaps]]
key = "ctrl+shift+h"
command = "global_search_replace"

# Focus Editor Groups
[[keymaps]]
key = "ctrl+1"
command = "focus_editor_group_1"

[[keymaps]]
key = "ctrl+2"
command = "focus_editor_group_2"

[[keymaps]]
key = "ctrl+3"
command = "focus_editor_group_3"

# Fold/Unfold
[[keymaps]]
key = "ctrl+shift+["
command = "fold"

[[keymaps]]
key = "ctrl+shift+]"
command = "unfold"
```

### Phase 3: Low Priority Polish

```toml
# Save All
[[keymaps]]
key = "ctrl+k s"
command = "save_all"

# Close All Editors
[[keymaps]]
key = "ctrl+k ctrl+w"
command = "close_all_editors"

# Toggle Word Wrap
[[keymaps]]
key = "alt+z"
command = "toggle_word_wrap"
```

---

## Conflicts to Resolve

| Keybinding | Current Lapce Use | VS Code Use | Resolution |
|------------|-------------------|-------------|------------|
| `Ctrl+H` | `delete_backward` (vim mode) | Replace | **Keep `Ctrl+H` for Replace in insert mode**, preserve vim mode behavior |
| `Ctrl+B` | `left` (vim normal) | Toggle Sidebar | **Add `Ctrl+B` for sidebar**, keep vim `h` for left |
| `Ctrl+T` | `palette.workspace_symbol` | Go to Symbol in Workspace | **Already correct**, both do same thing |
| `Ctrl+J` | Unbound | Toggle Bottom Panel | **Add safely** |
| `Alt+Left/Right` | Unbound | Navigate history | **Add safely** |

---

## Commands to Verify Exist in Lapce

Need to check if these commands exist in Lapce codebase:
- `replace` or `search_replace`
- `toggle_sidebar`
- `toggle_panel` or `toggle_bottom_panel`
- `reopen_closed_editor`
- `format_document`
- `delete_line`
- `global_search_replace`
- `focus_editor_group_N`
- `fold` / `unfold` / `fold_all` / `unfold_all`
- `save_all`
- `close_all_editors`
- `toggle_word_wrap`
- `peek_definition`
- `show_hover`

---

## Next Steps

1. **Verify Command Names**: Check `/home/verma/lapce/lapce-app/src/command.rs` for exact command names
2. **Test Conflicts**: Ensure new keybindings don't break existing functionality
3. **Update keymaps-nonmacos.toml**: Add/modify keybindings in phases
4. **Update keymaps-macos.toml**: Mirror changes with Cmd key
5. **Test thoroughly**: Ensure muscle memory from VS Code works
6. **Document changes**: Update user-facing documentation

---

## Success Criteria

✅ All Priority 1 keybindings work identically to VS Code
✅ No conflicts with existing vim mode keybindings  
✅ Users can switch from VS Code with minimal friction
✅ All keybindings properly documented

