/// Complete Connection Pool Implementation
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Semaphore, RwLock};
use dashmap::DashMap;
use anyhow::Result;

#[derive(Clone)]
pub struct Connection {
    pub id: u64,
    pub created_at: Instant,
    pub last_used: Instant,
}

impl Connection {
    pub fn new(id: u64) -> Self {
        let now = Instant::now();
        Self {
            id,
            created_at: now,
            last_used: now,
        }
    }
    
    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_used.elapsed() > timeout
    }
}

pub struct ConnectionPool {
    idle: Arc<RwLock<Vec<Connection>>>,
    active: Arc<DashMap<u64, Arc<Connection>>>,
    max_idle: usize,
    idle_timeout: Duration,
    semaphore: Arc<Semaphore>,
}

impl ConnectionPool {
    pub fn new(max_connections: usize, idle_timeout: Duration) -> Self {
        Self {
            idle: Arc::new(RwLock::new(Vec::with_capacity(max_connections / 2))),
            active: Arc::new(DashMap::new()),
            max_idle: max_connections / 2,
            idle_timeout,
            semaphore: Arc::new(Semaphore::new(max_connections)),
        }
    }
    
    pub async fn acquire(&self) -> Result<Arc<Connection>> {
        // Try to get idle connection
        {
            let mut idle = self.idle.write().await;
            while let Some(conn) = idle.pop() {
                if !conn.is_expired(self.idle_timeout) {
                    let conn = Arc::new(conn);
                    self.active.insert(conn.id, conn.clone());
                    return Ok(conn);
                }
            }
        }
        
        // Create new connection
        let _permit = self.semaphore.acquire().await?;
        let conn = Arc::new(Connection::new(rand::random()));
        self.active.insert(conn.id, conn.clone());
        Ok(conn)
    }
    
    pub async fn release(&self, conn: Arc<Connection>) {
        self.active.remove(&conn.id);
        
        let mut idle = self.idle.write().await;
        if idle.len() < self.max_idle {
            if let Ok(mut conn) = Arc::try_unwrap(conn) {
                conn.last_used = Instant::now();
                idle.push(conn);
            }
        }
    }
    
    pub fn remove(&self, id: u64) {
        self.active.remove(&id);
    }
    
    pub fn get(&self, id: u64) -> Option<Arc<Connection>> {
        self.active.get(&id).map(|entry| entry.clone())
    }
    
    pub fn close(&self, id: u64) {
        self.active.remove(&id);
    }
    
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}
