// Basic test file to verify tree-sitter parsing works
fn main() {
    println!("Hello, world!");
}

struct Person {
    name: String,
    age: u32,
}

impl Person {
    fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }
    
    fn greet(&self) {
        println!("Hello, my name is {}", self.name);
    }
}

// Test function
fn calculate(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_calculate() {
        assert_eq!(calculate(2, 3), 5);
    }
}
