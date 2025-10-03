pub struct Service_16 {
    name: String,
}

impl Service_16 {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
    
    pub async fn execute(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(format!("Service {} executed", self.name))
    }
}