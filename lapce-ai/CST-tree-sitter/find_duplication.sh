#!/bin/bash

echo "FINDING MEMORY DUPLICATION IN CODEBASE"
echo "======================================="
echo ""

echo "1. Multiple Cache Structures:"
echo "-----------------------------"
grep -rn "struct.*Cache" src/ | grep -v "^Binary" | head -20

echo ""
echo "2. Places storing Trees:"
echo "------------------------"
grep -rn "pub tree: Tree" src/

echo ""
echo "3. Places storing Source:"
echo "-------------------------"
grep -rn "pub source:" src/ | head -15

echo ""
echo "4. Duplicate CachedTree definitions:"
echo "-----------------------------------"
grep -rn "struct CachedTree" src/

echo ""
echo "5. Check if trees stored in multiple places:"
echo "--------------------------------------------"
echo "NativeParserManager has TreeCache"
grep -A 5 "pub struct NativeParserManager" src/native_parser_manager.rs | head -10

echo ""
echo "IntegratedSystem has cache"
grep -A 10 "pub struct IntegratedTreeSitter" src/integrated_system.rs | head -15

echo ""
echo "6. Check for HashMap/DashMap storing trees:"
echo "-------------------------------------------"
grep -rn "DashMap.*Tree\|HashMap.*Tree" src/ | head -10

