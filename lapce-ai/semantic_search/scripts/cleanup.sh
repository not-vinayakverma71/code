#!/bin/bash
# Safe cleanup script using trash-put instead of rm
# Security: CST-SEC02 - Files moved to trash instead of permanent deletion

set -euo pipefail

# Check if trash-put is available
if ! command -v trash-put &> /dev/null; then
    echo "ERROR: trash-put not found. Please install trash-cli:"
    echo "  Ubuntu/Debian: sudo apt-get install trash-cli"
    echo "  macOS: brew install trash-cli"
    echo "  Arch: sudo pacman -S trash-cli"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "🧹 Cleaning up semantic_search project"
echo "Project root: $PROJECT_ROOT"
echo ""

# Function to safely remove files/directories
safe_remove() {
    local path="$1"
    local description="$2"
    
    if [ -e "$path" ]; then
        echo "  Moving to trash: $description"
        trash-put "$path"
    else
        echo "  Skipping (not found): $description"
    fi
}

cd "$PROJECT_ROOT"

# Clean build artifacts
echo "1️⃣ Cleaning build artifacts..."
safe_remove "target/" "Cargo build artifacts"
safe_remove "Cargo.lock" "Cargo lock file (will be regenerated)"

# Clean test artifacts
echo ""
echo "2️⃣ Cleaning test artifacts..."
safe_remove "test_output.log" "Test output logs"
safe_remove "build_errors.log" "Build error logs"
safe_remove "full_benchmark_results.log" "Benchmark results"
safe_remove "ci_failure.log" "CI failure logs"
safe_remove "nuclear_windows_transport.log" "Transport logs"

# Clean temporary files
echo ""
echo "3️⃣ Cleaning temporary files..."
safe_remove "/tmp/test_shm.rs" "Temporary shared memory test"
safe_remove "/tmp/test_shm" "Compiled test binary"
safe_remove "/tmp/test_shm.bin" "Temporary shared memory file"
safe_remove "/tmp/zero_copy.bin" "Zero-copy test file"

# Clean coverage artifacts
echo ""
echo "4️⃣ Cleaning coverage artifacts..."
safe_remove "lcov-*.info" "Coverage info files"
safe_remove "target/llvm-cov/" "LLVM coverage data"

# Clean benchmark artifacts
echo ""
echo "5️⃣ Cleaning benchmark artifacts..."
safe_remove "target/criterion/" "Criterion benchmark data"

# Clean cache directories
echo ""
echo "6️⃣ Cleaning cache directories..."
safe_remove ".cache/" "Local cache directory"
safe_remove "storage/" "Test storage directory"

# Clean IDE artifacts
echo ""
echo "7️⃣ Cleaning IDE artifacts..."
safe_remove ".vscode/" "VSCode settings"
safe_remove ".idea/" "IntelliJ settings"
safe_remove "*.swp" "Vim swap files"
safe_remove "*.swo" "Vim swap files"

# Clean macOS artifacts
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo ""
    echo "8️⃣ Cleaning macOS artifacts..."
    safe_remove ".DS_Store" "macOS metadata"
    find . -name ".DS_Store" -type f -exec trash-put {} \; 2>/dev/null || true
fi

echo ""
echo "✅ Cleanup complete!"
echo ""
echo "📝 Notes:"
echo "  - All files moved to trash (can be recovered)"
echo "  - Use 'trash-list' to see deleted files"
echo "  - Use 'trash-restore' to recover files"
echo "  - Use 'trash-empty' to permanently delete"
