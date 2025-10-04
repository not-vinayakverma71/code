#!/usr/bin/env python3
"""
Extract all Codex query strings from TypeScript files and create .scm files
"""
import os
import re
from pathlib import Path

CODEX_QUERIES = "/home/verma/lapce/Codex/src/services/tree-sitter/queries"
TARGET_QUERIES = "/home/verma/lapce/lapce-ai-rust/CST-tree-sitter/queries"

# Mapping of Codex .ts files to target directories
LANGUAGE_MAPPING = {
    "go.ts": "go",
    "c.ts": "c",
    "cpp.ts": "cpp",
    "c-sharp.ts": "c-sharp",
    "ruby.ts": "ruby",
    "java.ts": "java",
    "php.ts": "php",
    "swift.ts": "swift",
    "kotlin.ts": "kotlin",
    "css.ts": "css",
    "html.ts": "html",
    "ocaml.ts": "ocaml",
    "solidity.ts": "solidity",
    "toml.ts": "toml",
    "vue.ts": "vue",
    "lua.ts": "lua",
    "scala.ts": "scala",
    "zig.ts": "zig",
    "systemrdl.ts": "systemrdl",
    "tlaplus.ts": "tlaplus",
    "embedded_template.ts": "embedded-template",
    "elisp.ts": "elisp",
    "elixir.ts": "elixir",
}

def extract_query_from_ts(ts_file_path):
    """Extract the query string from a TypeScript file."""
    with open(ts_file_path, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Find the export default `...` pattern
    # Handle both: export default `...` and export const name = `...`
    match = re.search(r'export\s+(?:default|const\s+\w+\s*=)\s*`([^`]*)`', content, re.DOTALL)
    if match:
        query_string = match.group(1)
        return query_string
    
    return None

def create_tags_scm(lang_dir, query_string, source_file):
    """Create tags.scm file with the extracted query."""
    tags_file = os.path.join(lang_dir, "tags.scm")
    
    # Add header comment
    header = f"; Extracted from Codex {os.path.basename(source_file)}\n\n"
    
    with open(tags_file, 'w', encoding='utf-8') as f:
        f.write(header)
        f.write(query_string)
        if not query_string.endswith('\n'):
            f.write('\n')
    
    print(f"✅ Created {tags_file}")

def main():
    success_count = 0
    failed_count = 0
    
    for ts_file, lang_dir_name in LANGUAGE_MAPPING.items():
        ts_path = os.path.join(CODEX_QUERIES, ts_file)
        lang_dir = os.path.join(TARGET_QUERIES, lang_dir_name)
        
        # Check if TypeScript source exists
        if not os.path.exists(ts_path):
            print(f"❌ Source not found: {ts_path}")
            failed_count += 1
            continue
        
        # Create language directory if it doesn't exist
        if not os.path.exists(lang_dir):
            os.makedirs(lang_dir, exist_ok=True)
            print(f"📁 Created directory: {lang_dir}")
            
            # Create empty files for other query types
            for query_type in ['highlights.scm', 'injections.scm', 'locals.scm', 'folds.scm']:
                empty_file = os.path.join(lang_dir, query_type)
                if not os.path.exists(empty_file):
                    with open(empty_file, 'w') as f:
                        f.write(f"; {lang_dir_name} {query_type}\n; TODO: Add language-specific queries\n")
        
        # Extract and create tags.scm
        try:
            query_string = extract_query_from_ts(ts_path)
            if query_string:
                create_tags_scm(lang_dir, query_string, ts_path)
                success_count += 1
            else:
                print(f"⚠️  Could not extract query from {ts_file}")
                failed_count += 1
        except Exception as e:
            print(f"❌ Error processing {ts_file}: {e}")
            failed_count += 1
    
    print(f"\n{'='*60}")
    print(f"✅ Success: {success_count}/{len(LANGUAGE_MAPPING)}")
    print(f"❌ Failed: {failed_count}/{len(LANGUAGE_MAPPING)}")
    print(f"{'='*60}")

if __name__ == "__main__":
    main()
