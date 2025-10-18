use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    sub: String,
    exp: usize,
}

/// Generates a JWT token for the given user ID
pub fn generate_jwt_token(user_id: &str, secret: &str) -> Result<String> {
    let expiration = Utc::now() + Duration::hours(24);
    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration.timestamp() as usize,
    };
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref())
    )?;
    Ok(token)
}

/// Validates a JWT token and returns the claims
pub fn validate_jwt_token(token: &str, secret: &str) -> Result<Claims> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default()
    )?;
    Ok(token_data.claims)
}