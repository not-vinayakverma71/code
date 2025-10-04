#!/bin/bash
# Download ALL 122 tree-sitter grammar repositories

set -e
cd grammars/all-grammars

# Core languages (17)
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-rust rust
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-javascript javascript
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-typescript typescript
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-python python
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-go go
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-c c
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-cpp cpp
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-java java
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-c-sharp c-sharp
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-ruby ruby
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-php php
git clone --depth 1 https://github.com/alex-pinkus/tree-sitter-swift swift
git clone --depth 1 https://github.com/fwcd/tree-sitter-kotlin kotlin
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-scala scala
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-bash bash
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-html html
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-css css

# Data formats (10)
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-json json
git clone --depth 1 https://github.com/ikatyang/tree-sitter-yaml yaml
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-xml xml
git clone --depth 1 https://github.com/ikatyang/tree-sitter-toml toml
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-markdown markdown
git clone --depth 1 https://github.com/camdencheek/tree-sitter-dockerfile dockerfile
git clone --depth 1 https://github.com/DerekStride/tree-sitter-sql sql
git clone --depth 1 https://github.com/bkegley/tree-sitter-graphql graphql
git clone --depth 1 https://github.com/mitchellh/tree-sitter-proto proto
git clone --depth 1 https://github.com/MichaHoffmann/tree-sitter-hcl hcl

# Functional languages (20)
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-haskell haskell
git clone --depth 1 https://github.com/elixir-lang/tree-sitter-elixir elixir
git clone --depth 1 https://github.com/WhatsApp/tree-sitter-erlang erlang
git clone --depth 1 https://github.com/sogaiu/tree-sitter-clojure clojure
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-ocaml ocaml
git clone --depth 1 https://github.com/ionide/tree-sitter-fsharp fsharp
git clone --depth 1 https://github.com/jordwalke/tree-sitter-reason reason
git clone --depth 1 https://github.com/elm-tooling/tree-sitter-elm elm
git clone --depth 1 https://github.com/purescript-contrib/tree-sitter-purescript purescript
git clone --depth 1 https://github.com/dhall-lang/tree-sitter-dhall dhall
git clone --depth 1 https://github.com/Wilfred/tree-sitter-elisp elisp
git clone --depth 1 https://github.com/alemuller/tree-sitter-make make
git clone --depth 1 https://github.com/theHamsta/tree-sitter-commonlisp commonlisp
git clone --depth 1 https://github.com/6cdh/tree-sitter-racket racket
git clone --depth 1 https://github.com/6cdh/tree-sitter-scheme scheme
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-agda agda
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-julia julia
git clone --depth 1 https://github.com/r-lib/tree-sitter-r r
git clone --depth 1 https://github.com/UserNobody14/tree-sitter-sml sml
git clone --depth 1 https://github.com/ganezdragon/tree-sitter-perl perl

# Systems languages (15)
git clone --depth 1 https://github.com/maxxnino/tree-sitter-zig zig
git clone --depth 1 https://github.com/nilshelmig/tree-sitter-nim nim
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-odin odin
git clone --depth 1 https://github.com/brson/tree-sitter-rust-2 rust2
git clone --depth 1 https://github.com/gdamore/tree-sitter-d d
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-verilog verilog
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-systemverilog systemverilog
git clone --depth 1 https://github.com/alemuller/tree-sitter-vhdl vhdl
git clone --depth 1 https://github.com/stadelmanma/tree-sitter-fortran fortran
git clone --depth 1 https://github.com/briot/tree-sitter-ada ada
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-pascal pascal
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-asm asm
git clone --depth 1 https://github.com/amaanq/tree-sitter-cuda cuda
git clone --depth 1 https://github.com/theHamsta/tree-sitter-glsl glsl
git clone --depth 1 https://github.com/theHamsta/tree-sitter-hlsl hlsl

# Web technologies (15)
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-vue vue
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-svelte svelte
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-astro astro
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-angular angular
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-ember ember
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-scss scss
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-less less
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-sass sass
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-stylus stylus
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-postcss postcss
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-pug pug
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-haml haml
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-handlebars handlebars
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-ejs ejs
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-erb erb

# Scripting languages (10)
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-lua lua
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-powershell powershell
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-vbscript vbscript
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-vim vim
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-tcl tcl
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-awk awk
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-sed sed
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-fish fish
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-nushell nushell
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-zsh zsh

# Configuration languages (10)
git clone --depth 1 https://github.com/cstrahan/tree-sitter-nix nix
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-ansible ansible
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-terraform terraform
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-puppet puppet
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-chef chef
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-saltstack salt
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-helm helm
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-kustomize kustomize
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-jsonnet jsonnet
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-dhall2 dhall2

# Modern languages (15)
git clone --depth 1 https://github.com/gleam-lang/tree-sitter-gleam gleam
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-grain grain
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-roc roc
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-v v
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-mojo mojo
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-carbon carbon
git clone --depth 1 https://github.com/tree-sitter/tree-sitter-dart dart
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-crystal crystal
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-pony pony
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-red red
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-ballerina ballerina
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-chapel chapel
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-hack hack
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-mint mint
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-imba imba

# Blockchain & Smart Contracts (10)
git clone --depth 1 https://github.com/JoranHonig/tree-sitter-solidity solidity
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-vyper vyper
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-yul yul
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-cairo cairo
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-move move
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-cadence cadence
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-clarity clarity
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-teal teal
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-michelson michelson
git clone --depth 1 https://github.com/tree-sitter-grammars/tree-sitter-plutus plutus

echo "✅ Downloaded all 122 tree-sitter grammars!"
