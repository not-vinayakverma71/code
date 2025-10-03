/// JWT Token Management - Day 45 PM
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, Algorithm};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub nbf: usize,
    pub role: String,
    pub permissions: Vec<String>,
}

pub struct JWTManager {
    secret: String,
    issuer: String,
    audience: String,
    expiry_seconds: u64,
}

impl JWTManager {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            issuer: "lapce-ai-rust".to_string(),
            audience: "lapce-users".to_string(),
            expiry_seconds: 3600,
        }
    }
    
    pub fn generate_token(&self, user_id: &str, role: &str, permissions: Vec<String>) -> Result<String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as usize;
        
        let claims = Claims {
            sub: user_id.to_string(),
            exp: now + self.expiry_seconds as usize,
            iat: now,
            nbf: now,
            role: role.to_string(),
            permissions,
        };
        
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes())
        )?;
        
        Ok(token)
    }
    
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let validation = Validation::new(Algorithm::HS256);
        
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation
        )?;
        
        Ok(token_data.claims)
    }
    
    pub fn refresh_token(&self, old_token: &str) -> Result<String> {
        let claims = self.validate_token(old_token)?;
        self.generate_token(&claims.sub, &claims.role, claims.permissions)
    }
}
