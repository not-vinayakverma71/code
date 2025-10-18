# Testing New Lapce Features

## ✅ What Was Added

### 1. File Type Icons (126 mappings!)
- Programming languages: `.rs`, `.py`, `.js`, `.ts`, `.go`, `.java`, etc.
- Config files: `.toml`, `.json`, `.yaml`, `.env`
- Special files: `Cargo.toml`, `package.json`, `README.md`, `LICENSE`
- Archives: `.zip`, `.tar`, `.gz`
- Media: `.png`, `.jpg`, `.mp4`
- And many more!

### 2. New VS Code Themes (5 total)
- Lapce Dark Modern (updated - darker blacks)
- Lapce Light (existing)
- Lapce Light Modern (NEW)
- Lapce Dark High Contrast (NEW)
- Lapce Light High Contrast (NEW)

## 🧪 How to Test

### Step 1: Kill Old Instances
```bash
killall lapce
```

### Step 2: Run New Build with Workspace
```bash
cd /home/verma/lapce
./target/release/lapce .
```
**OR** if you want to open a different folder:
```bash
./target/release/lapce /path/to/your/project
```

### Step 3: Verify File Icons
1. Click the **File Explorer** icon (📁) in the left sidebar
2. You should see:
   - `.rs` files with code icon
   - `.toml` files with gear icon
   - `README.md` with info icon
   - Different icons for different file types!

### Step 4: Verify Panels Work
All these buttons in the left sidebar should now work:

| Icon | Panel | What to Expect |
|------|-------|----------------|
| 📁 | File Explorer | Shows project files with icons |
| 🌿 | Source Control | Git status (if git repo) |
| 🔍 | Search | Global search in files |
| ⚠️ | Problems | LSP errors/warnings |
| 🐛 | Debug | Debugger panel |
| 🧩 | Plugins | Extension manager |
| 📄 | Document Symbol | File outline |
| 🖥️ | Terminal | Terminal (already worked ✅) |

### Step 5: Test New Themes
1. Open Command Palette: `Ctrl+Shift+P` (or `Cmd+Shift+P` on Mac)
2. Type "color theme"
3. Select from 5 themes:
   - Lapce Dark Modern ⬛ (darkest)
   - Lapce Light Modern ⬜ (clean modern light)
   - Lapce Dark High Contrast (for accessibility)
   - Lapce Light High Contrast (for accessibility)
   - Lapce Light (original)

## 🔧 Troubleshooting

### "Icons still not showing!"
**Solution**: Make sure you:
1. Killed all old Lapce processes: `killall lapce`
2. Are running `/home/verma/lapce/target/release/lapce` (the NEW build)
3. Opened a folder/workspace (File → Open Folder)

### "Panels are empty!"
**Panels need content to show:**
- **File Explorer**: Needs a folder open
- **Source Control**: Needs a git repository
- **Problems**: Needs files with errors (LSP must be running)
- **Search**: Needs a workspace
- **Plugins**: Should always work!

### "New themes not appearing!"
The themes are embedded in the binary. If you see only 2 themes instead of 5:
1. You're running an old version
2. Run: `./target/release/lapce` (not system installed lapce)

## 📊 Quick Verification Checklist

```bash
# 1. Verify binary has new icons
strings target/release/lapce | grep "rs.*file-code"
# Should output: "rs" = "file-code.svg"

# 2. Check binary modification time
ls -lh target/release/lapce
# Should show today's date at 17:45 or later

# 3. Test run with workspace
./target/release/lapce /home/verma/lapce
# Should open with file explorer showing different icons!
```

## 🎯 Expected Results

✅ **File Explorer shows icons for**:
- Rust files (.rs) → Code icon
- TOML files (.toml) → Gear icon  
- Markdown files (.md) → Info icon
- JSON files (.json) → JSON icon
- And 120+ more types!

✅ **All panel buttons work** when you have a workspace open

✅ **5 color themes** available in theme selector

✅ **Darker Dark Modern theme** - much deeper blacks like VS Code

---

## 💡 Pro Tip

To make this permanent, you can install it:
```bash
cd /home/verma/lapce
cargo install --path lapce-app --force
```

Then run just `lapce` from anywhere!
