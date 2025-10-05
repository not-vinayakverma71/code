use std::time::Instant;
use std::collections::HashMap;
use tree_sitter::{Parser, Language};

fn main() {
    println!("🚀 Tree-Sitter 0.24 Comprehensive Benchmark for 22 Languages");
    println!("{}", "=".repeat(70));
    
    let mut total_success = 0;
    let mut total_failed = 0;
    let mut total_parse_time = 0.0;
    let mut total_memory = 0;
    let mut total_speed = 0;
    
    // Test all currently supported languages (22 working, excluding version conflicts)
    let tests = vec![
        ("JavaScript", get_js_test()),
        ("TypeScript", get_ts_test()),
        ("TSX", get_tsx_test()),
        ("Python", get_python_test()),
        ("Rust", get_rust_test()),
        ("Go", get_go_test()),
        ("C", get_c_test()),
        ("C++", get_cpp_test()),
        ("C#", get_csharp_test()),
        ("Ruby", get_ruby_test()),
        ("Java", get_java_test()),
        ("PHP", get_php_test()),
        ("Swift", get_swift_test()),
        ("Lua", get_lua_test()),
        ("Elixir", get_elixir_test()),
        ("Scala", get_scala_test()),
        ("Bash", get_bash_test()),
        ("CSS", get_css_test()),
        ("JSON", get_json_test()),
        ("HTML", get_html_test()),
        ("Elm", get_elm_test()),
        ("OCaml", get_ocaml_test()),
    ];
    
    println!("Testing {} languages with tree-sitter 0.24\n", tests.len());
    
    for (name, (language, code)) in tests {
        let result = benchmark_language(name, language, &code);
        if result.0 {
            total_success += 1;
            total_parse_time += result.1;
            total_memory += result.2;
            total_speed += result.3;
        } else {
            total_failed += 1;
        }
    }
    
    println!("\n{}", "=".repeat(70));
    println!("📊 BENCHMARK SUMMARY");
    println!("{}", "=".repeat(70));
    
    println!("✅ Successful: {}/{}", total_success, total_success + total_failed);
    println!("❌ Failed: {}/{}", total_failed, total_success + total_failed);
    
    if total_success > 0 {
        let avg_parse_time = total_parse_time / total_success as f64;
        let avg_speed = total_speed / total_success;
        
        println!("\n📈 Performance Metrics:");
        println!("   Average Parse Time: {:.2}ms", avg_parse_time);
        println!("   Average Speed: {} lines/sec", avg_speed);
        println!("   Total Memory: {} KB ({:.2} MB)", total_memory, total_memory as f64 / 1024.0);
        
        println!("\n🎯 Requirements Check:");
        println!("   Memory < 5MB: {} (Used: {:.2} MB)", 
                 if total_memory < 5120 { "✅ PASS" } else { "❌ FAIL" },
                 total_memory as f64 / 1024.0);
        println!("   Speed > 125K lines/s: {} (Avg: {}K lines/s)", 
                 if avg_speed > 125_000 { "✅ PASS" } else { "⚠️  BELOW TARGET" },
                 avg_speed / 1000);
    }
}

fn benchmark_language(name: &str, language: Language, code: &str) -> (bool, f64, usize, usize) {
    let mut parser = Parser::new();
    
    // Test language setup
    if parser.set_language(&language).is_err() {
        println!("❌ {:<12} | Failed to set language", name);
        return (false, 0.0, 0, 0);
    }
    
    // Test initial parse
    let parse_start = Instant::now();
    let tree = match parser.parse(code, None) {
        Some(t) => t,
        None => {
            println!("❌ {:<12} | Failed to parse code", name);
            return (false, 0.0, 0, 0);
        }
    };
    let parse_time = parse_start.elapsed();
    
    let line_count = code.lines().count();
    let parse_time_ms = parse_time.as_secs_f64() * 1000.0;
    let lines_per_sec = if parse_time.as_secs_f64() > 0.0 {
        (line_count as f64 / parse_time.as_secs_f64()) as usize
    } else {
        line_count * 1000
    };
    
    // Test incremental parse
    let incremental_code = format!("{}\n// Added line", code);
    let incr_start = Instant::now();
    let _incr_tree = parser.parse(&incremental_code, Some(&tree));
    let incr_time = incr_start.elapsed();
    
    // Count nodes
    let root = tree.root_node();
    let node_count = count_nodes(&root);
    
    // Estimate memory
    let memory_kb = (code.len() + node_count * 32) / 1024;
    
    println!("✅ {:<12} | Parse: {:6.2}ms | Incr: {:6.2}ms | Speed: {:7} lines/s | Nodes: {:5} | Mem: {:4} KB",
             name, 
             parse_time_ms, 
             incr_time.as_secs_f64() * 1000.0,
             lines_per_sec, 
             node_count,
             memory_kb);
    
    (true, parse_time_ms, memory_kb, lines_per_sec)
}

fn count_nodes(node: &tree_sitter::Node) -> usize {
    let mut count = 1;
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            count += count_nodes(&child);
        }
    }
    count
}

// Language tests with sample code
fn get_js_test() -> (Language, String) {
    (tree_sitter_javascript::language().into(), r#"
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

class Calculator {
    constructor() {
        this.result = 0;
    }
    
    add(x) {
        this.result += x;
        return this;
    }
}

const calc = new Calculator();
console.log(calc.add(5).result);
"#.to_string())
}

fn get_ts_test() -> (Language, String) {
    (tree_sitter_typescript::language_typescript().into(), r#"
interface User {
    id: number;
    name: string;
    email?: string;
}

class UserService {
    private users: Map<number, User> = new Map();
    
    addUser(user: User): void {
        this.users.set(user.id, user);
    }
    
    getUser(id: number): User | undefined {
        return this.users.get(id);
    }
}
"#.to_string())
}

fn get_tsx_test() -> (Language, String) {
    (tree_sitter_typescript::language_tsx().into(), r#"
import React from 'react';

interface Props {
    title: string;
    count: number;
}

const Component: React.FC<Props> = ({ title, count }) => {
    return (
        <div className="container">
            <h1>{title}</h1>
            <span>Count: {count}</span>
        </div>
    );
};
"#.to_string())
}

fn get_python_test() -> (Language, String) {
    (tree_sitter_python::LANGUAGE.into(), r#"
import asyncio
from typing import List, Optional

class DataProcessor:
    def __init__(self, name: str):
        self.name = name
        self.data: List[int] = []
    
    async def process(self, items: List[int]) -> Optional[int]:
        await asyncio.sleep(0.1)
        self.data.extend(items)
        return sum(self.data) if self.data else None

async def main():
    processor = DataProcessor("main")
    result = await processor.process([1, 2, 3, 4, 5])
    print(f"Result: {result}")
"#.to_string())
}

fn get_rust_test() -> (Language, String) {
    (tree_sitter_rust::LANGUAGE.into(), r#"
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct User {
    id: u64,
    name: String,
    active: bool,
}

impl User {
    fn new(id: u64, name: String) -> Self {
        Self { id, name, active: true }
    }
    
    fn deactivate(&mut self) {
        self.active = false;
    }
}

fn main() {
    let mut users: HashMap<u64, User> = HashMap::new();
    users.insert(1, User::new(1, "Alice".to_string()));
}
"#.to_string())
}

fn get_go_test() -> (Language, String) {
    (tree_sitter_go::LANGUAGE.into(), r#"
package main

import (
    "fmt"
    "sync"
)

type Counter struct {
    mu    sync.Mutex
    value int
}

func (c *Counter) Increment() {
    c.mu.Lock()
    defer c.mu.Unlock()
    c.value++
}

func main() {
    counter := &Counter{}
    fmt.Printf("Count: %d\n", counter.value)
}
"#.to_string())
}

fn get_c_test() -> (Language, String) {
    (tree_sitter_c::LANGUAGE.into(), r#"
#include <stdio.h>
#include <stdlib.h>

typedef struct Node {
    int data;
    struct Node* next;
} Node;

Node* create_node(int data) {
    Node* node = (Node*)malloc(sizeof(Node));
    node->data = data;
    node->next = NULL;
    return node;
}

int main() {
    Node* head = create_node(1);
    free(head);
    return 0;
}
"#.to_string())
}

fn get_cpp_test() -> (Language, String) {
    (tree_sitter_cpp::LANGUAGE.into(), r#"
#include <iostream>
#include <vector>

template<typename T>
class Stack {
private:
    std::vector<T> elements;
    
public:
    void push(T const& elem) {
        elements.push_back(elem);
    }
    
    bool empty() const {
        return elements.empty();
    }
};

int main() {
    Stack<int> intStack;
    intStack.push(42);
    return 0;
}
"#.to_string())
}

fn get_csharp_test() -> (Language, String) {
    (tree_sitter_c_sharp::LANGUAGE.into(), r#"
using System;
using System.Collections.Generic;

namespace Example
{
    public class User
    {
        public int Id { get; set; }
        public string Name { get; set; }
        
        public User(int id, string name)
        {
            Id = id;
            Name = name;
        }
    }
}
"#.to_string())
}

fn get_ruby_test() -> (Language, String) {
    (tree_sitter_ruby::LANGUAGE.into(), r#"
class User
  attr_accessor :name, :email
  
  def initialize(name, email = nil)
    @name = name
    @email = email
  end
  
  def valid?
    !@name.nil? && !@name.empty?
  end
end

users = [User.new("Alice")]
"#.to_string())
}

fn get_java_test() -> (Language, String) {
    (tree_sitter_java::LANGUAGE.into(), r#"
import java.util.*;

public class UserService {
    private Map<Long, User> users = new HashMap<>();
    
    public static class User {
        private Long id;
        private String name;
        
        public User(Long id, String name) {
            this.id = id;
            this.name = name;
        }
    }
}
"#.to_string())
}

fn get_php_test() -> (Language, String) {
    (tree_sitter_php::LANGUAGE_PHP.into(), r#"
<?php

class Database {
    private $connection;
    
    public function __construct($host) {
        $this->host = $host;
    }
    
    public function query($sql) {
        return [];
    }
}

$db = new Database('localhost');
"#.to_string())
}

fn get_swift_test() -> (Language, String) {
    (tree_sitter_swift::LANGUAGE.into(), r#"
import Foundation

struct User {
    let id: Int
    var name: String
}

class UserManager {
    private var users: [Int: User] = [:]
    
    func addUser(_ user: User) {
        users[user.id] = user
    }
}
"#.to_string())
}

fn get_lua_test() -> (Language, String) {
    (tree_sitter_lua::LANGUAGE.into(), r#"
local function fibonacci(n)
    if n <= 1 then
        return n
    end
    return fibonacci(n - 1) + fibonacci(n - 2)
end

local result = fibonacci(10)
print("Result: " .. result)
"#.to_string())
}

fn get_elixir_test() -> (Language, String) {
    (tree_sitter_elixir::LANGUAGE.into(), r#"
defmodule User do
  defstruct [:id, :name, :email]
  
  def new(id, name) do
    %User{id: id, name: name}
  end
end

users = [User.new(1, "Alice")]
"#.to_string())
}

fn get_scala_test() -> (Language, String) {
    (tree_sitter_scala::LANGUAGE.into(), r#"
case class User(id: Int, name: String)

object UserService {
  def getUser(id: Int): Option[User] = {
    Some(User(id, "Test"))
  }
}
"#.to_string())
}

fn get_bash_test() -> (Language, String) {
    (tree_sitter_bash::LANGUAGE.into(), r#"
#!/bin/bash

function process_files() {
    local dir="$1"
    
    for file in "$dir"/*; do
        echo "Processing: $file"
    done
}

process_files "/tmp"
"#.to_string())
}

fn get_css_test() -> (Language, String) {
    (tree_sitter_css::LANGUAGE.into(), r#"
.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
}

.button {
    background-color: #3498db;
    color: white;
    border: none;
    padding: 10px 20px;
}
"#.to_string())
}

fn get_json_test() -> (Language, String) {
    (tree_sitter_json::LANGUAGE.into(), r#"
{
    "name": "example",
    "version": "1.0.0",
    "dependencies": {
        "express": "^4.18.0"
    }
}
"#.to_string())
}

fn get_html_test() -> (Language, String) {
    (tree_sitter_html::LANGUAGE.into(), r#"
<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body>
    <h1>Hello World</h1>
    <p>This is a test.</p>
</body>
</html>
"#.to_string())
}

fn get_elm_test() -> (Language, String) {
    (tree_sitter_elm::LANGUAGE().into(), r#"
module Main exposing (main)

import Html exposing (text)

main =
    text "Hello, World!"
"#.to_string())
}

fn get_ocaml_test() -> (Language, String) {
    (tree_sitter_ocaml::LANGUAGE_OCAML.into(), r#"
type user = {
  id : int;
  name : string;
}

let create_user id name =
  { id; name }

let () =
  let user = create_user 1 "Alice" in
  Printf.printf "User: %s\n" user.name
"#.to_string())
}
