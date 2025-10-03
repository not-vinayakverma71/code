/// Exact 1:1 Translation of TypeScript handleUri from codex-reference/activate/handleUri.ts
/// DAY 10 H1-2: Translate handleUri.ts

use std::collections::HashMap;

/// Uri structure placeholder
#[derive(Debug, Clone)]
pub struct Uri {
    pub path: String,
    pub query: String,
}

/// handleUri - exact translation lines 7-72
pub async fn handle_uri(uri: Uri) -> Result<(), String> {
    let path = uri.path;
    let query = parse_query_string(&uri.query);
    
    // Get visible provider instance - line 10
    let visible_provider = match ClineProvider::get_visible_instance() {
        Some(provider) => provider,
        None => return Ok(()), // line 13
    };
    
    // Switch on path - lines 16-71
    match path.as_str() {
        "/glama" => {
            if let Some(code) = query.get("code") {
                visible_provider.handle_glama_callback(code).await?;
            }
        }
        "/openrouter" => {
            if let Some(code) = query.get("code") {
                visible_provider.handle_open_router_callback(code).await?;
            }
        }
        "/kilocode" => {
            if let Some(token) = query.get("token") {
                visible_provider.handle_kilo_code_callback(token).await?;
            }
        }
        // kilocode_change - lines 39-48
        "/kilocode/profile" => {
            visible_provider.post_message_to_webview(crate::handler_registration::WebviewMessage {
                type_: "action".to_string(),
                text: Some("openSettingsAccountTab".to_string()),
                data: None,
            }).await?;
            
            visible_provider.post_message_to_webview(crate::handler_registration::WebviewMessage {
                type_: "updateProfileData".to_string(),
                text: None,
                data: None,
            }).await?;
        }
        "/requesty" => {
            if let Some(code) = query.get("code") {
                visible_provider.handle_requesty_callback(code).await?;
            }
        }
        "/auth/clerk/callback" => {
            let code = query.get("code").cloned();
            let state = query.get("state").cloned();
            let organization_id = query.get("organizationId")
                .and_then(|id| if id == "null" { None } else { Some(id.clone()) });
            
            CloudService::handle_auth_callback(
                code,
                state,
                organization_id,
            ).await?;
        }
        _ => {
            // Default case - do nothing
        }
    }
    
    Ok(())
}

/// Parse query string into HashMap
fn parse_query_string(query: &str) -> HashMap<String, String> {
    // Replace + with %2B before parsing
    let query = query.replace('+', "%2B");
    
    let mut params = HashMap::new();
    
    for pair in query.split('&') {
        let parts: Vec<&str> = pair.split('=').collect();
        if parts.len() == 2 {
            let key = urlencoding::decode(parts[0]).unwrap_or_else(|_| parts[0].into()).to_string();
            let value = urlencoding::decode(parts[1]).unwrap_or_else(|_| parts[1].into()).to_string();
            params.insert(key, value);
        }
    }
    
    params
}

/// ClineProvider placeholder
pub struct ClineProvider;

impl ClineProvider {
    pub fn get_visible_instance() -> Option<ClineProvider> {
        // Placeholder implementation
        Some(ClineProvider)
    }
    
    pub async fn handle_glama_callback(&self, code: &str) -> Result<(), String> {
        println!("Handling Glama callback with code: {}", code);
        Ok(())
    }
    
    pub async fn handle_open_router_callback(&self, code: &str) -> Result<(), String> {
        println!("Handling OpenRouter callback with code: {}", code);
        Ok(())
    }
    
    pub async fn handle_kilo_code_callback(&self, token: &str) -> Result<(), String> {
        println!("Handling KiloCode callback with token: {}", token);
        Ok(())
    }
    
    pub async fn handle_requesty_callback(&self, code: &str) -> Result<(), String> {
        println!("Handling Requesty callback with code: {}", code);
        Ok(())
    }
    
    pub async fn post_message_to_webview(&self, message: crate::handler_registration::WebviewMessage) -> Result<(), String> {
        println!("Posting message to webview: {:?}", message);
        Ok(())
    }
}

/// WebviewMessage structure
#[derive(Debug, Clone, Default)]
pub struct WebviewMessage {
    pub msg_type: String,
    pub action: Option<String>,
}

/// CloudService placeholder
pub struct CloudService;

impl CloudService {
    pub async fn handle_auth_callback(
        code: Option<String>,
        state: Option<String>,
        organization_id: Option<String>,
    ) -> Result<(), String> {
        println!("Handling auth callback - code: {:?}, state: {:?}, org_id: {:?}", 
            code, state, organization_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_query_string() {
        let query = "code=test123&state=abc+def&organizationId=null";
        let params = parse_query_string(query);
        
        assert_eq!(params.get("code"), Some(&"test123".to_string()));
        assert_eq!(params.get("state"), Some(&"abc%2Bdef".to_string()));
        assert_eq!(params.get("organizationId"), Some(&"null".to_string()));
    }
    
    #[tokio::test]
    async fn test_handle_uri_glama() {
        let uri = Uri {
            path: "/glama".to_string(),
            query: "code=test_code".to_string(),
        };
        
        let result = handle_uri(uri).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_handle_uri_kilocode_profile() {
        let uri = Uri {
            path: "/kilocode/profile".to_string(),
            query: "".to_string(),
        };
        
        let result = handle_uri(uri).await;
        assert!(result.is_ok());
    }
}
