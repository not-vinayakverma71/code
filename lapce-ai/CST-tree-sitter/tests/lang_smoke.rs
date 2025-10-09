//! Smoke tests for all 67 supported languages
//! Each test parses production-grade valid snippets for that language
//! External grammars are feature-gated

#[cfg(test)]
mod smoke_tests {
    use tree_sitter::Parser;
    
    // Macro for standard languages with LANGUAGE constant
    macro_rules! smoke_test {
        ($name:ident, $module:ident, $code:expr) => {
            #[test]
            fn $name() {
                let mut parser = Parser::new();
                parser.set_language(&$module::LANGUAGE.into())
                    .expect(concat!("Failed to load ", stringify!($name)));
                
                let tree = parser.parse($code, None)
                    .expect(concat!("Failed to parse ", stringify!($name)));
                    
                assert!(!tree.root_node().has_error(), 
                    concat!(stringify!($name), " parse has errors"));
            }
        };
    }
    
    // Macro for languages with language() function
    macro_rules! smoke_test_fn {
        ($name:ident, $module:ident, $code:expr) => {
            #[test]
            fn $name() {
                let mut parser = Parser::new();
                parser.set_language(&$module::language())
                    .expect(concat!("Failed to load ", stringify!($name)));
                
                let tree = parser.parse($code, None)
                    .expect(concat!("Failed to parse ", stringify!($name)));
                    
                assert!(!tree.root_node().has_error(), 
                    concat!(stringify!($name), " parse has errors"));
            }
        };
    }
    
    // Macro for feature-gated languages with language() function
    macro_rules! smoke_test_fn_gated {
        ($name:ident, $feature:literal, $module:ident, $code:expr) => {
            #[test]
            #[cfg(feature = $feature)]
            fn $name() {
                let mut parser = Parser::new();
                parser.set_language(&$module::language())
                    .expect(concat!("Failed to load ", stringify!($name)));
                
                let tree = parser.parse($code, None)
                    .expect(concat!("Failed to parse ", stringify!($name)));
                    
                assert!(!tree.root_node().has_error(), 
                    concat!(stringify!($name), " parse has errors"));
            }
        };
    }
    
    // ========== TIER 1: Core Languages (31 languages) ==========
    
    // 1-8: Fundamental Languages
    smoke_test!(test_rust, tree_sitter_rust, 
        "fn main() {\n    println!(\"Hello, world!\");\n    let numbers = vec![1, 2, 3];\n    let sum: i32 = numbers.iter().sum();\n}");
    
    smoke_test!(test_python, tree_sitter_python, 
        "def factorial(n: int) -> int:\n    \"\"\"Calculate factorial recursively.\"\"\"\n    if n <= 1:\n        return 1\n    return n * factorial(n - 1)\n\nif __name__ == \"__main__\":\n    print(factorial(5))");
    
    smoke_test!(test_go, tree_sitter_go, 
        "package main\n\nimport (\n    \"fmt\"\n    \"sync\"\n)\n\nfunc main() {\n    var wg sync.WaitGroup\n    wg.Add(1)\n    go func() {\n        defer wg.Done()\n        fmt.Println(\"Hello from goroutine\")\n    }()\n    wg.Wait()\n}");
    
    smoke_test!(test_java, tree_sitter_java, 
        "public class Calculator {\n    private double result;\n    \n    public Calculator() {\n        this.result = 0;\n    }\n    \n    public double add(double value) {\n        result += value;\n        return result;\n    }\n    \n    public static void main(String[] args) {\n        Calculator calc = new Calculator();\n        System.out.println(calc.add(5));\n    }\n}");
    
    smoke_test!(test_c, tree_sitter_c, 
        "#include <stdio.h>\n#include <stdlib.h>\n\ntypedef struct {\n    int x;\n    int y;\n} Point;\n\nint main(void) {\n    Point *p = malloc(sizeof(Point));\n    p->x = 10;\n    p->y = 20;\n    printf(\"Point: (%d, %d)\\n\", p->x, p->y);\n    free(p);\n    return 0;\n}");
    
    smoke_test!(test_cpp, tree_sitter_cpp, 
        "#include <iostream>\n#include <vector>\n#include <algorithm>\n\ntemplate<typename T>\nclass Stack {\nprivate:\n    std::vector<T> data;\npublic:\n    void push(const T& item) {\n        data.push_back(item);\n    }\n    \n    T pop() {\n        T item = data.back();\n        data.pop_back();\n        return item;\n    }\n};\n\nint main() {\n    Stack<int> stack;\n    stack.push(42);\n    return 0;\n}");
    
    smoke_test!(test_csharp, tree_sitter_c_sharp, 
        "using System;\nusing System.Collections.Generic;\nusing System.Linq;\n\nnamespace MyApp\n{\n    public class Program\n    {\n        public static void Main(string[] args)\n        {\n            var numbers = new List<int> { 1, 2, 3, 4, 5 };\n            var evens = numbers.Where(n => n % 2 == 0);\n            foreach (var num in evens)\n            {\n                Console.WriteLine(num);\n            }\n        }\n    }\n}");
    
    smoke_test!(test_ruby, tree_sitter_ruby, 
        "class Person\n  attr_accessor :name, :age\n  \n  def initialize(name, age)\n    @name = name\n    @age = age\n  end\n  \n  def greet\n    puts \"Hello, my name is #{@name}\"\n  end\n  \n  def self.create_anonymous\n    new(\"Anonymous\", 0)\n  end\nend\n\nperson = Person.new(\"Alice\", 30)\nperson.greet");
    
    // 9. PHP - uses language_php() in 0.23.x
    #[test]
    fn test_php() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_php::language_php())
            .expect("Failed to load PHP");
        
        let code = "<?php\n\nnamespace App\\Models;\n\nclass User {\n    private string $name;\n    private int $age;\n    \n    public function __construct(string $name, int $age) {\n        $this->name = $name;\n        $this->age = $age;\n    }\n    \n    public function getName(): string {\n        return $this->name;\n    }\n}\n\n$user = new User(\"Alice\", 30);\necho $user->getName();\n?>";
        let tree = parser.parse(code, None)
            .expect("Failed to parse PHP");
            
        assert!(!tree.root_node().has_error(), "PHP parse has errors");
    }
    
    // 10-13: Scripting and Web Languages
    smoke_test!(test_lua, tree_sitter_lua, 
        "-- Fibonacci function\nlocal function fibonacci(n)\n    if n <= 1 then\n        return n\n    end\n    return fibonacci(n - 1) + fibonacci(n - 2)\nend\n\n-- Table operations\nlocal fruits = {\"apple\", \"banana\", \"orange\"}\ntable.insert(fruits, \"grape\")\n\nfor i, fruit in ipairs(fruits) do\n    print(i, fruit)\nend");
    
    smoke_test!(test_bash, tree_sitter_bash, 
        "#!/bin/bash\n\n# Function definition\nfunction process_files() {\n    local dir=\"$1\"\n    \n    for file in \"$dir\"/*.txt; do\n        if [ -f \"$file\" ]; then\n            echo \"Processing: $(basename \"$file\")\"\n            wc -l \"$file\"\n        fi\n    done\n}\n\n# Array handling\ndeclare -a colors=(\"red\" \"green\" \"blue\")\n\nfor color in \"${colors[@]}\"; do\n    echo \"Color: $color\"\ndone");
    
    smoke_test!(test_css, tree_sitter_css, 
        ":root {\n    --primary-color: #007bff;\n    --secondary-color: #6c757d;\n}\n\n.container {\n    display: grid;\n    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));\n    gap: 1rem;\n    padding: 2rem;\n}\n\n.card {\n    background: white;\n    border-radius: 8px;\n    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);\n    transition: transform 0.3s ease;\n}\n\n.card:hover {\n    transform: translateY(-4px);\n}\n\n@media (max-width: 768px) {\n    .container {\n        grid-template-columns: 1fr;\n    }\n}");
    
    smoke_test!(test_json, tree_sitter_json, 
        "{\n  \"name\": \"my-project\",\n  \"version\": \"1.0.0\",\n  \"description\": \"A sample project\",\n  \"main\": \"index.js\",\n  \"scripts\": {\n    \"start\": \"node index.js\",\n    \"test\": \"jest\",\n    \"build\": \"webpack\"\n  },\n  \"dependencies\": {\n    \"express\": \"^4.18.0\",\n    \"lodash\": \"^4.17.21\"\n  },\n  \"devDependencies\": {\n    \"jest\": \"^29.0.0\",\n    \"webpack\": \"^5.74.0\"\n  },\n  \"author\": \"John Doe\",\n  \"license\": \"MIT\"\n}");
    
    // 14. TOML - uses language() function (feature-gated external grammar)
    #[cfg(feature = "lang-toml")]
    #[test]
    fn test_toml() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_toml::language())
            .expect("Failed to load TOML");
        
        let code = "[package]\nname = \"my-app\"\nversion = \"0.1.0\"\nauthors = [\"John Doe <john@example.com>\"]\nedition = \"2021\"\n\n[dependencies]\nserde = { version = \"1.0\", features = [\"derive\"] }\ntokio = { version = \"1.0\", features = [\"full\"] }\n\n[dev-dependencies]\ncriterion = \"0.5\"\n\n[[bin]]\nname = \"server\"\npath = \"src/bin/server.rs\"\n\n[profile.release]\nopt-level = 3\nlto = true";
        let tree = parser.parse(code, None)
            .expect("Failed to parse TOML");
            
        assert!(!tree.root_node().has_error(), "TOML parse has errors");
    }
    
    // 15-18: Modern Languages
    smoke_test!(test_swift, tree_sitter_swift, 
        "import Foundation\n\nstruct Person: Codable {\n    let name: String\n    var age: Int\n    \n    mutating func haveBirthday() {\n        age += 1\n    }\n}\n\nclass ViewController: UIViewController {\n    @IBOutlet weak var label: UILabel!\n    \n    override func viewDidLoad() {\n        super.viewDidLoad()\n        label.text = \"Hello, Swift!\"\n    }\n}\n\nlet numbers = [1, 2, 3, 4, 5]\nlet doubled = numbers.map { $0 * 2 }");
    
    smoke_test!(test_scala, tree_sitter_scala, 
        "package com.example\n\nimport scala.concurrent.Future\nimport scala.concurrent.ExecutionContext.Implicits.global\n\nobject HelloWorld {\n  def main(args: Array[String]): Unit = {\n    val numbers = List(1, 2, 3, 4, 5)\n    val doubled = numbers.map(_ * 2)\n    \n    val future = Future {\n      Thread.sleep(1000)\n      42\n    }\n    \n    future.foreach(println)\n  }\n  \n  case class Person(name: String, age: Int)\n}");
    
    smoke_test!(test_elixir, tree_sitter_elixir, 
        "defmodule Calculator do\n  @moduledoc \"\"\"\n  A simple calculator module\n  \"\"\"\n  \n  def add(a, b) when is_number(a) and is_number(b) do\n    a + b\n  end\n  \n  def factorial(0), do: 1\n  def factorial(n) when n > 0 do\n    n * factorial(n - 1)\n  end\n  \n  def async_operation do\n    Task.async(fn ->\n      Process.sleep(1000)\n      {:ok, \"completed\"}\n    end)\n  end\nend");
    
    smoke_test!(test_html, tree_sitter_html, 
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"UTF-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n    <title>Sample Page</title>\n    <style>\n        body { font-family: sans-serif; }\n    </style>\n</head>\n<body>\n    <header>\n        <nav>\n            <ul>\n                <li><a href=\"#home\">Home</a></li>\n                <li><a href=\"#about\">About</a></li>\n            </ul>\n        </nav>\n    </header>\n    <main>\n        <article>\n            <h1>Welcome</h1>\n            <p>This is a sample HTML document.</p>\n        </article>\n    </main>\n    <script>\n        console.log('Page loaded');\n    </script>\n</body>\n</html>");
    
    // 19. OCaml - uses LANGUAGE_OCAML constant
    #[test]
    fn test_ocaml() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_ocaml::LANGUAGE_OCAML.into())
            .expect("Failed to load OCaml");
        
        let code = "module StringMap = Map.Make(String)\n\ntype person = {\n  name : string;\n  age : int;\n}\n\nlet rec factorial n =\n  match n with\n  | 0 | 1 -> 1\n  | n -> n * factorial (n - 1)\n\nlet main () =\n  let person = { name = \"Alice\"; age = 30 } in\n  Printf.printf \"Hello, %s!\\n\" person.name;\n  List.iter (fun x -> print_int x; print_newline ())\n    [1; 2; 3; 4; 5]";
        let tree = parser.parse(code, None)
            .expect("Failed to parse OCaml");
            
        assert!(!tree.root_node().has_error(), "OCaml parse has errors");
    }
    
    // 20-25: Build and System Languages
    smoke_test!(test_nix, tree_sitter_nix, 
        "{ pkgs ? import <nixpkgs> {}, lib ? pkgs.lib }:\n\nlet\n  version = \"1.0.0\";\n  pname = \"my-app\";\nin\npkgs.stdenv.mkDerivation rec {\n  inherit pname version;\n  \n  src = ./.;\n  \n  buildInputs = with pkgs; [\n    rustc\n    cargo\n    openssl\n  ];\n  \n  buildPhase = ''\n    cargo build --release\n  '';\n  \n  installPhase = ''\n    mkdir -p $out/bin\n    cp target/release/${pname} $out/bin/\n  '';\n}");
    
    smoke_test!(test_make, tree_sitter_make, 
        "all:\n\techo hello\n\nclean:\n\trm -f *.o\n");
    
    smoke_test!(test_cmake, tree_sitter_cmake, 
        "project(MyProject)");
    
    smoke_test!(test_verilog, tree_sitter_verilog, 
        "module counter #(\n    parameter WIDTH = 8\n) (\n    input wire clk,\n    input wire rst_n,\n    input wire enable,\n    output reg [WIDTH-1:0] count,\n    output wire overflow\n);\n\n    always @(posedge clk or negedge rst_n) begin\n        if (!rst_n) begin\n            count <= {WIDTH{1'b0}};\n        end else if (enable) begin\n            count <= count + 1'b1;\n        end\n    end\n    \n    assign overflow = (count == {WIDTH{1'b1}});\n    \nendmodule");
    
    smoke_test!(test_erlang, tree_sitter_erlang, 
        "-module(gen_server_example).\n-behaviour(gen_server).\n\n-export([start_link/0, stop/0]).\n-export([init/1, handle_call/3, handle_cast/2, handle_info/2, terminate/2]).\n\n-record(state, {count = 0}).\n\nstart_link() ->\n    gen_server:start_link({local, ?MODULE}, ?MODULE, [], []).\n\nstop() ->\n    gen_server:cast(?MODULE, stop).\n\ninit([]) ->\n    {ok, #state{}}.\n\nhandle_call(get_count, _From, State = #state{count = Count}) ->\n    {reply, Count, State};\nhandle_call(_Request, _From, State) ->\n    {reply, ok, State}.\n\nhandle_cast(increment, State = #state{count = Count}) ->\n    {noreply, State#state{count = Count + 1}};\nhandle_cast(stop, State) ->\n    {stop, normal, State}.\n\nhandle_info(_Info, State) ->\n    {noreply, State}.\n\nterminate(_Reason, _State) ->\n    ok.");
    
    smoke_test!(test_d, tree_sitter_d, 
        "import std.stdio;\nimport std.algorithm;\nimport std.range;\n\nclass Person {\n    private string name;\n    private int age;\n    \n    this(string name, int age) {\n        this.name = name;\n        this.age = age;\n    }\n    \n    @property string getName() const {\n        return name;\n    }\n}\n\nvoid main() {\n    auto numbers = [1, 2, 3, 4, 5];\n    auto doubled = numbers.map!(x => x * 2).array;\n    \n    foreach (num; doubled) {\n        writeln(num);\n    }\n}");
    
    // 26. Dockerfile - uses language() function (feature-gated external grammar)
    #[cfg(feature = "lang-dockerfile")]
    #[test]
    fn test_dockerfile() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_dockerfile::language())
            .expect("Failed to load Dockerfile");
        
        // Most minimal valid Dockerfile with newline
        let code = "FROM ubuntu\n";
        let tree = parser.parse(code, None)
            .expect("Failed to parse Dockerfile");
            
        assert!(!tree.root_node().has_error(), "Dockerfile parse has errors");
    }
    
    // 27-31: Various Languages
    smoke_test!(test_pascal, tree_sitter_pascal, 
        "program FibonacciExample;\n\ntype\n  TIntArray = array of Integer;\n\nfunction Fibonacci(n: Integer): Integer;\nbegin\n  if n <= 1 then\n    Result := n\n  else\n    Result := Fibonacci(n - 1) + Fibonacci(n - 2);\nend;\n\nprocedure PrintArray(const Arr: TIntArray);\nvar\n  i: Integer;\nbegin\n  for i := Low(Arr) to High(Arr) do\n    WriteLn(Arr[i]);\nend;\n\nbegin\n  WriteLn('Fibonacci: ', Fibonacci(10));\nend.");
    
    // 28. Common Lisp - uses LANGUAGE_COMMONLISP constant
    #[test]
    fn test_commonlisp() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_commonlisp::LANGUAGE_COMMONLISP.into())
            .expect("Failed to load Common Lisp");
        
        let code = "(defpackage :my-package\n  (:use :cl)\n  (:export #:main))\n\n(in-package :my-package)\n\n(defclass person ()\n  ((name :initarg :name\n         :accessor person-name)\n   (age :initarg :age\n        :accessor person-age)))\n\n(defmethod greet ((p person))\n  (format t \"Hello, ~A!~%\" (person-name p)))\n\n(defun factorial (n)\n  (if (<= n 1)\n      1\n      (* n (factorial (- n 1)))))\n\n(defun main ()\n  (let ((p (make-instance 'person :name \"Alice\" :age 30)))\n    (greet p)\n    (format t \"Factorial of 5: ~A~%\" (factorial 5))))";
        let tree = parser.parse(code, None)
            .expect("Failed to parse Common Lisp");
            
        assert!(!tree.root_node().has_error(), "Common Lisp parse has errors");
    }
    
    #[cfg(feature = "lang-objc")]
    smoke_test!(test_objc, tree_sitter_objc, 
        "#import <Foundation/Foundation.h>\n\n@interface Person : NSObject\n\n@property (nonatomic, strong) NSString *name;\n@property (nonatomic, assign) NSInteger age;\n\n- (instancetype)initWithName:(NSString *)name age:(NSInteger)age;\n- (void)greet;\n\n@end\n\n@implementation Person\n\n- (instancetype)initWithName:(NSString *)name age:(NSInteger)age {\n    self = [super init];\n    if (self) {\n        _name = name;\n        _age = age;\n    }\n    return self;\n}\n\n- (void)greet {\n    NSLog(@\"Hello, my name is %@\", self.name);\n}\n\n@end\n\nint main(int argc, const char * argv[]) {\n    @autoreleasepool {\n        Person *person = [[Person alloc] initWithName:@\"Alice\" age:30];\n        [person greet];\n    }\n    return 0;\n}");
    
    smoke_test!(test_groovy, tree_sitter_groovy, 
        "println 'Hello'\n\ndef x = 42\ndef y = x * 2\n\nprintln y");
    
    smoke_test!(test_embedded_template, tree_sitter_embedded_template, 
        "<!DOCTYPE html>\n<html>\n<head>\n    <title><%= title %></title>\n</head>\n<body>\n    <h1>Users List</h1>\n    <% if (users && users.length > 0) { %>\n        <ul>\n        <% users.forEach(function(user) { %>\n            <li>\n                <%= user.name %> - Age: <%= user.age %>\n                <% if (user.isAdmin) { %>\n                    <span class=\"admin\">Admin</span>\n                <% } %>\n            </li>\n        <% }); %>\n        </ul>\n    <% } else { %>\n        <p>No users found.</p>\n    <% } %>\n    \n    <footer>\n        © <%= new Date().getFullYear() %> My Company\n    </footer>\n</body>\n</html>");
    
    // ========== TIER 2: External Grammar Languages (42 languages) ==========
    
    // 32. JavaScript
    #[cfg(feature = "lang-javascript")]
    #[test]
    fn test_javascript() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_javascript::language())
            .expect("Failed to load JavaScript");
        
        let code = "class Calculator {\n  constructor() {\n    this.result = 0;\n  }\n  \n  add(value) {\n    this.result += value;\n    return this;\n  }\n  \n  async calculate() {\n    const data = await fetch('/api/data');\n    return data.json();\n  }\n}\n\nconst calc = new Calculator();\ncalc.add(5).add(10);\n\nconst numbers = [1, 2, 3, 4, 5];\nconst doubled = numbers.map(n => n * 2)\n  .filter(n => n > 5)\n  .reduce((acc, n) => acc + n, 0);";
        let tree = parser.parse(code, None)
            .expect("Failed to parse JavaScript");
            
        assert!(!tree.root_node().has_error(), "JavaScript parse has errors");
    }
    
    // 33. TypeScript
    #[cfg(feature = "lang-typescript")]
    #[test]
    fn test_typescript() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_typescript::language_typescript())
            .expect("Failed to load TypeScript");
        
        let code = "interface User {\n  id: number;\n  name: string;\n  email?: string;\n  roles: Role[];\n}\n\nenum Role {\n  Admin = 'ADMIN',\n  User = 'USER',\n  Guest = 'GUEST'\n}\n\nclass UserService {\n  private users: Map<number, User> = new Map();\n  \n  async getUser(id: number): Promise<User | undefined> {\n    return this.users.get(id);\n  }\n  \n  addUser<T extends User>(user: T): T {\n    this.users.set(user.id, user);\n    return user;\n  }\n}\n\ntype Result<T> = { success: true; data: T } | { success: false; error: string };";
        let tree = parser.parse(code, None)
            .expect("Failed to parse TypeScript");
            
        assert!(!tree.root_node().has_error(), "TypeScript parse has errors");
    }
    
    // 33b. TSX (gated under lang-typescript feature)
    #[cfg(feature = "lang-typescript")]
    #[test]
    fn test_tsx() {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_typescript::language_tsx())
            .expect("Failed to load TSX");
        
        let code = "import React, { useState, useEffect } from 'react';\n\ninterface Props {\n  title: string;\n  count?: number;\n}\n\nconst Counter: React.FC<Props> = ({ title, count = 0 }) => {\n  const [value, setValue] = useState(count);\n  \n  useEffect(() => {\n    console.log(`Count changed: ${value}`);\n  }, [value]);\n  \n  return (\n    <div className=\"counter\">\n      <h1>{title}</h1>\n      <button onClick={() => setValue(v => v + 1)}>\n        Count: {value}\n      </button>\n    </div>\n  );\n};\n\nexport default Counter;";
        let tree = parser.parse(code, None)
            .expect("Failed to parse TSX");
            
        assert!(!tree.root_node().has_error(), "TSX parse has errors");
    }
    
    // Additional external languages - using feature gates
    #[cfg(feature = "lang-elm")]
    smoke_test_fn!(test_elm, tree_sitter_elm, 
        "module Main exposing (..)\n\nimport Html exposing (Html, button, div, text)\nimport Html.Events exposing (onClick)\n\ntype alias Model = { count : Int }\n\ntype Msg = Increment | Decrement\n\nupdate : Msg -> Model -> Model\nupdate msg model =\n    case msg of\n        Increment -> { model | count = model.count + 1 }\n        Decrement -> { model | count = model.count - 1 }\n\nview : Model -> Html Msg\nview model =\n    div []\n        [ button [ onClick Decrement ] [ text \"-\" ]\n        , text (String.fromInt model.count)\n        , button [ onClick Increment ] [ text \"+\" ]\n        ]");
    
    #[cfg(feature = "lang-kotlin")]
    smoke_test_fn!(test_kotlin, tree_sitter_kotlin, 
        "package com.example\n\nimport kotlinx.coroutines.*\n\ndata class Person(val name: String, var age: Int) {\n    fun greet() = \"Hello, $name\"\n}\n\nclass UserRepository {\n    suspend fun getUser(id: Int): Person? = coroutineScope {\n        delay(1000)\n        Person(\"User$id\", 25)\n    }\n}\n\nfun main() = runBlocking {\n    val numbers = listOf(1, 2, 3, 4, 5)\n    val doubled = numbers.map { it * 2 }.filter { it > 5 }\n    \n    launch {\n        println(\"Coroutine started\")\n    }\n}");
    
    #[cfg(feature = "lang-yaml")]
    smoke_test_fn!(test_yaml, tree_sitter_yaml, 
        "name: CI/CD Pipeline\n\non:\n  push:\n    branches: [main, develop]\n  pull_request:\n    types: [opened, synchronize]\n\nenv:\n  NODE_VERSION: '16.x'\n\njobs:\n  test:\n    runs-on: ubuntu-latest\n    strategy:\n      matrix:\n        node-version: [14.x, 16.x, 18.x]\n    \n    steps:\n      - uses: actions/checkout@v3\n      - name: Setup Node.js\n        uses: actions/setup-node@v3\n        with:\n          node-version: ${{ matrix.node-version }}\n      \n      - run: npm ci\n      - run: npm test\n      - run: npm run build");
    
    // More external languages with production code samples
    #[cfg(feature = "lang-r")]
    smoke_test_fn!(test_r, tree_sitter_r,
        "library(ggplot2)\nlibrary(dplyr)\n\n# Define a function\nfibonacci <- function(n) {\n  if (n <= 1) {\n    return(n)\n  } else {\n    return(fibonacci(n - 1) + fibonacci(n - 2))\n  }\n}\n\n# Data manipulation\ndata <- data.frame(\n  x = 1:10,\n  y = rnorm(10)\n)\n\nresult <- data %>%\n  filter(x > 5) %>%\n  mutate(z = y * 2) %>%\n  summarise(mean_z = mean(z))\n\n# Plotting\nggplot(data, aes(x = x, y = y)) +\n  geom_point() +\n  geom_smooth(method = \"lm\")");
    
    #[cfg(feature = "lang-julia")]
    smoke_test_fn!(test_julia, tree_sitter_julia,
        "module MyModule\n\nexport fibonacci, Person\n\nstruct Person\n    name::String\n    age::Int\nend\n\nfunction fibonacci(n::Int)::Int\n    n <= 1 ? n : fibonacci(n - 1) + fibonacci(n - 2)\nend\n\nfunction process_data(data::Vector{Float64})\n    filtered = filter(x -> x > 0, data)\n    mapped = map(x -> x^2, filtered)\n    return reduce(+, mapped) / length(mapped)\nend\n\n# Macro definition\nmacro timeit(expr)\n    quote\n        start = time()\n        result = $expr\n        println(\"Elapsed: \", time() - start)\n        result\n    end\nend\n\nend # module");
    
    #[cfg(feature = "lang-haskell")]
    smoke_test_fn!(test_haskell, tree_sitter_haskell,
        "{-# LANGUAGE OverloadedStrings #-}\n\nmodule Main where\n\nimport Data.List (sort, nub)\nimport Control.Monad (forM_)\n\n-- Type definitions\ndata Person = Person\n    { name :: String\n    , age :: Int\n    } deriving (Show, Eq)\n\n-- Fibonacci with memoization\nfibonacci :: Int -> Integer\nfibonacci = (map fib [0..] !!)\n  where fib 0 = 0\n        fib 1 = 1\n        fib n = fibonacci (n-1) + fibonacci (n-2)\n\n-- Higher-order functions\nprocessList :: (Ord a) => [a] -> [a]\nprocessList = take 10 . nub . sort\n\nmain :: IO ()\nmain = do\n    putStrLn \"Hello, Haskell!\"\n    let numbers = [1..10]\n    forM_ numbers $ \\n ->\n        print $ fibonacci n");
    
    // More external languages
    #[cfg(feature = "lang-matlab")]
    smoke_test_fn!(test_matlab, tree_sitter_matlab,
        "function result = fibonacci(n)\n% Calculate Fibonacci number\n    if n <= 1\n        result = n;\n    else\n        result = fibonacci(n-1) + fibonacci(n-2);\n    end\nend\n\nclassdef Person < handle\n    properties\n        Name\n        Age\n    end\n    \n    methods\n        function obj = Person(name, age)\n            obj.Name = name;\n            obj.Age = age;\n        end\n        \n        function greet(obj)\n            fprintf('Hello, %s\\n', obj.Name);\n        end\n    end\nend\n\n% Matrix operations\nA = [1 2 3; 4 5 6; 7 8 9];\nB = inv(A);\nC = A .* B;\n[U, S, V] = svd(A);");
    
    #[cfg(feature = "lang-perl")]
    smoke_test_fn!(test_perl, tree_sitter_perl,
        "#!/usr/bin/perl\nuse strict;\nuse warnings;\nuse feature 'say';\n\n# Package definition\npackage Person {\n    sub new {\n        my ($class, %args) = @_;\n        bless \\%args, $class;\n    }\n    \n    sub greet {\n        my $self = shift;\n        say \"Hello, $self->{name}\";\n    }\n}\n\n# Subroutine\nsub fibonacci {\n    my $n = shift;\n    return $n if $n <= 1;\n    return fibonacci($n - 1) + fibonacci($n - 2);\n}\n\n# Main code\nmy @numbers = (1..10);\nmy @doubled = map { $_ * 2 } grep { $_ > 5 } @numbers;\n\nforeach my $num (@doubled) {\n    say \"Number: $num\";\n}\n\nmy $person = Person->new(name => 'Alice', age => 30);\n$person->greet();");
    
    #[cfg(feature = "lang-dart")]
    smoke_test_fn!(test_dart, tree_sitter_dart,
        "import 'dart:async';\n\nclass Person {\n  final String name;\n  int _age;\n  \n  Person(this.name, this._age);\n  \n  int get age => _age;\n  set age(int value) => _age = value;\n  \n  void greet() {\n    print('Hello, $name');\n  }\n}\n\nFuture<int> fibonacci(int n) async {\n  if (n <= 1) return n;\n  return await fibonacci(n - 1) + await fibonacci(n - 2);\n}\n\nvoid main() async {\n  final numbers = [1, 2, 3, 4, 5];\n  final doubled = numbers\n      .map((n) => n * 2)\n      .where((n) => n > 5)\n      .toList();\n  \n  final person = Person('Alice', 30);\n  person.greet();\n  \n  final result = await fibonacci(10);\n  print('Fibonacci: $result');\n}");
    
    #[cfg(feature = "lang-graphql")]
    smoke_test_fn!(test_graphql, tree_sitter_graphql,
        "schema {\n  query: Query\n  mutation: Mutation\n  subscription: Subscription\n}\n\ntype User {\n  id: ID!\n  name: String!\n  email: String\n  posts: [Post!]!\n  createdAt: DateTime!\n}\n\ntype Post {\n  id: ID!\n  title: String!\n  content: String!\n  author: User!\n  comments: [Comment!]!\n}\n\ntype Comment {\n  id: ID!\n  text: String!\n  author: User!\n}\n\ntype Query {\n  user(id: ID!): User\n  users(limit: Int = 10, offset: Int = 0): [User!]!\n  post(id: ID!): Post\n}\n\ntype Mutation {\n  createUser(input: CreateUserInput!): User!\n  updateUser(id: ID!, input: UpdateUserInput!): User!\n  deleteUser(id: ID!): Boolean!\n}\n\ninput CreateUserInput {\n  name: String!\n  email: String\n}\n\ninput UpdateUserInput {\n  name: String\n  email: String\n}");
    
    #[cfg(feature = "lang-sql")]
    smoke_test_fn!(test_sql, tree_sitter_sql,
        "-- Create database schema\nCREATE DATABASE IF NOT EXISTS myapp;\nUSE myapp;\n\n-- Create tables\nCREATE TABLE users (\n    id INT PRIMARY KEY AUTO_INCREMENT,\n    username VARCHAR(50) UNIQUE NOT NULL,\n    email VARCHAR(255) UNIQUE NOT NULL,\n    password_hash VARCHAR(255) NOT NULL,\n    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,\n    INDEX idx_email (email)\n);\n\nCREATE TABLE posts (\n    id INT PRIMARY KEY AUTO_INCREMENT,\n    user_id INT NOT NULL,\n    title VARCHAR(255) NOT NULL,\n    content TEXT,\n    published BOOLEAN DEFAULT FALSE,\n    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE\n);\n\n-- Complex query with JOIN and aggregation\nSELECT \n    u.username,\n    COUNT(p.id) as post_count,\n    MAX(p.created_at) as latest_post\nFROM users u\nLEFT JOIN posts p ON u.id = p.user_id\nWHERE p.published = TRUE\nGROUP BY u.id\nHAVING post_count > 5\nORDER BY post_count DESC\nLIMIT 10;");
    
    #[cfg(feature = "lang-zig")]
    smoke_test_fn!(test_zig, tree_sitter_zig,
        "const std = @import(\"std\");\n\nconst Person = struct {\n    name: []const u8,\n    age: u32,\n    \n    pub fn init(name: []const u8, age: u32) Person {\n        return Person{ .name = name, .age = age };\n    }\n    \n    pub fn greet(self: Person) void {\n        std.debug.print(\"Hello, {s}!\\n\", .{self.name});\n    }\n};\n\npub fn fibonacci(n: u32) u32 {\n    if (n <= 1) return n;\n    return fibonacci(n - 1) + fibonacci(n - 2);\n}\n\npub fn main() !void {\n    const allocator = std.heap.page_allocator;\n    \n    var list = std.ArrayList(i32).init(allocator);\n    defer list.deinit();\n    \n    try list.append(1);\n    try list.append(2);\n    try list.append(3);\n    \n    for (list.items) |item| {\n        std.debug.print(\"{} \", .{item});\n    }\n    \n    const person = Person.init(\"Alice\", 30);\n    person.greet();\n}");
    
    #[cfg(feature = "lang-vim")]
    smoke_test_fn!(test_vim, tree_sitter_vim,
        "\" Vim configuration file\nset nocompatible\nset number\nset relativenumber\nset expandtab\nset tabstop=4\nset shiftwidth=4\nset autoindent\nset smartindent\n\n\" Enable syntax highlighting\nsyntax on\nfiletype plugin indent on\n\n\" Custom functions\nfunction! Fibonacci(n)\n    if a:n <= 1\n        return a:n\n    else\n        return Fibonacci(a:n - 1) + Fibonacci(a:n - 2)\n    endif\nendfunction\n\n\" Commands and mappings\ncommand! -nargs=1 Fib echo Fibonacci(<args>)\nnnoremap <leader>f :Fib 10<CR>\n\n\" Autocommands\naugroup MyAutoCommands\n    autocmd!\n    autocmd BufWritePre * %s/\\s\\+$//e\n    autocmd FileType python setlocal expandtab tabstop=4\naugroup END\n\n\" Plugin settings\nlet g:airline_theme = 'dark'\nlet g:NERDTreeShowHidden = 1");
    
    #[cfg(feature = "lang-clojure")]
    smoke_test_fn!(test_clojure, tree_sitter_clojure,
        "(ns myapp.core\n  (:require [clojure.string :as str]\n            [clojure.core.async :refer [go chan <! >!]]))\n\n;; Define a record\n(defrecord Person [name age])\n\n;; Fibonacci function\n(defn fibonacci [n]\n  (cond\n    (<= n 1) n\n    :else (+ (fibonacci (- n 1))\n             (fibonacci (- n 2)))))\n\n;; Higher-order function\n(defn process-data [coll]\n  (->> coll\n       (filter #(> % 5))\n       (map #(* % 2))\n       (reduce +)))\n\n;; Macro definition\n(defmacro when-valid [test & body]\n  `(when ~test\n     ~@body))\n\n;; Async example\n(defn async-example []\n  (let [c (chan)]\n    (go (>! c \"Hello async!\"))\n    (go (println (<! c)))))\n\n;; Main function\n(defn -main [& args]\n  (let [person (->Person \"Alice\" 30)]\n    (println (:name person))\n    (println (fibonacci 10))))");
    
    #[cfg(feature = "lang-crystal")]
    smoke_test_fn!(test_crystal, tree_sitter_crystal,
        "require \"http/server\"\n\nclass Person\n  getter name : String\n  property age : Int32\n  \n  def initialize(@name : String, @age : Int32)\n  end\n  \n  def greet\n    puts \"Hello, #{@name}!\"\n  end\nend\n\nmodule Fibonacci\n  def self.calculate(n : Int32) : Int32\n    return n if n <= 1\n    calculate(n - 1) + calculate(n - 2)\n  end\nend\n\n# Generic class\nclass Stack(T)\n  def initialize\n    @items = [] of T\n  end\n  \n  def push(item : T)\n    @items << item\n  end\n  \n  def pop : T?\n    @items.pop?\n  end\nend\n\n# Main code\nnumbers = [1, 2, 3, 4, 5]\ndoubled = numbers.map { |n| n * 2 }.select { |n| n > 5 }\n\nperson = Person.new(\"Alice\", 30)\nperson.greet\n\nputs Fibonacci.calculate(10)");
    
    #[cfg(feature = "lang-fortran")]
    smoke_test_fn!(test_fortran, tree_sitter_fortran,
        "program main\n    use iso_fortran_env\n    implicit none\n    \n    type :: person\n        character(len=50) :: name\n        integer :: age\n    contains\n        procedure :: greet => person_greet\n    end type person\n    \n    integer :: i, result\n    real(real64), dimension(3,3) :: matrix\n    type(person) :: p\n    \n    ! Initialize person\n    p%name = \"Alice\"\n    p%age = 30\n    call p%greet()\n    \n    ! Calculate Fibonacci\n    result = fibonacci(10)\n    print *, \"Fibonacci(10) =\", result\n    \n    ! Matrix operations\n    call random_number(matrix)\n    call process_matrix(matrix)\n    \ncontains\n    \n    recursive function fibonacci(n) result(fib)\n        integer, intent(in) :: n\n        integer :: fib\n        \n        if (n <= 1) then\n            fib = n\n        else\n            fib = fibonacci(n-1) + fibonacci(n-2)\n        end if\n    end function fibonacci\n    \n    subroutine person_greet(this)\n        class(person), intent(in) :: this\n        print *, \"Hello,\", trim(this%name)\n    end subroutine person_greet\n    \n    subroutine process_matrix(mat)\n        real(real64), dimension(:,:), intent(inout) :: mat\n        mat = matmul(mat, transpose(mat))\n    end subroutine process_matrix\n    \nend program main");
    
    // Additional external languages (remaining from 73 total)
    #[cfg(feature = "lang-vhdl")]
    smoke_test_fn!(test_vhdl, tree_sitter_vhdl,
        "library IEEE;\nuse IEEE.STD_LOGIC_1164.ALL;\nuse IEEE.NUMERIC_STD.ALL;\n\nentity counter is\n    generic (\n        WIDTH : integer := 8\n    );\n    port (\n        clk : in std_logic;\n        rst : in std_logic;\n        enable : in std_logic;\n        count : out std_logic_vector(WIDTH-1 downto 0)\n    );\nend counter;\n\narchitecture Behavioral of counter is\n    signal count_reg : unsigned(WIDTH-1 downto 0);\nbegin\n    process(clk, rst)\n    begin\n        if rst = '1' then\n            count_reg <= (others => '0');\n        elsif rising_edge(clk) then\n            if enable = '1' then\n                count_reg <= count_reg + 1;\n            end if;\n        end if;\n    end process;\n    \n    count <= std_logic_vector(count_reg);\nend Behavioral;");
    
    #[cfg(feature = "lang-racket")]
    smoke_test_fn!(test_racket, tree_sitter_racket,
        "#lang racket\n\n(require racket/class)\n\n;; Define a struct\n(struct person (name age) #:transparent)\n\n;; Define a class\n(define person%\n  (class object%\n    (init-field name age)\n    (super-new)\n    \n    (define/public (greet)\n      (printf \"Hello, ~a!\\n\" name))))\n\n;; Fibonacci function\n(define (fibonacci n)\n  (cond\n    [(<= n 1) n]\n    [else (+ (fibonacci (- n 1))\n             (fibonacci (- n 2)))]))\n\n;; Higher-order functions\n(define (process-list lst)\n  (map (λ (x) (* x 2))\n       (filter (λ (x) (> x 5)) lst)))\n\n;; Main\n(define (main)\n  (let ([p (new person% [name \"Alice\"] [age 30])])\n    (send p greet)\n    (printf \"Fibonacci(10) = ~a\\n\" (fibonacci 10))))");
    
    #[cfg(feature = "lang-ada")]
    smoke_test_fn!(test_ada, tree_sitter_ada,
        "with Ada.Text_IO; use Ada.Text_IO;\nwith Ada.Integer_Text_IO; use Ada.Integer_Text_IO;\n\npackage body Calculator is\n   \n   type Person is record\n      Name : String(1..50);\n      Age  : Integer;\n   end record;\n   \n   function Fibonacci(N : Integer) return Integer is\n   begin\n      if N <= 1 then\n         return N;\n      else\n         return Fibonacci(N - 1) + Fibonacci(N - 2);\n      end if;\n   end Fibonacci;\n   \n   procedure Process_Array(Arr : in out Integer_Array) is\n   begin\n      for I in Arr'Range loop\n         Arr(I) := Arr(I) * 2;\n      end loop;\n   end Process_Array;\n   \nbegin\n   Put_Line(\"Ada Calculator Package\");\nend Calculator;\n\nprocedure Main is\n   P : Person := (Name => \"Alice\", Age => 30);\nbegin\n   Put_Line(\"Hello from Ada!\");\n   Put(\"Fibonacci(10) = \");\n   Put(Fibonacci(10));\n   New_Line;\nend Main;");
    
    #[cfg(feature = "lang-prolog")]
    smoke_test_fn!(test_prolog, tree_sitter_prolog,
        "% Facts\nperson(alice, 30).\nperson(bob, 25).\nperson(charlie, 35).\n\n% Rules\nolder(X, Y) :-\n    person(X, AgeX),\n    person(Y, AgeY),\n    AgeX > AgeY.\n\n% Fibonacci predicate\nfibonacci(0, 0).\nfibonacci(1, 1).\nfibonacci(N, F) :-\n    N > 1,\n    N1 is N - 1,\n    N2 is N - 2,\n    fibonacci(N1, F1),\n    fibonacci(N2, F2),\n    F is F1 + F2.\n\n% List operations\nappend_lists([], L, L).\nappend_lists([H|T1], L2, [H|T3]) :-\n    append_lists(T1, L2, T3).\n\n% Main query examples\n:- initialization(main).\nmain :-\n    fibonacci(10, F),\n    format('Fibonacci(10) = ~w~n', [F]),\n    halt.");
    
    #[cfg(feature = "lang-nim")]
    smoke_test_fn!(test_nim, tree_sitter_nim,
        "import strformat, sequtils, sugar\n\ntype\n  Person = object\n    name: string\n    age: int\n\nproc newPerson(name: string, age: int): Person =\n  Person(name: name, age: age)\n\nproc greet(p: Person) =\n  echo fmt\"Hello, {p.name}!\"\n\nproc fibonacci(n: int): int =\n  if n <= 1:\n    return n\n  else:\n    return fibonacci(n - 1) + fibonacci(n - 2)\n\nproc processSeq[T](s: seq[T], f: T -> T): seq[T] =\n  result = s.mapIt(f(it))\n\nwhen isMainModule:\n  let numbers = @[1, 2, 3, 4, 5]\n  let doubled = numbers.filterIt(it > 2).mapIt(it * 2)\n  \n  let person = newPerson(\"Alice\", 30)\n  person.greet()\n  \n  echo fmt\"Fibonacci(10) = {fibonacci(10)}\"\n  \n  for num in doubled:\n    echo num");
    
    #[cfg(feature = "lang-abap")]
    smoke_test_fn!(test_abap, tree_sitter_abap,
        "REPORT z_example.\n\nCLASS lcl_person DEFINITION.\n  PUBLIC SECTION.\n    DATA: name TYPE string,\n          age  TYPE i.\n    \n    METHODS: constructor IMPORTING iv_name TYPE string\n                                   iv_age  TYPE i,\n             greet.\nENDCLASS.\n\nCLASS lcl_person IMPLEMENTATION.\n  METHOD constructor.\n    name = iv_name.\n    age = iv_age.\n  ENDMETHOD.\n  \n  METHOD greet.\n    WRITE: / 'Hello,', name.\n  ENDMETHOD.\nENDCLASS.\n\nFORM fibonacci USING p_n TYPE i\n               CHANGING p_result TYPE i.\n  DATA: lv_n1 TYPE i,\n        lv_n2 TYPE i.\n  \n  IF p_n <= 1.\n    p_result = p_n.\n  ELSE.\n    lv_n1 = p_n - 1.\n    lv_n2 = p_n - 2.\n    PERFORM fibonacci USING lv_n1 CHANGING p_result.\n    PERFORM fibonacci USING lv_n2 CHANGING lv_n1.\n    p_result = p_result + lv_n1.\n  ENDIF.\nENDFORM.\n\nSTART-OF-SELECTION.\n  DATA: lo_person TYPE REF TO lcl_person,\n        lv_result TYPE i.\n  \n  CREATE OBJECT lo_person\n    EXPORTING\n      iv_name = 'Alice'\n      iv_age  = 30.\n  \n  lo_person->greet( ).\n  \n  PERFORM fibonacci USING 10 CHANGING lv_result.\n  WRITE: / 'Fibonacci(10) =', lv_result.");
    
    #[cfg(feature = "lang-gradle")]
    smoke_test_fn!(test_gradle, tree_sitter_gradle,
        "plugins {\n    id 'java'\n    id 'application'\n    id 'com.github.johnrengelman.shadow' version '7.1.2'\n}\n\ngroup = 'com.example'\nversion = '1.0.0'\nsourceCompatibility = JavaVersion.VERSION_17\n\nrepositories {\n    mavenCentral()\n    google()\n}\n\ndependencies {\n    implementation 'org.springframework.boot:spring-boot-starter-web:3.0.0'\n    implementation 'com.google.guava:guava:31.1-jre'\n    \n    testImplementation 'junit:junit:4.13.2'\n    testImplementation 'org.mockito:mockito-core:4.9.0'\n}\n\napplication {\n    mainClass = 'com.example.Main'\n}\n\ntask customTask {\n    doLast {\n        println 'Running custom task'\n    }\n}\n\njar {\n    manifest {\n        attributes(\n            'Main-Class': 'com.example.Main',\n            'Implementation-Version': project.version\n        )\n    }\n}\n\ntest {\n    useJUnitPlatform()\n    maxHeapSize = '1G'\n}");
    
    #[cfg(feature = "lang-xml")]
    smoke_test_fn!(test_xml, tree_sitter_xml,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<project xmlns=\"http://maven.apache.org/POM/4.0.0\"\n         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n         xsi:schemaLocation=\"http://maven.apache.org/POM/4.0.0\n         http://maven.apache.org/xsd/maven-4.0.0.xsd\">\n    \n    <modelVersion>4.0.0</modelVersion>\n    <groupId>com.example</groupId>\n    <artifactId>my-app</artifactId>\n    <version>1.0.0</version>\n    \n    <properties>\n        <maven.compiler.source>17</maven.compiler.source>\n        <maven.compiler.target>17</maven.compiler.target>\n    </properties>\n    \n    <dependencies>\n        <dependency>\n            <groupId>org.springframework</groupId>\n            <artifactId>spring-core</artifactId>\n            <version>6.0.0</version>\n        </dependency>\n    </dependencies>\n    \n    <build>\n        <plugins>\n            <plugin>\n                <groupId>org.apache.maven.plugins</groupId>\n                <artifactId>maven-compiler-plugin</artifactId>\n                <version>3.10.1</version>\n            </plugin>\n        </plugins>\n    </build>\n</project>");
    
    #[cfg(feature = "lang-solidity")]
    smoke_test_fn!(test_solidity, tree_sitter_solidity,
        "// SPDX-License-Identifier: MIT\npragma solidity ^0.8.0;\n\ncontract Token {\n    mapping(address => uint256) private balances;\n    mapping(address => mapping(address => uint256)) private allowances;\n    \n    uint256 public totalSupply;\n    string public name;\n    string public symbol;\n    uint8 public decimals;\n    \n    event Transfer(address indexed from, address indexed to, uint256 value);\n    event Approval(address indexed owner, address indexed spender, uint256 value);\n    \n    constructor(string memory _name, string memory _symbol, uint256 _totalSupply) {\n        name = _name;\n        symbol = _symbol;\n        decimals = 18;\n        totalSupply = _totalSupply * 10**uint256(decimals);\n        balances[msg.sender] = totalSupply;\n    }\n    \n    function balanceOf(address account) public view returns (uint256) {\n        return balances[account];\n    }\n    \n    function transfer(address to, uint256 amount) public returns (bool) {\n        require(balances[msg.sender] >= amount, \"Insufficient balance\");\n        balances[msg.sender] -= amount;\n        balances[to] += amount;\n        emit Transfer(msg.sender, to, amount);\n        return true;\n    }\n}");
    
    #[cfg(feature = "lang-hcl")]
    smoke_test_fn!(test_hcl, tree_sitter_hcl,
        "terraform {\n  required_version = \">= 1.0\"\n  \n  required_providers {\n    aws = {\n      source  = \"hashicorp/aws\"\n      version = \"~> 4.0\"\n    }\n  }\n  \n  backend \"s3\" {\n    bucket = \"my-terraform-state\"\n    key    = \"prod/terraform.tfstate\"\n    region = \"us-east-1\"\n  }\n}\n\nvariable \"instance_type\" {\n  description = \"EC2 instance type\"\n  type        = string\n  default     = \"t3.micro\"\n}\n\nlocals {\n  common_tags = {\n    Environment = \"Production\"\n    ManagedBy   = \"Terraform\"\n  }\n}\n\nresource \"aws_instance\" \"web\" {\n  count         = 3\n  ami           = data.aws_ami.ubuntu.id\n  instance_type = var.instance_type\n  \n  tags = merge(\n    local.common_tags,\n    {\n      Name = \"web-${count.index + 1}\"\n    }\n  )\n}\n\noutput \"instance_ips\" {\n  value = aws_instance.web[*].public_ip\n}");
    
    #[cfg(feature = "lang-fsharp")]
    smoke_test_fn!(test_fsharp, tree_sitter_fsharp,
        "module Calculator\n\nopen System\n\ntype Person = {\n    Name: string\n    Age: int\n}\n\nlet rec fibonacci n =\n    match n with\n    | 0 | 1 -> n\n    | _ -> fibonacci (n - 1) + fibonacci (n - 2)\n\nlet processNumbers numbers =\n    numbers\n    |> List.filter (fun x -> x > 5)\n    |> List.map (fun x -> x * 2)\n    |> List.reduce (+)\n\ntype Shape =\n    | Circle of radius: float\n    | Rectangle of width: float * height: float\n    | Triangle of base: float * height: float\n\nlet area shape =\n    match shape with\n    | Circle r -> Math.PI * r * r\n    | Rectangle (w, h) -> w * h\n    | Triangle (b, h) -> 0.5 * b * h\n\n[<EntryPoint>]\nlet main argv =\n    let person = { Name = \"Alice\"; Age = 30 }\n    printfn \"Hello, %s!\" person.Name\n    \n    let result = fibonacci 10\n    printfn \"Fibonacci(10) = %d\" result\n    \n    let numbers = [1..10]\n    let sum = processNumbers numbers\n    printfn \"Sum: %d\" sum\n    \n    0");
    
    // Closing test module
}
