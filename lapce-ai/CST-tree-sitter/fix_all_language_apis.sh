#!/bin/bash

# Fix all language API calls systematically

echo "Fixing all language API calls..."

# Fix Python - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_python::LANGUAGE/tree_sitter_python::LANGUAGE.into()/g' {} \;

# Fix Rust - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_rust::LANGUAGE/tree_sitter_rust::LANGUAGE.into()/g' {} \;

# Fix Go - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_go::LANGUAGE/tree_sitter_go::LANGUAGE.into()/g' {} \;

# Fix C - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_c::LANGUAGE/tree_sitter_c::LANGUAGE.into()/g' {} \;

# Fix C++ - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_cpp::LANGUAGE/tree_sitter_cpp::LANGUAGE.into()/g' {} \;

# Fix Ruby - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_ruby::LANGUAGE/tree_sitter_ruby::LANGUAGE.into()/g' {} \;

# Fix Java - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_java::LANGUAGE/tree_sitter_java::LANGUAGE.into()/g' {} \;

# Fix PHP - uses LANGUAGE_PHP constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_php::LANGUAGE_PHP/tree_sitter_php::LANGUAGE_PHP.into()/g' {} \;

# Fix Lua - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_lua::LANGUAGE/tree_sitter_lua::LANGUAGE.into()/g' {} \;

# Fix Bash - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_bash::LANGUAGE/tree_sitter_bash::LANGUAGE.into()/g' {} \;

# Fix CSS - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_css::LANGUAGE/tree_sitter_css::LANGUAGE.into()/g' {} \;

# Fix JSON - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_json::LANGUAGE/tree_sitter_json::LANGUAGE.into()/g' {} \;

# Fix HTML - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_html::LANGUAGE/tree_sitter_html::LANGUAGE.into()/g' {} \;

# Fix C# - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_c_sharp::LANGUAGE/tree_sitter_c_sharp::LANGUAGE.into()/g' {} \;

# Fix Swift - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_swift::LANGUAGE/tree_sitter_swift::LANGUAGE.into()/g' {} \;

# Fix Scala - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_scala::LANGUAGE/tree_sitter_scala::LANGUAGE.into()/g' {} \;

# Fix Elixir - uses LANGUAGE constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_elixir::LANGUAGE/tree_sitter_elixir::LANGUAGE.into()/g' {} \;

# Fix OCaml - uses LANGUAGE_OCAML constant that needs .into()
find src -name "*.rs" -exec sed -i 's/tree_sitter_ocaml::LANGUAGE_OCAML/tree_sitter_ocaml::LANGUAGE_OCAML.into()/g' {} \;

# Clean up double .into().into() that might have been created
find src -name "*.rs" -exec sed -i 's/\.into()\.into()/.into()/g' {} \;

# Remove unsafe blocks where not needed
find src -name "*.rs" -exec sed -i 's/unsafe { \([^}]*\)\.into() }/\1.into()/g' {} \;

echo "Fixed all language API calls!"
