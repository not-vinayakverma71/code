# ⚠️ IMPORTANT: Don't Click Update Button!

## What Happened

You clicked the "Update" button in Lapce → It replaced your custom-built Lapce with the official version → Lost your AI Chat panel!

## Why This Happened

- **Official Lapce:** Downloaded from releases, NO AI panel
- **Your Custom Lapce:** `./target/release/lapce` with AI panel built-in
- **Update button:** Downloads official version, overwrites your build

## ✅ Solution (Done)

Just rebuilt and restarted your custom version:
```bash
cargo build --package lapce-app --release
./target/release/lapce &
```

Your AI panel is back! ✅

## 🚫 DON'T Click These in Lapce:

1. ❌ **"Check for Updates" button**
2. ❌ **"Update Lapce" button** 
3. ❌ **Any auto-update prompts**

## ✅ DO This Instead:

### To Update YOUR Custom Lapce:
```bash
cd /home/verma/lapce
git pull  # If tracking upstream
cargo build --package lapce-app --release
./target/release/lapce &
```

## 🎯 How to Always Use Your Custom Build

### Option 1: Shell Alias (Recommended)
Add to `~/.bashrc` or `~/.zshrc`:
```bash
alias lapce='/home/verma/lapce/target/release/lapce'
```

Then just type `lapce` in terminal!

### Option 2: Desktop Shortcut
Create `~/.local/share/applications/lapce-custom.desktop`:
```desktop
[Desktop Entry]
Name=Lapce (Custom AI Build)
Exec=/home/verma/lapce/target/release/lapce
Type=Application
Icon=lapce
Categories=Development;
```

### Option 3: Symlink (Advanced)
```bash
# Find official lapce location
which lapce  # e.g., /usr/bin/lapce or ~/.local/bin/lapce

# Replace with your build (backup first!)
sudo mv /usr/bin/lapce /usr/bin/lapce.official
sudo ln -s /home/verma/lapce/target/release/lapce /usr/bin/lapce
```

## 🔒 Prevent Auto-Updates

### Disable Lapce Auto-Update (if it exists):
1. Open Lapce settings
2. Search for "update"
3. Disable any auto-update settings

## 📝 Remember

**You're developing Lapce itself!**
- Official Lapce = No AI panel
- Your build = Has AI panel + all your work
- Always run from `./target/release/lapce`

## 🎉 Current Status

- ✅ Code still exists
- ✅ Rebuilt in 6 seconds
- ✅ Running your custom version
- ✅ AI panel restored!

**All your work is safe!** Just don't click update buttons in the IDE. 😊
