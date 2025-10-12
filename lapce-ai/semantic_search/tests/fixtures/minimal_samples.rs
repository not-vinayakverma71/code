// This file contains minimal valid code samples for all 67 languages
// Used for testing that each language can be parsed successfully

use std::collections::HashMap;

pub fn get_minimal_samples() -> HashMap<&'static str, &'static str> {
    let mut samples = HashMap::new();
    
    // Core languages (31)
    samples.insert("rust", "fn main() { println!(\"Hello\"); }");
    samples.insert("python", "def hello():\n    print('Hello')");
    samples.insert("go", "package main\n\nfunc main() {}");
    samples.insert("java", "public class Test {\n    public static void main(String[] args) {}\n}");
    samples.insert("c", "int main() { return 0; }");
    samples.insert("cpp", "#include <iostream>\nint main() { return 0; }");
    samples.insert("c_sharp", "class Program { static void Main() {} }");
    samples.insert("ruby", "puts 'Hello'");
    samples.insert("php", "<?php echo 'Hello'; ?>");
    samples.insert("lua", "print('Hello')");
    samples.insert("bash", "#!/bin/bash\necho 'Hello'");
    samples.insert("css", "body { color: red; }");
    samples.insert("json", "{ \"key\": \"value\" }");
    samples.insert("swift", "print(\"Hello\")");
    samples.insert("scala", "object Main extends App { println(\"Hello\") }");
    samples.insert("elixir", "IO.puts \"Hello\"");
    samples.insert("html", "<!DOCTYPE html>\n<html><body></body></html>");
    samples.insert("ocaml", "let () = print_endline \"Hello\"");
    samples.insert("nix", "{ pkgs ? import <nixpkgs> {} }: pkgs.hello");
    samples.insert("make", "all:\n\t@echo 'Hello'");
    samples.insert("cmake", "cmake_minimum_required(VERSION 3.0)\nproject(Test)");
    samples.insert("verilog", "module test; endmodule");
    samples.insert("erlang", "-module(test).\n-export([hello/0]).\nhello() -> io:format(\"Hello\").");
    samples.insert("d", "void main() { import std.stdio; writeln(\"Hello\"); }");
    samples.insert("pascal", "program Hello; begin writeln('Hello'); end.");
    samples.insert("commonlisp", "(print \"Hello\")");
    samples.insert("objc", "#import <Foundation/Foundation.h>\nint main() { return 0; }");
    samples.insert("groovy", "println 'Hello'");
    samples.insert("embedded_template", "<%= 'Hello' %>");
    samples.insert("javascript", "console.log('Hello');");
    samples.insert("typescript", "const message: string = 'Hello';");
    
    // External grammar languages (36)
    samples.insert("toml", "title = \"Test\"");
    samples.insert("dockerfile", "FROM alpine\nRUN echo hello");
    samples.insert("elm", "module Main exposing (..)\nmain = text \"Hello\"");
    samples.insert("kotlin", "fun main() { println(\"Hello\") }");
    samples.insert("yaml", "key: value");
    samples.insert("r", "print(\"Hello\")");
    samples.insert("matlab", "disp('Hello')");
    samples.insert("perl", "print \"Hello\\n\";");
    samples.insert("dart", "void main() { print('Hello'); }");
    samples.insert("julia", "println(\"Hello\")");
    samples.insert("haskell", "main = putStrLn \"Hello\"");
    samples.insert("graphql", "type Query { hello: String }");
    samples.insert("sql", "SELECT * FROM users;");
    samples.insert("zig", "pub fn main() void {}");
    samples.insert("vim", "echo 'Hello'");
    samples.insert("abap", "WRITE 'Hello'.");
    samples.insert("nim", "echo \"Hello\"");
    samples.insert("clojure", "(println \"Hello\")");
    samples.insert("crystal", "puts \"Hello\"");
    samples.insert("fortran", "program hello\n  print *, 'Hello'\nend program");
    samples.insert("vhdl", "entity test is\nend test;");
    samples.insert("racket", "#lang racket\n(displayln \"Hello\")");
    samples.insert("ada", "with Ada.Text_IO; use Ada.Text_IO;\nprocedure Hello is\nbegin\n  Put_Line(\"Hello\");\nend Hello;");
    samples.insert("prolog", "hello :- write('Hello').");
    samples.insert("gradle", "task hello { doLast { println 'Hello' } }");
    samples.insert("xml", "<?xml version=\"1.0\"?>\n<root></root>");
    samples.insert("markdown", "# Hello\n\nWorld");
    samples.insert("svelte", "<script>\n  let name = 'world';\n</script>\n<h1>Hello {name}!</h1>");
    samples.insert("scheme", "(display \"Hello\")");
    samples.insert("fennel", "(print \"Hello\")");
    samples.insert("gleam", "pub fn main() { io.println(\"Hello\") }");
    samples.insert("hcl", "resource \"test\" \"example\" {}");
    samples.insert("solidity", "pragma solidity ^0.8.0;\ncontract Test {}");
    samples.insert("fsharp", "printfn \"Hello\"");
    samples.insert("cobol", "       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELLO.\n       PROCEDURE DIVISION.\n           DISPLAY 'Hello'.\n           STOP RUN.");
    samples.insert("systemverilog", "module test; endmodule");
    
    samples
}

pub fn get_file_extension(language: &str) -> &'static str {
    match language {
        "rust" => "rs",
        "python" => "py",
        "go" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" => "cpp",
        "c_sharp" => "cs",
        "ruby" => "rb",
        "php" => "php",
        "lua" => "lua",
        "bash" => "sh",
        "css" => "css",
        "json" => "json",
        "swift" => "swift",
        "scala" => "scala",
        "elixir" => "ex",
        "html" => "html",
        "ocaml" => "ml",
        "nix" => "nix",
        "make" => "mk",
        "cmake" => "cmake",
        "verilog" => "v",
        "erlang" => "erl",
        "d" => "d",
        "pascal" => "pas",
        "commonlisp" => "lisp",
        "objc" => "mm",
        "groovy" => "groovy",
        "embedded_template" => "erb",
        "javascript" => "js",
        "typescript" => "ts",
        "toml" => "toml",
        "dockerfile" => "dockerfile",
        "elm" => "elm",
        "kotlin" => "kt",
        "yaml" => "yaml",
        "r" => "r",
        "matlab" => "m",
        "perl" => "pl",
        "dart" => "dart",
        "julia" => "jl",
        "haskell" => "hs",
        "graphql" => "graphql",
        "sql" => "sql",
        "zig" => "zig",
        "vim" => "vim",
        "abap" => "abap",
        "nim" => "nim",
        "clojure" => "clj",
        "crystal" => "cr",
        "fortran" => "f90",
        "vhdl" => "vhd",
        "racket" => "rkt",
        "ada" => "adb",
        "prolog" => "pl",
        "gradle" => "gradle",
        "xml" => "xml",
        "markdown" => "md",
        "svelte" => "svelte",
        "scheme" => "scm",
        "fennel" => "fnl",
        "gleam" => "gleam",
        "hcl" => "hcl",
        "solidity" => "sol",
        "fsharp" => "fs",
        "cobol" => "cob",
        "systemverilog" => "sv",
        _ => "txt"
    }
}
