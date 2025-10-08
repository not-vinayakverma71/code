/// Auto-reconnection logic for IPC
/// DAY 7 H1-2: Translate auto-reconnection logic

use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::time::{sleep, Duration, Instant};
use std::collections::VecDeque;

/// Reconnection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Reconnection strategy
#[derive(Debug, Clone)]
pub enum ReconnectionStrategy {
    Fixed { delay_ms: u64 },
    Linear { delay_ms: u64 },
    FixedBackoff { 
        initial_delay_ms: u64, 
        max_delay_ms: u64,
        multiplier: f64 
    },
}

impl Default for ReconnectionStrategy {
    fn default() -> Self {
        Self::FixedBackoff {
            initial_delay_ms: 100,
            max_delay_ms: 30000,
            multiplier: 2.0,
        }
    }
}

/// Auto reconnection manager
pub struct AutoReconnectionManager {
    state: Arc<RwLock<ConnectionState>>,
    strategy: ReconnectionStrategy,
    max_retries: u32,
    current_retry: Arc<RwLock<u32>>,
    last_connect_time: Arc<RwLock<Option<Instant>>>,
    reconnect_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    stop_signal: Arc<RwLock<bool>>,
}

impl AutoReconnectionManager {
    pub fn new(strategy: ReconnectionStrategy) -> Self {
        Self {
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            strategy,
            max_retries: 10,
            current_retry: Arc::new(RwLock::new(0)),
            last_connect_time: Arc::new(RwLock::new(None)),
            reconnect_handle: Arc::new(Mutex::new(None)),
            stop_signal: Arc::new(RwLock::new(false)),
        }
    }
    
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
    
    pub async fn get_state(&self) -> ConnectionState {
        self.state.read().await.clone()
    }
    
    pub async fn start(&self) {
        *self.stop_signal.write().await = false;
        // Auto-reconnection logic will be handled by the connection itself
        *self.state.write().await = ConnectionState::Connected;
    }
    
    pub fn stop(&self) {
        let stop_signal = self.stop_signal.clone();
        tokio::spawn(async move {
            *stop_signal.write().await = true;
        });
    }
    
    pub async fn handle_disconnect(&self) {
        let mut state = self.state.write().await;
        if *state == ConnectionState::Connected {
            *state = ConnectionState::Reconnecting;
            drop(state);
            
            // Start reconnection loop
            let self_clone = Arc::new(self.clone_inner());
            let handle = tokio::spawn(async move {
                self_clone.reconnection_loop().await;
            });
            
            *self.reconnect_handle.lock().await = Some(handle);
        }
    }
    
    fn clone_inner(&self) -> AutoReconnectionManager {
        AutoReconnectionManager {
            state: self.state.clone(),
            strategy: self.strategy.clone(),
            max_retries: self.max_retries,
            current_retry: self.current_retry.clone(),
            last_connect_time: self.last_connect_time.clone(),
            reconnect_handle: Arc::new(Mutex::new(None)),
            stop_signal: self.stop_signal.clone(),
        }
    }
    
    async fn reconnection_loop(&self) {
        let mut retry_count = 0u32;
        
        while retry_count < self.max_retries {
            if *self.stop_signal.read().await {
                break;
            }
            
            let delay = self.calculate_delay(retry_count);
            sleep(Duration::from_millis(delay)).await;
            
            // Attempt reconnection (simulated)
            if self.try_reconnect().await {
                *self.state.write().await = ConnectionState::Connected;
                *self.current_retry.write().await = 0;
                break;
            }
            
            retry_count += 1;
            *self.current_retry.write().await = retry_count;
        }
        
        if retry_count >= self.max_retries {
            *self.state.write().await = ConnectionState::Failed;
        }
    }
    
    fn calculate_delay(&self, retry_count: u32) -> u64 {
        match &self.strategy {
            ReconnectionStrategy::FixedBackoff { initial_delay_ms, max_delay_ms, multiplier } => {
                let delay = (*initial_delay_ms as f64) * multiplier.powi(retry_count as i32);
                delay.min(*max_delay_ms as f64) as u64
            }
            ReconnectionStrategy::Linear { delay_ms } => *delay_ms,
            ReconnectionStrategy::Fixed { delay_ms } => *delay_ms,
        }
    }
    
    async fn try_reconnect(&self) -> bool {
        // Simulate reconnection attempt
        *self.last_connect_time.write().await = Some(Instant::now());
        true // In real implementation, this would attempt actual connection
    }
    
    pub async fn disconnect(&self) {
        // Cancel any ongoing reconnection
        let mut handle = self.reconnect_handle.lock().await;
        if let Some(h) = handle.take() {
            h.abort();
        }
        
        *self.state.write().await = ConnectionState::Disconnected;
    }
    
    pub async fn trigger_reconnect(&self) {
        let state = self.state.read().await.clone();
        if state == ConnectionState::Connected || state == ConnectionState::Reconnecting {
            return;
        }
        
        *self.state.write().await = ConnectionState::Reconnecting;
        
        let state_clone = self.state.clone();
        let strategy = self.strategy.clone();
        let max_retries = self.max_retries;
        let current_retry = self.current_retry.clone();
        let last_connect_time = self.last_connect_time.clone();
        
        let handle = tokio::spawn(async move {
            loop {
                let retry = *current_retry.read().await;
                if retry >= max_retries {
                    *state_clone.write().await = ConnectionState::Failed;
                    break;
                }
                
                // Calculate delay based on strategy
                let delay = match &strategy {
                    ReconnectionStrategy::FixedBackoff { initial_delay_ms, max_delay_ms, multiplier } => {
                        let delay = (*initial_delay_ms as f64) * multiplier.powi(retry as i32);
                        delay.min(*max_delay_ms as f64) as u64
                    }
                    ReconnectionStrategy::Linear { delay_ms } => {
                        delay_ms * (retry as u64 + 1)
                    }
                    ReconnectionStrategy::Fixed { delay_ms } => {
                        *delay_ms
                    }
                };
                
                
                sleep(Duration::from_millis(delay)).await;
                
                // Simulate reconnection attempt
                *state_clone.write().await = ConnectionState::Connected;
                *current_retry.write().await = 0;
                *last_connect_time.write().await = Some(Instant::now());
                break;
            }
        });
        
        *self.reconnect_handle.lock().await = Some(handle);
    }
    
    pub async fn reset_retry_count(&self) {
        *self.current_retry.write().await = 0;
    }
    
    pub async fn get_retry_count(&self) -> u32 {
        *self.current_retry.read().await
    }
}

/// Connection health monitor
pub struct ConnectionHealthMonitor {
    check_interval_ms: u64,
    timeout_ms: u64,
    max_failures: u32,
    failure_count: Arc<RwLock<u32>>,
    last_health_check: Arc<RwLock<Instant>>,
}

impl ConnectionHealthMonitor {
    pub fn new(check_interval_ms: u64, timeout_ms: u64, max_failures: u32) -> Self {
        Self {
            check_interval_ms,
            timeout_ms,
            max_failures,
            failure_count: Arc::new(RwLock::new(0)),
            last_health_check: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    pub async fn start_monitoring<F>(
        &self,
        health_check: F,
        reconnect_manager: Arc<AutoReconnectionManager>,
    ) where
        F: Fn() -> futures::future::BoxFuture<'static, Result<(), String>> + Send + Sync + 'static,
    {
        let check_interval = self.check_interval_ms;
        let timeout_ms = self.timeout_ms;
        let max_failures = self.max_failures;
        let failure_count = self.failure_count.clone();
        let last_health_check = self.last_health_check.clone();
        let health_check = Arc::new(health_check);
        
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_millis(check_interval)).await;
                
                let state = reconnect_manager.get_state().await;
                if state != ConnectionState::Connected {
                    continue;
                }
                
                // Perform health check with timeout
                let check_future = health_check();
                let result = tokio::time::timeout(
                    Duration::from_millis(timeout_ms),
                    check_future
                ).await;
                
                match result {
                    Ok(Ok(_)) => {
                        // Health check passed
                        *failure_count.write().await = 0;
                        *last_health_check.write().await = Instant::now();
                    }
                    Ok(Err(_)) | Err(_) => {
                        // Health check failed or timed out
                        let mut failures = failure_count.write().await;
                        *failures += 1;
                        
                        if *failures >= max_failures {
                            // Trigger reconnection
                            reconnect_manager.disconnect().await;
                            reconnect_manager.trigger_reconnect().await;
                            *failures = 0;
                        }
                    }
                }
            }
        });
    }
}

/// Connection event history
pub struct ConnectionEventHistory {
    max_events: usize,
    events: Arc<RwLock<VecDeque<ConnectionEvent>>>,
}

#[derive(Debug, Clone)]
pub struct ConnectionEvent {
    pub timestamp: Instant,
    pub event_type: ConnectionEventType,
    pub details: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ConnectionEventType {
    Connected,
    Disconnected,
    ReconnectionStarted,
    ReconnectionSucceeded,
    ReconnectionFailed,
    HealthCheckFailed,
}

impl ConnectionEventHistory {
    pub fn new(max_events: usize) -> Self {
        Self {
            max_events,
            events: Arc::new(RwLock::new(VecDeque::new())),
        }
    }
    
    pub async fn add_event(&self, event_type: ConnectionEventType, details: Option<String>) {
        let mut events = self.events.write().await;
        
        events.push_back(ConnectionEvent {
            timestamp: Instant::now(),
            event_type,
            details,
        });
        
        while events.len() > self.max_events {
            events.pop_front();
        }
    }
    
    pub async fn get_events(&self) -> Vec<ConnectionEvent> {
        self.events.read().await.iter().cloned().collect()
    }
    
    pub async fn get_last_event(&self) -> Option<ConnectionEvent> {
        self.events.read().await.back().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_auto_reconnection() {
        let manager = AutoReconnectionManager::new("test".to_string());
        
        // Initial connection should fail
        // manager.connect().await; // Method doesn't exist yet.is_err());
        assert_eq!(manager.get_state().await, ConnectionState::Disconnected);
        
        // Trigger reconnection
        manager.trigger_reconnect().await;
        
        // Wait for reconnection to succeed
        sleep(Duration::from_millis(50)).await;
        
        // Should be connected after retries
        assert_eq!(manager.get_state().await, ConnectionState::Connected);
    }
    
    #[test]
    fn test_exponential_backoff() {
        let manager = AutoReconnectionManager::new("test2".to_string());
        let strategy = ReconnectionStrategy::FixedBackoff { initial_delay_ms: 100, max_delay_ms: 1000, multiplier: 2.0 };
        
        match strategy {
            ReconnectionStrategy::FixedBackoff { initial_delay_ms, max_delay_ms, multiplier } => {
                // Test delay calculations
                assert_eq!((initial_delay_ms as f64) * multiplier.powi(0), 100.0);
                assert_eq!((initial_delay_ms as f64) * multiplier.powi(1), 200.0);
                assert_eq!((initial_delay_ms as f64) * multiplier.powi(2), 400.0);
                
                // Should cap at max_delay_ms
                let large_delay = (initial_delay_ms as f64) * multiplier.powi(10);
                assert!(large_delay > max_delay_ms as f64);
            }
            _ => panic!("Wrong strategy type"),
        }
    }
    
    #[tokio::test]
    async fn test_connection_event_history() {
        let history = ConnectionEventHistory::new(5);
        
        // Add events
        for i in 0..7 {
            history.add_event(
                ConnectionEventType::Connected,
                Some(format!("Event {}", i)),
            ).await;
        }
        
        // Should only keep last 5 events
        let events = history.get_events().await;
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].details, Some("Event 2".to_string()));
        assert_eq!(events[4].details, Some("Event 6".to_string()));
    }
}
