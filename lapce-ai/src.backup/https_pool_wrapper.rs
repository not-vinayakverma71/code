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
        // Don't do health checks on every acquisition - trust the connection
        // This was causing every acquisition to make a network request
        if conn.is_broken() {
            return Err(PoolError("Connection is broken".to_string()));
        }
        Ok(())
    }
    
    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        // Only mark as broken if truly broken, not just expired
        conn.is_broken()
    }
}
