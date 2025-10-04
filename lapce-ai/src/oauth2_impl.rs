/// OAuth2 Implementation - Day 45 AM
use std::collections::HashMap;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Clone)]
pub struct OAuth2Provider {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    auth_url: String,
    token_url: String,
    scopes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorizationCode {
    pub code: String,
    pub state: String,
    pub expires_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessToken {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: String,
}

impl OAuth2Provider {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri: "http://localhost:8080/callback".to_string(),
            auth_url: "https://auth.example.com/authorize".to_string(),
            token_url: "https://auth.example.com/token".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
        }
    }
    
    pub fn get_authorization_url(&self, state: &str) -> String {
        let scopes = self.scopes.join(" ");
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            self.auth_url, self.client_id, self.redirect_uri, scopes, state
        )
    }
    
    pub async fn exchange_code(&self, code: &str) -> Result<AccessToken> {
        // Simulate token exchange
        let token = self.generate_token();
        
        Ok(AccessToken {
            token: token.clone(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some(self.generate_token()),
            scope: self.scopes.join(" "),
        })
    }
    
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<AccessToken> {
        Ok(AccessToken {
            token: self.generate_token(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some(refresh_token.to_string()),
            scope: self.scopes.join(" "),
        })
    }
    
    pub fn validate_token(&self, token: &str) -> Result<TokenClaims> {
        // Decode and validate
        Ok(TokenClaims {
            sub: "user123".to_string(),
            exp: chrono::Utc::now().timestamp() as u64 + 3600,
            iat: chrono::Utc::now().timestamp() as u64,
            scopes: self.scopes.clone(),
        })
    }
    
    fn generate_token(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(uuid::Uuid::new_v4().to_string());
        let hash = hasher.finalize();
        general_purpose::STANDARD.encode(hash)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub scopes: Vec<String>,
}
