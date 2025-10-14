# VS Code Keybindings for Lapce

This fork includes **enhanced VS Code keybindings** built into the defaults, making Lapce feel just like VS Code out of the box!

## 🎯 What's Included

**50+ additional VS Code keybindings** added to the default configuration:

### Navigation
- `Alt+Left/Right` - Navigate backward/forward
- `Ctrl+G` - Go to line
- `Ctrl+Shift+O` - Go to symbol

### Panel Toggles
- `Ctrl+B` - Toggle sidebar
- `Ctrl+J` - Toggle bottom panel
- `Ctrl+Shift+E` - File explorer
- `Ctrl+Shift+F` - Search panel
- `Ctrl+Shift+X` - Extensions
- `Ctrl+Shift+M` - Problems
- `Ctrl+Shift+D` - Debug

### Line Operations
- `Alt+Up/Down` - Move line up/down
- `Alt+Shift+Up/Down` - Duplicate line
- `Ctrl+Enter` - New line below
- `Ctrl+Shift+Enter` - New line above
- `Ctrl+L` - Select current line

### Multi-Cursor
- `Ctrl+D` - Select next occurrence
- `Ctrl+Alt+Up/Down` - Add cursor above/below
- `Shift+Alt+I` - Add cursors to line ends
- `Ctrl+U` - Undo cursor

### Word Navigation
- `Ctrl+Left/Right` - Move by word
- `Ctrl+Backspace/Delete` - Delete word

### Tab Navigation
- `Ctrl+Tab` / `Ctrl+Shift+Tab` - Navigate tabs

### Split Editor
- `Ctrl+\` - Split editor vertically

### Code Actions
- `Ctrl+.` - Quick fix
- `Shift+Alt+F` - Format document
- `Ctrl+K Ctrl+I` - Show hover

### And many more!

## 🚀 Quick Start

### For Users Building from Source

```bash
# Clone this fork
git clone https://github.com/yourusername/lapce.git
cd lapce

# Build
cargo build --release

# Run
./target/release/lapce
```

**All VS Code keybindings work immediately!** No configuration needed.

### Default Settings

This fork comes with these defaults:
- ✅ Vim mode disabled (`modal = false`)
- ✅ VS Code keybindings enabled
- ✅ Dark theme
- ✅ All standard Lapce features

### Want Vim Mode?

If you prefer vim mode with your keybindings:

1. Open Settings (`Ctrl+,`)
2. Set `core.modal = true`
3. VS Code shortcuts will work in INSERT mode (press `i`)

## 📋 Complete Keybindings Reference

See `VSCODE_KEYBINDINGS_COMPLETE.md` for the full list of all 76 keybindings.

## 🔧 Customization

To add your own keybindings:

1. Open keyboard shortcuts: `Ctrl+K Ctrl+S`
2. Or edit: `~/.config/lapce-nightly/keymaps.toml`

Your custom keybindings will override the defaults.

## 📦 What Changed from Upstream

### Modified Files:
- `defaults/keymaps-nonmacos.toml` - Added 50+ VS Code keybindings
- `defaults/settings.toml` - Already had `modal = false` (unchanged)

### Backward Compatibility:
✅ All original Lapce keybindings still work  
✅ Can switch back to vim mode anytime  
✅ No breaking changes

## 🎉 Benefits

- **Zero configuration** - Works out of the box
- **Familiar** - Feels just like VS Code
- **Fast** - Native Rust performance
- **Compatible** - All Lapce features work normally

## 🤝 Contributing

If you find keybindings that don't work or want to add more, feel free to open an issue or PR!

## 📝 License

Same as upstream Lapce (Apache 2.0)
