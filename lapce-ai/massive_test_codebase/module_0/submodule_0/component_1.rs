// Component 1
use std::collections::HashMap;
use async_trait::async_trait;

pub struct Component1 {
id: String,
data: HashMap<String, String>,
}

impl Component1 {
pub async fn process(&self) -> Result<()> {
// Processing logic
Ok(())
}
}

#[cfg(test)]
mod tests {
use super::*;
#[test]
fn test_component() {
let comp = Component1 { id: "test".into(), data: HashMap::new() };
assert_eq!(comp.id, "test");
}
}