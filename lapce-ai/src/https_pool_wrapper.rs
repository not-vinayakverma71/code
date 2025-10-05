/// Wrapper to make HttpsConnectionManager work with bb8
/// Implements ManageConnection trait for bb8 pool

use std::time::Duration;
use anyhow::Result;
use bb8::ManageConnection;
use async_trait::async_trait;

use crate::https_connection_manager_real::HttpsConnectionManager;

/// Custom error type that implements std::error::Error
#[derive(Debug)]
pub struct PoolError(String);

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PoolError {}

impl From<anyhow::Error> for PoolError {
    fn from(e: anyhow::Error) -> Self {
        PoolError(e.to_string())
    }
}

/// Wrapper to adapt HttpsConnectionManager for bb8
#[derive(Debug, Clone)]
pub struct HttpsConnectionPool;

#[async_trait]
impl ManageConnection for HttpsConnectionPool {
    type Connection = HttpsConnectionManager;
    type Error = PoolError;
    
    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        HttpsConnectionManager::new().await.map_err(Into::into)
    }
    
    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        // Use the real validation method
        conn.is_valid().await.map_err(Into::into)
    }
    
    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        // Check if connection is broken
        conn.is_broken() || conn.is_expired(Duration::from_secs(300))
    }
}
