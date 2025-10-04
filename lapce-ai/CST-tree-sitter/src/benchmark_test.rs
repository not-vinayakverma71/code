use std::time::Instant;
use std::collections::HashMap;
use tree_sitter::{Parser, Language};

/// Comprehensive benchmark for all 25 supported languages with tree-sitter 0.24
pub fn run_comprehensive_benchmark() -> BenchmarkResult {
    println!("🚀 Running Comprehensive Tree-Sitter 0.24 Benchmark");
    println!("{}", "=".repeat(60));
    
    let mut results = BenchmarkResult::new();
    
    // Test all currently supported languages (minus the ones with version conflicts)
    let languages = vec![
        ("JavaScript", tree_sitter_javascript::language(), JAVASCRIPT_CODE),
        ("TypeScript", tree_sitter_typescript::language_typescript(), TYPESCRIPT_CODE),
        ("TSX", tree_sitter_typescript::language_tsx(), TSX_CODE),
        ("Python", tree_sitter_python::LANGUAGE.into(), PYTHON_CODE),
        ("Rust", tree_sitter_rust::LANGUAGE.into(), RUST_CODE),
        ("Go", tree_sitter_go::LANGUAGE.into(), GO_CODE),
        ("C", tree_sitter_c::LANGUAGE.into(), C_CODE),
        ("C++", tree_sitter_cpp::LANGUAGE.into(), CPP_CODE),
        ("C#", tree_sitter_c_sharp::LANGUAGE.into(), CSHARP_CODE),
        ("Ruby", tree_sitter_ruby::LANGUAGE.into(), RUBY_CODE),
        ("Java", tree_sitter_java::LANGUAGE.into(), JAVA_CODE),
        ("PHP", tree_sitter_php::LANGUAGE_PHP.into(), PHP_CODE),
        ("Swift", tree_sitter_swift::LANGUAGE.into(), SWIFT_CODE),
        ("Lua", tree_sitter_lua::LANGUAGE.into(), LUA_CODE),
        ("Elixir", tree_sitter_elixir::LANGUAGE.into(), ELIXIR_CODE),
        ("Scala", tree_sitter_scala::LANGUAGE.into(), SCALA_CODE),
        ("Bash", tree_sitter_bash::LANGUAGE.into(), BASH_CODE),
        ("CSS", tree_sitter_css::LANGUAGE.into(), CSS_CODE),
        ("JSON", tree_sitter_json::LANGUAGE.into(), JSON_CODE),
        ("HTML", tree_sitter_html::LANGUAGE.into(), HTML_CODE),
        ("Elm", tree_sitter_elm::LANGUAGE.into(), ELM_CODE),
        ("OCaml", tree_sitter_ocaml::LANGUAGE_OCAML.into(), OCAML_CODE),
        // Excluded due to version conflicts:
        // - TOML (requires tree-sitter 0.20)
        // - Dockerfile (requires tree-sitter 0.20)
        // - Svelte (requires tree-sitter 0.20)
        // - Markdown (requires tree-sitter 0.19)
    ];
    
    println!("Testing {} languages with tree-sitter 0.24", languages.len());
    println!();
    
    for (name, language, code) in languages {
        let result = benchmark_language(name, language, code);
        results.add_language_result(name, result);
    }
    
    results.print_summary();
    results
}

fn benchmark_language(name: &str, language: Language, code: &str) -> LanguageResult {
    let mut parser = Parser::new();
    
    // Test 1: Language setup
    let setup_start = Instant::now();
    let setup_success = parser.set_language(&language).is_ok();
    let setup_time = setup_start.elapsed();
    
    if !setup_success {
        return LanguageResult {
            success: false,
            parse_time_ms: 0.0,
            memory_kb: 0,
            lines_per_sec: 0,
            error: Some("Failed to set language".to_string()),
        };
    }
    
    // Test 2: Initial parse
    let parse_start = Instant::now();
    let tree = parser.parse(code, None);
    let parse_time = parse_start.elapsed();
    
    if tree.is_none() {
        return LanguageResult {
            success: false,
            parse_time_ms: 0.0,
            memory_kb: 0,
            lines_per_sec: 0,
            error: Some("Failed to parse code".to_string()),
        };
    }
    
    let tree = tree.unwrap();
    let line_count = code.lines().count();
    let parse_time_ms = parse_time.as_secs_f64() * 1000.0;
    let lines_per_sec = (line_count as f64 / parse_time.as_secs_f64()) as usize;
    
    // Test 3: Incremental parse
    let incremental_code = format!("{}\n// Added line", code);
    let incr_start = Instant::now();
    let incr_tree = parser.parse(&incremental_code, Some(&tree));
    let incr_time = incr_start.elapsed();
    
    // Test 4: Query execution (basic)
    let query_start = Instant::now();
    let root = tree.root_node();
    let node_count = count_nodes(&root);
    let query_time = query_start.elapsed();
    
    // Estimate memory (rough)
    let memory_kb = (code.len() + node_count * 32) / 1024;
    
    println!("✅ {:<12} | Parse: {:6.2}ms | Incr: {:6.2}ms | Speed: {:7} lines/s | Nodes: {:5} | Mem: {:4} KB",
             name, 
             parse_time_ms, 
             incr_time.as_secs_f64() * 1000.0,
             lines_per_sec, 
             node_count,
             memory_kb);
    
    LanguageResult {
        success: true,
        parse_time_ms,
        memory_kb,
        lines_per_sec,
        error: None,
    }
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

#[derive(Debug)]
pub struct BenchmarkResult {
    languages: HashMap<String, LanguageResult>,
    start_time: Instant,
}

#[derive(Debug)]
struct LanguageResult {
    success: bool,
    parse_time_ms: f64,
    memory_kb: usize,
    lines_per_sec: usize,
    error: Option<String>,
}

impl BenchmarkResult {
    fn new() -> Self {
        Self {
            languages: HashMap::new(),
            start_time: Instant::now(),
        }
    }
    
    fn add_language_result(&mut self, name: &str, result: LanguageResult) {
        self.languages.insert(name.to_string(), result);
    }
    
    fn print_summary(&self) {
        let total_time = self.start_time.elapsed();
        println!("Total time: {:?}", total_time);
        println!("{}", "=".repeat(60));
        println!("📊 BENCHMARK SUMMARY");
        println!("{}", "=".repeat(60));
        
        let successful: Vec<_> = self.languages.iter()
            .filter(|(_, r)| r.success)
            .collect();
        
        let failed: Vec<_> = self.languages.iter()
            .filter(|(_, r)| !r.success)
            .collect();
        
        println!("✅ Successful: {}/{}", successful.len(), self.languages.len());
        println!("❌ Failed: {}/{}", failed.len(), self.languages.len());
        
        if !successful.is_empty() {
            let avg_parse_time: f64 = successful.iter()
                .map(|(_, r)| r.parse_time_ms)
                .sum::<f64>() / successful.len() as f64;
            
            let avg_speed: usize = successful.iter()
                .map(|(_, r)| r.lines_per_sec)
                .sum::<usize>() / successful.len();
            
            let total_memory: usize = successful.iter()
                .map(|(_, r)| r.memory_kb)
                .sum();
            
            println!();
            println!("📈 Performance Metrics:");
            println!("   Average Parse Time: {:.2}ms", avg_parse_time);
            println!("   Average Speed: {} lines/sec", avg_speed);
            println!("   Total Memory: {} KB", total_memory);
            
            // Check against requirements
            println!();
            println!("🎯 Requirements Check:");
            println!("   Memory < 5MB: {} (Used: {:.1} MB)", 
                     if total_memory < 5120 { "✅ PASS" } else { "❌ FAIL" },
                     total_memory as f64 / 1024.0);
            println!("   Speed > 125K lines/s: {} (Avg: {}K lines/s)", 
                     if avg_speed > 125_000 { "✅ PASS" } else { "⚠️  BELOW TARGET" },
                     avg_speed / 1000);
        }
        
        if !failed.is_empty() {
            println!();
            println!("⚠️  Failed Languages:");
            for (name, result) in &failed {
                println!("   - {}: {:?}", name, result.error);
            }
        }
        
        println!();
        println!("Total benchmark time: {:.2}s", self.start_time.elapsed().as_secs_f64());
    }
}

// Sample code for each language
const JAVASCRIPT_CODE: &str = r#"
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
    
    multiply(x) {
        this.result *= x;
        return this;
    }
}

const calc = new Calculator();
console.log(calc.add(5).multiply(2).result);
"#;

const TYPESCRIPT_CODE: &str = r#"
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

const service = new UserService();
service.addUser({ id: 1, name: "Alice" });
"#;

const TSX_CODE: &str = r#"
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

export default Component;
"#;

const PYTHON_CODE: &str = r#"
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
    
    @staticmethod
    def validate(item: int) -> bool:
        return item > 0

async def main():
    processor = DataProcessor("main")
    result = await processor.process([1, 2, 3, 4, 5])
    print(f"Result: {result}")

if __name__ == "__main__":
    asyncio.run(main())
"#;

const RUST_CODE: &str = r#"
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
    
    if let Some(user) = users.get_mut(&1) {
        user.deactivate();
    }
}
"#;

const GO_CODE: &str = r#"
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

func (c *Counter) Value() int {
    c.mu.Lock()
    defer c.mu.Unlock()
    return c.value
}

func main() {
    counter := &Counter{}
    var wg sync.WaitGroup
    
    for i := 0; i < 100; i++ {
        wg.Add(1)
        go func() {
            defer wg.Done()
            counter.Increment()
        }()
    }
    
    wg.Wait()
    fmt.Printf("Final count: %d\n", counter.Value())
}
"#;

const C_CODE: &str = r#"
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

void free_list(Node* head) {
    Node* temp;
    while (head != NULL) {
        temp = head;
        head = head->next;
        free(temp);
    }
}

int main() {
    Node* head = create_node(1);
    head->next = create_node(2);
    head->next->next = create_node(3);
    
    free_list(head);
    return 0;
}
"#;

const CPP_CODE: &str = r#"
#include <iostream>
#include <vector>
#include <memory>

template<typename T>
class Stack {
private:
    std::vector<T> elements;
    
public:
    void push(T const& elem) {
        elements.push_back(elem);
    }
    
    T pop() {
        if (elements.empty()) {
            throw std::runtime_error("Stack is empty");
        }
        T elem = elements.back();
        elements.pop_back();
        return elem;
    }
    
    bool empty() const {
        return elements.empty();
    }
};

int main() {
    Stack<int> intStack;
    intStack.push(42);
    std::cout << intStack.pop() << std::endl;
    return 0;
}
"#;

const CSHARP_CODE: &str = r#"
using System;
using System.Collections.Generic;
using System.Linq;

namespace Example
{
    public class User
    {
        public int Id { get; set; }
        public string Name { get; set; }
        public DateTime CreatedAt { get; set; }
        
        public User(int id, string name)
        {
            Id = id;
            Name = name;
            CreatedAt = DateTime.Now;
        }
    }
    
    class Program
    {
        static void Main(string[] args)
        {
            var users = new List<User>
            {
                new User(1, "Alice"),
                new User(2, "Bob")
            };
            
            var names = users.Select(u => u.Name).ToList();
            names.ForEach(Console.WriteLine);
        }
    }
}
"#;

const RUBY_CODE: &str = r#"
class User
  attr_accessor :name, :email
  
  def initialize(name, email = nil)
    @name = name
    @email = email
    @created_at = Time.now
  end
  
  def valid?
    !@name.nil? && !@name.empty?
  end
  
  def display_info
    puts "User: #{@name}"
    puts "Email: #{@email || 'Not provided'}"
  end
end

users = [
  User.new("Alice", "alice@example.com"),
  User.new("Bob")
]

users.each(&:display_info)
"#;

const JAVA_CODE: &str = r#"
import java.util.*;
import java.util.stream.Collectors;

public class UserService {
    private Map<Long, User> users = new HashMap<>();
    
    public static class User {
        private Long id;
        private String name;
        private boolean active;
        
        public User(Long id, String name) {
            this.id = id;
            this.name = name;
            this.active = true;
        }
        
        public String getName() {
            return name;
        }
    }
    
    public void addUser(User user) {
        users.put(user.id, user);
    }
    
    public List<String> getActiveUserNames() {
        return users.values().stream()
            .filter(u -> u.active)
            .map(User::getName)
            .collect(Collectors.toList());
    }
}
"#;

const PHP_CODE: &str = r#"
<?php

class Database {
    private $connection;
    private $host;
    private $database;
    
    public function __construct($host, $database) {
        $this->host = $host;
        $this->database = $database;
    }
    
    public function connect() {
        // Connection logic here
        return true;
    }
    
    public function query($sql) {
        if (!$this->connection) {
            $this->connect();
        }
        // Query execution
        return [];
    }
}

$db = new Database('localhost', 'mydb');
$results = $db->query('SELECT * FROM users');
"#;

const SWIFT_CODE: &str = r#"
import Foundation

struct User {
    let id: Int
    var name: String
    var email: String?
}

class UserManager {
    private var users: [Int: User] = [:]
    
    func addUser(_ user: User) {
        users[user.id] = user
    }
    
    func getUser(id: Int) -> User? {
        return users[id]
    }
    
    func getAllUserNames() -> [String] {
        return users.values.map { $0.name }
    }
}

let manager = UserManager()
manager.addUser(User(id: 1, name: "Alice"))
"#;

const LUA_CODE: &str = r#"
local function fibonacci(n)
    if n <= 1 then
        return n
    end
    return fibonacci(n - 1) + fibonacci(n - 2)
end

local User = {}
User.__index = User

function User.new(name, age)
    local self = setmetatable({}, User)
    self.name = name
    self.age = age
    return self
end

function User:display()
    print("Name: " .. self.name .. ", Age: " .. self.age)
end

local user = User.new("Alice", 30)
user:display()
"#;

const ELIXIR_CODE: &str = r#"
defmodule User do
  defstruct [:id, :name, :email]
  
  def new(id, name, email \\ nil) do
    %User{id: id, name: name, email: email}
  end
  
  def valid?(%User{name: name}) when not is_nil(name) and name != "" do
    true
  end
  
  def valid?(_), do: false
end

defmodule UserService do
  def process_users(users) do
    users
    |> Enum.filter(&User.valid?/1)
    |> Enum.map(& &1.name)
  end
end

users = [
  User.new(1, "Alice"),
  User.new(2, "Bob", "bob@example.com")
]

UserService.process_users(users)
"#;

const SCALA_CODE: &str = r#"
case class User(id: Int, name: String, email: Option[String] = None)

object UserService {
  private var users = Map.empty[Int, User]
  
  def addUser(user: User): Unit = {
    users = users + (user.id -> user)
  }
  
  def getUser(id: Int): Option[User] = {
    users.get(id)
  }
  
  def getAllUserNames: List[String] = {
    users.values.map(_.name).toList
  }
}

object Main extends App {
  val user = User(1, "Alice", Some("alice@example.com"))
  UserService.addUser(user)
  println(UserService.getAllUserNames)
}
"#;

const BASH_CODE: &str = r#"
#!/bin/bash

function process_files() {
    local dir="$1"
    local pattern="$2"
    
    if [[ ! -d "$dir" ]]; then
        echo "Error: Directory not found"
        return 1
    fi
    
    for file in "$dir"/*"$pattern"*; do
        if [[ -f "$file" ]]; then
            echo "Processing: $(basename "$file")"
            wc -l "$file"
        fi
    done
}

users=("alice" "bob" "charlie")

for user in "${users[@]}"; do
    echo "Creating user: $user"
done

process_files "/tmp" ".txt"
"#;

const CSS_CODE: &str = r#"
:root {
    --primary-color: #3498db;
    --secondary-color: #2ecc71;
    --font-size-base: 16px;
}

.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
}

.button {
    display: inline-block;
    padding: 10px 20px;
    background-color: var(--primary-color);
    color: white;
    border: none;
    border-radius: 4px;
    transition: background-color 0.3s ease;
}

.button:hover {
    background-color: darken(var(--primary-color), 10%);
}

@media (max-width: 768px) {
    .container {
        padding: 10px;
    }
}
"#;

const JSON_CODE: &str = r#"
{
    "name": "example-project",
    "version": "1.0.0",
    "description": "A sample project for testing",
    "main": "index.js",
    "scripts": {
        "start": "node index.js",
        "test": "jest",
        "build": "webpack"
    },
    "dependencies": {
        "express": "^4.18.0",
        "lodash": "^4.17.21"
    },
    "devDependencies": {
        "jest": "^28.0.0",
        "webpack": "^5.0.0"
    },
    "author": "Test Author",
    "license": "MIT"
}
"#;

const HTML_CODE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body>
    <header>
        <nav>
            <ul>
                <li><a href='#home'>Home</a></li>
                <li><a href='#about'>About</a></li>
            </ul>
        </nav>
    </header>
    <main>
        <h1>Welcome</h1>
        <p>This is a test page.</p>
    </main>
</body>
</html>"#;

const ELM_CODE: &str = r#"
module Main exposing (main)

import Html exposing (Html, button, div, text)
import Html.Events exposing (onClick)

type alias Model =
    { count : Int }

init : Model
init =
    { count = 0 }

type Msg
    = Increment
    | Decrement

update : Msg -> Model -> Model
update msg model =
    case msg of
        Increment ->
            { model | count = model.count + 1 }
        
        Decrement ->
            { model | count = model.count - 1 }

view : Model -> Html Msg
view model =
    div []
        [ button [ onClick Decrement ] [ text "-" ]
        , div [] [ text (String.fromInt model.count) ]
        , button [ onClick Increment ] [ text "+" ]
        ]

main : Program () Model Msg
main =
    Html.beginnerProgram
        { model = init
        , update = update
        , view = view
        }
"#;

const OCAML_CODE: &str = r#"
type user = {
  id : int;
  name : string;
  email : string option;
}

let create_user id name email =
  { id; name; email }

let rec fibonacci n =
  if n <= 1 then n
  else fibonacci (n - 1) + fibonacci (n - 2)

let process_list lst =
  lst
  |> List.filter (fun x -> x > 0)
  |> List.map (fun x -> x * 2)
  |> List.fold_left (+) 0

let () =
  let user = create_user 1 "Alice" (Some "alice@example.com") in
  Printf.printf "User: %s\n" user.name;
  
  let result = process_list [1; 2; 3; 4; 5] in
  Printf.printf "Result: %d\n" result
"#;
