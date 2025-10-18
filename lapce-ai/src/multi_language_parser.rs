// Day 19: Multi-Language Parser Support

pub trait LanguageParser {
    fn parse(&self, content: &str) -> ParseResult;
    fn extract_functions(&self, content: &str) -> Vec<FunctionDef>;
    fn extract_classes(&self, content: &str) -> Vec<ClassDef>;
    fn extract_imports(&self, content: &str) -> Vec<ImportDef>;
}

pub struct ParseResult {
    pub functions: Vec<FunctionDef>,
    pub classes: Vec<ClassDef>,
    pub imports: Vec<ImportDef>,
    pub variables: Vec<VariableDef>,
}

pub struct FunctionDef {
    pub name: String,
    pub params: Vec<String>,
    pub return_type: Option<String>,
    pub line_start: usize,
    pub line_end: usize,
}

pub struct ClassDef {
    pub name: String,
    pub methods: Vec<FunctionDef>,
    pub fields: Vec<VariableDef>,
}

pub struct ImportDef {
    pub module: String,
    pub items: Vec<String>,
}

pub struct VariableDef {
    pub name: String,
    pub var_type: Option<String>,
    pub value: Option<String>,
}

// Rust Parser
pub struct RustParser;
impl LanguageParser for RustParser {
    fn parse(&self, content: &str) -> ParseResult {
        ParseResult {
            functions: self.extract_functions(content),
            classes: self.extract_classes(content),
            imports: self.extract_imports(content),
            variables: vec![],
        }
    }
    
    fn extract_functions(&self, content: &str) -> Vec<FunctionDef> {
        let mut functions = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if line.trim().starts_with("fn ") {
                let name = line.split_whitespace()
                    .nth(1)
                    .unwrap_or("")
                    .split('(').next()
                    .unwrap_or("")
                    .to_string();
                functions.push(FunctionDef {
                    name,
                    params: vec![],
                    return_type: None,
                    line_start: i + 1,
                    line_end: i + 1,
                });
            }
        }
        functions
    }
    
    fn extract_classes(&self, content: &str) -> Vec<ClassDef> {
        let mut classes = Vec::new();
        for line in content.lines() {
            if line.trim().starts_with("struct ") {
                let name = line.split_whitespace()
                    .nth(1)
                    .unwrap_or("")
                    .to_string();
                classes.push(ClassDef {
                    name,
                    methods: vec![],
                    fields: vec![],
                });
            }
        }
        classes
    }
    
    fn extract_imports(&self, content: &str) -> Vec<ImportDef> {
        content.lines()
            .filter(|l| l.trim().starts_with("use "))
            .map(|l| ImportDef {
                module: l.trim()[4..].trim_end_matches(';').to_string(),
                items: vec![],
            })
            .collect()
    }
}

// Python Parser
pub struct PythonParser;
impl LanguageParser for PythonParser {
    fn parse(&self, content: &str) -> ParseResult {
        ParseResult {
            functions: self.extract_functions(content),
            classes: self.extract_classes(content),
            imports: self.extract_imports(content),
            variables: vec![],
        }
    }
    
    fn extract_functions(&self, content: &str) -> Vec<FunctionDef> {
        content.lines()
            .enumerate()
            .filter(|(_, l)| l.trim().starts_with("def "))
            .map(|(i, l)| FunctionDef {
                name: l.split_whitespace()
                    .nth(1)
                    .unwrap_or("")
                    .split('(').next()
                    .unwrap_or("")
                    .to_string(),
                params: vec![],
                return_type: None,
                line_start: i + 1,
                line_end: i + 1,
            })
            .collect()
    }
    
    fn extract_classes(&self, content: &str) -> Vec<ClassDef> {
        content.lines()
            .filter(|l| l.trim().starts_with("class "))
            .map(|l| ClassDef {
                name: l.split_whitespace()
                    .nth(1)
                    .unwrap_or("")
                    .split(':').next()
                    .unwrap_or("")
                    .to_string(),
                methods: vec![],
                fields: vec![],
            })
            .collect()
    }
    
    fn extract_imports(&self, content: &str) -> Vec<ImportDef> {
        content.lines()
            .filter(|l| l.trim().starts_with("import ") || l.trim().starts_with("from "))
            .map(|l| ImportDef {
                module: l.trim().to_string(),
                items: vec![],
            })
            .collect()
    }
}

// JavaScript/TypeScript Parser
pub struct JSParser;
impl LanguageParser for JSParser {
    fn parse(&self, content: &str) -> ParseResult {
        ParseResult {
            functions: self.extract_functions(content),
            classes: self.extract_classes(content),
            imports: self.extract_imports(content),
            variables: vec![],
        }
    }
    
    fn extract_functions(&self, content: &str) -> Vec<FunctionDef> {
        content.lines()
            .enumerate()
            .filter(|(_, l)| l.contains("function ") || l.contains("const ") && l.contains("=>"))
            .map(|(i, l)| FunctionDef {
                name: extract_js_function_name(l),
                params: vec![],
                return_type: None,
                line_start: i + 1,
                line_end: i + 1,
            })
            .collect()
    }
    
    fn extract_classes(&self, content: &str) -> Vec<ClassDef> {
        content.lines()
            .filter(|l| l.trim().starts_with("class "))
            .map(|l| ClassDef {
                name: l.split_whitespace()
                    .nth(1)
                    .unwrap_or("")
                    .to_string(),
                methods: vec![],
                fields: vec![],
            })
            .collect()
    }
    
    fn extract_imports(&self, content: &str) -> Vec<ImportDef> {
        content.lines()
            .filter(|l| l.trim().starts_with("import ") || l.trim().starts_with("const ") && l.contains("require"))
            .map(|l| ImportDef {
                module: l.trim().to_string(),
                items: vec![],
            })
            .collect()
    }
}

// Go Parser
pub struct GoParser;
impl LanguageParser for GoParser {
    fn parse(&self, content: &str) -> ParseResult {
        ParseResult {
            functions: self.extract_functions(content),
            classes: self.extract_classes(content),
            imports: self.extract_imports(content),
            variables: vec![],
        }
    }
    
    fn extract_functions(&self, content: &str) -> Vec<FunctionDef> {
        content.lines()
            .enumerate()
            .filter(|(_, l)| l.trim().starts_with("func "))
            .map(|(i, l)| FunctionDef {
                name: extract_go_function_name(l),
                params: vec![],
                return_type: None,
                line_start: i + 1,
                line_end: i + 1,
            })
            .collect()
    }
    
    fn extract_classes(&self, content: &str) -> Vec<ClassDef> {
        content.lines()
            .filter(|l| l.trim().starts_with("type ") && l.contains("struct"))
            .map(|l| ClassDef {
                name: l.split_whitespace()
                    .nth(1)
                    .unwrap_or("")
                    .to_string(),
                methods: vec![],
                fields: vec![],
            })
            .collect()
    }
    
    fn extract_imports(&self, content: &str) -> Vec<ImportDef> {
        content.lines()
            .filter(|l| l.trim().starts_with("import "))
            .map(|l| ImportDef {
                module: l.trim().to_string(),
                items: vec![],
            })
            .collect()
    }
}

fn extract_js_function_name(line: &str) -> String {
    if line.contains("function ") {
        line.split("function ")
            .nth(1)
            .and_then(|s| s.split('(').next())
            .unwrap_or("")
            .trim()
            .to_string()
    } else if line.contains("const ") {
        line.split("const ")
            .nth(1)
            .and_then(|s| s.split('=').next())
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        String::new()
    }
}

fn extract_go_function_name(line: &str) -> String {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() > 1 {
        if parts[1].starts_with('(') {
            // Method
            parts.get(2).unwrap_or(&"").split('(').next().unwrap_or("").to_string()
        } else {
            // Function
            parts[1].split('(').next().unwrap_or("").to_string()
        }
    } else {
        String::new()
    }
}

pub fn get_parser(language: &str) -> Box<dyn LanguageParser> {
    match language.to_lowercase().as_str() {
        "rust" | "rs" => Box::new(RustParser),
        "python" | "py" => Box::new(PythonParser),
        "javascript" | "js" | "jsx" => Box::new(JSParser),
        "typescript" | "ts" | "tsx" => Box::new(JSParser),
        "go" => Box::new(GoParser),
        _ => Box::new(RustParser), // Default
    }
}
