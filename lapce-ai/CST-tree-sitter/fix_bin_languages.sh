#!/bin/bash

echo "Fixing bin files language calls..."

# Apply same fixes to bin files
for file in src/bin/*.rs; do
    # Fix languages that use constants
    sed -i 's/tree_sitter_python::LANGUAGE\([^.]\)/tree_sitter_python::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_rust::LANGUAGE\([^.]\)/tree_sitter_rust::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_go::LANGUAGE\([^.]\)/tree_sitter_go::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_c::LANGUAGE\([^.]\)/tree_sitter_c::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_cpp::LANGUAGE\([^.]\)/tree_sitter_cpp::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_ruby::LANGUAGE\([^.]\)/tree_sitter_ruby::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_java::LANGUAGE\([^.]\)/tree_sitter_java::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_php::LANGUAGE_PHP\([^.]\)/tree_sitter_php::LANGUAGE_PHP.into()\1/g' "$file"
    sed -i 's/tree_sitter_lua::LANGUAGE\([^.]\)/tree_sitter_lua::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_bash::LANGUAGE\([^.]\)/tree_sitter_bash::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_css::LANGUAGE\([^.]\)/tree_sitter_css::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_json::LANGUAGE\([^.]\)/tree_sitter_json::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_html::LANGUAGE\([^.]\)/tree_sitter_html::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_c_sharp::LANGUAGE\([^.]\)/tree_sitter_c_sharp::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_swift::LANGUAGE\([^.]\)/tree_sitter_swift::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_scala::LANGUAGE\([^.]\)/tree_sitter_scala::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_elixir::LANGUAGE\([^.]\)/tree_sitter_elixir::LANGUAGE.into()\1/g' "$file"
    sed -i 's/tree_sitter_ocaml::LANGUAGE_OCAML\([^.]\)/tree_sitter_ocaml::LANGUAGE_OCAML.into()\1/g' "$file"
    
    # Clean up double .into()
    sed -i 's/\.into()\.into()/.into()/g' "$file"
done

echo "Fixed bin files!"
