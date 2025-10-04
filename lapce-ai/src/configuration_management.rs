/// Exact 1:1 Translation of TypeScript configuration from codex-reference/utils/config.ts
/// DAY 6 H3-4: Translate configuration management

use serde::{Deserialize, Serialize};
use serde_json::{Value, Map};
use std::collections::HashMap;
use std::env;
use regex::Regex;

/// InjectableConfigType - exact translation lines 1-11
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InjectableConfigType {
    String(String),
    Object(Map<String, Value>),
    Array(Vec<Value>),
    Number(f64),
    Boolean(bool),
    Null,
}

/// Deeply injects environment variables into a configuration object/string/json
/// Exact translation lines 20-22
pub async fn inject_env<C>(config: C, not_found_value: Option<String>) -> Result<C, String>
where
    C: Serialize + for<'de> Deserialize<'de>,
{
    let env_vars: HashMap<String, String> = env::vars().collect();
    let mut variables = HashMap::new();
    variables.insert("env".to_string(), VariableValue::Map(env_vars));
    
    inject_variables(config, variables, not_found_value).await
}

/// Variable value types
#[derive(Debug, Clone)]
pub enum VariableValue {
    String(String),
    Map(HashMap<String, String>),
}

/// Deeply injects variables into a configuration object/string/json
/// Exact translation lines 35-66
pub async fn inject_variables<C>(
    config: C,
    variables: HashMap<String, VariableValue>,
    prop_not_found_value: Option<String>,
) -> Result<C, String>
where
    C: Serialize + for<'de> Deserialize<'de>,
{
    // Serialize config to JSON string
    let config_json = serde_json::to_string(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    let is_string_input = serde_json::from_str::<String>(&config_json).is_ok();
    let mut config_string = if is_string_input {
        serde_json::from_str::<String>(&config_json).unwrap()
    } else {
        config_json
    };
    
    // Process variables
    for (key, value) in variables.iter() {
        match value {
            VariableValue::String(val) => {
                // Simple string replacement - line 47-48
                let pattern = format!(r"\$\{{{}\}}", regex::escape(key));
                let re = Regex::new(&pattern).unwrap();
                config_string = re.replace_all(&config_string, to_posix_path(val)).to_string();
            }
            VariableValue::Map(map) => {
                // Handle nested variables (e.g., ${env:VAR_NAME}) - lines 50-61
                let pattern = format!(r"\$\{{{}:(\w+)\}}", regex::escape(key));
                let re = Regex::new(&pattern).unwrap();
                
                config_string = re.replace_all(&config_string, |caps: &regex::Captures| {
                    let name = &caps[1];
                    
                    if let Some(nested_value) = map.get(name) {
                        to_posix_path(nested_value)
                    } else {
                        eprintln!("[injectVariables] variable \"{}\" referenced but not found in \"{}\"", name, key);
                        prop_not_found_value.as_ref()
                            .map(|v| v.clone())
                            .unwrap_or_else(|| caps[0].to_string())
                    }
                }).to_string();
            }
        }
    }
    
    // Deserialize back to original type
    if is_string_input {
        serde_json::from_str(&format!("\"{}\"", config_string))
            .map_err(|e| format!("Failed to deserialize config: {}", e))
    } else {
        serde_json::from_str(&config_string)
            .map_err(|e| format!("Failed to deserialize config: {}", e))
    }
}

/// Convert path to POSIX format (forward slashes)
fn to_posix_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// Configuration manager for handling app configuration
pub struct ConfigurationManager {
    config: HashMap<String, Value>,
    env_injected: bool,
}

impl ConfigurationManager {
    pub fn new() -> Self {
        Self {
            config: HashMap::new(),
            env_injected: false,
        }
    }
    
    /// Load configuration from JSON string
    pub fn load_from_json(&mut self, json: &str) -> Result<(), String> {
        let parsed: HashMap<String, Value> = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse config JSON: {}", e))?;
        
        self.config = parsed;
        self.env_injected = false;
        Ok(())
    }
    
    /// Get configuration value by key
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.config.get(key)
    }
    
    /// Set configuration value
    pub fn set(&mut self, key: String, value: Value) {
        self.config.insert(key, value);
        self.env_injected = false;
    }
    
    /// Inject environment variables into configuration
    pub async fn inject_env_vars(&mut self) -> Result<(), String> {
        if self.env_injected {
            return Ok(());
        }
        
        let injected = inject_env(self.config.clone(), None).await?;
        self.config = injected;
        self.env_injected = true;
        Ok(())
    }
    
    /// Export configuration as JSON string
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.config)
            .map_err(|e| format!("Failed to serialize config: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[tokio::test]
    async fn test_inject_env() {
        env::set_var("TEST_VAR", "test_value");
        
        let config = "${env:TEST_VAR}";
        let result: String = inject_env(config.to_string(), None).await.unwrap();
        
        assert_eq!(result, "test_value");
    }
    
    #[tokio::test]
    async fn test_inject_variables_nested() {
        let mut variables = HashMap::new();
        let mut env_map = HashMap::new();
        env_map.insert("HOME".to_string(), "/home/user".to_string());
        env_map.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        variables.insert("env".to_string(), VariableValue::Map(env_map));
        
        let config = r#"{"home": "${env:HOME}", "path": "${env:PATH}"}"#;
        let result: HashMap<String, String> = inject_variables(
            serde_json::from_str(config).unwrap(),
            variables,
            None
        ).await.unwrap();
        
        assert_eq!(result.get("home").unwrap(), "/home/user");
        assert_eq!(result.get("path").unwrap(), "/usr/bin:/bin");
    }
    
    #[test]
    fn test_to_posix_path() {
        assert_eq!(to_posix_path(r"C:\Users\test"), "C:/Users/test");
        assert_eq!(to_posix_path("/home/user"), "/home/user");
    }
    
    #[test]
    fn test_config_manager() {
        let mut manager = ConfigurationManager::new();
        
        manager.load_from_json(r#"{"key": "value", "number": 42}"#).unwrap();
        
        assert_eq!(manager.get("key"), Some(&Value::String("value".to_string())));
        assert_eq!(manager.get("number"), Some(&Value::Number(42.into())));
        
        manager.set("new_key".to_string(), Value::String("new_value".to_string()));
        assert_eq!(manager.get("new_key"), Some(&Value::String("new_value".to_string())));
    }
}
