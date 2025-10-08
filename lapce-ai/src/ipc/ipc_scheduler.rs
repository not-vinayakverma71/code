/// IPC Message Scheduler with semaphore-based flow control
/// Implements permit-based scheduling similar to HTTP/2 multiplexer

use std::sync::Arc;
use tokio::sync::{Semaphore, SemaphorePermit, mpsc};
use anyhow::{Result, bail};
use std::collections::VecDeque;
use dashmap::DashMap;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Message with priority for scheduling
#[derive(Debug, Clone)]
pub struct ScheduledMessage {
    pub connection_id: u64,
    pub message: Vec<u8>,
    pub priority: MessagePriority,
    pub created_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Scheduler configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Max concurrent messages in flight
    pub max_concurrent_messages: usize,
    
    /// Max messages queued per connection
    pub max_queued_per_connection: usize,
    
    /// Message timeout
    pub message_timeout: Duration,
    
    /// Enable fair scheduling across connections
    pub fair_scheduling: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_messages: 100,
            max_queued_per_connection: 50,
            message_timeout: Duration::from_secs(30),
            fair_scheduling: true,
        }
    }
}

/// IPC Message Scheduler
pub struct IpcScheduler {
    /// Semaphore for controlling concurrent messages
    semaphore: Arc<Semaphore>,
    
    /// Per-connection message queues
    connection_queues: Arc<DashMap<u64, ConnectionQueue>>,
    
    /// Global priority queue for fair scheduling
    priority_queue: Arc<parking_lot::Mutex<VecDeque<ScheduledMessage>>>,
    
    /// Configuration
    config: SchedulerConfig,
    
    /// Metrics
    messages_scheduled: Arc<std::sync::atomic::AtomicU64>,
    messages_completed: Arc<std::sync::atomic::AtomicU64>,
    messages_dropped: Arc<std::sync::atomic::AtomicU64>,
}

struct ConnectionQueue {
    queue: VecDeque<ScheduledMessage>,
    active_permits: usize,
    last_scheduled: Instant,
}

impl IpcScheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.max_concurrent_messages)),
            connection_queues: Arc::new(DashMap::new()),
            priority_queue: Arc::new(parking_lot::Mutex::new(VecDeque::new())),
            config,
            messages_scheduled: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            messages_completed: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            messages_dropped: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    /// Schedule a message for sending
    pub async fn schedule_message(
        &self,
        connection_id: u64,
        message: Vec<u8>,
        priority: MessagePriority,
    ) -> Result<MessageHandle> {
        let msg = ScheduledMessage {
            connection_id,
            message,
            priority,
            created_at: Instant::now(),
        };
        
        // Check if connection queue is full
        let mut entry = self.connection_queues.entry(connection_id)
            .or_insert_with(|| ConnectionQueue {
                queue: VecDeque::new(),
                active_permits: 0,
                last_scheduled: Instant::now(),
            });
        
        if entry.queue.len() >= self.config.max_queued_per_connection {
            self.messages_dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            bail!("Connection queue full");
        }
        
        // Add to queue based on priority
        match priority {
            MessagePriority::Critical => {
                entry.queue.push_front(msg.clone());
            }
            MessagePriority::High => {
                // Insert after critical messages
                let mut pos = 0;
                for (i, queued) in entry.queue.iter().enumerate() {
                    if queued.priority < MessagePriority::High {
                        pos = i;
                        break;
                    }
                }
                entry.queue.insert(pos, msg.clone());
            }
            _ => {
                entry.queue.push_back(msg.clone());
            }
        }
        
        // Also add to global queue if fair scheduling
        if self.config.fair_scheduling {
            self.priority_queue.lock().push_back(msg);
        }
        
        self.messages_scheduled.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        // Try to acquire permit
        let permit = self.try_acquire_permit().await?;
        
        Ok(MessageHandle {
            connection_id,
            permit: Some(permit),
            scheduler: self.clone_ref(),
        })
    }
    
    /// Try to acquire a semaphore permit
    async fn try_acquire_permit(&self) -> Result<tokio::sync::OwnedSemaphorePermit> {
        match timeout(
            self.config.message_timeout,
            self.semaphore.clone().acquire_owned()
        ).await {
            Ok(Ok(permit)) => Ok(permit),
            Ok(Err(_)) => bail!("Semaphore closed"),
            Err(_) => bail!("Timeout waiting for permit"),
        }
    }
    
    /// Get next message to process (fair scheduling)
    pub fn get_next_message(&self) -> Option<ScheduledMessage> {
        if self.config.fair_scheduling {
            // Round-robin across connections
            self.priority_queue.lock().pop_front()
        } else {
            // Process by priority only
            let mut best_msg = None;
            let mut best_priority = MessagePriority::Low;
            
            for mut entry in self.connection_queues.iter_mut() {
                if let Some(msg) = entry.queue.front() {
                    if msg.priority >= best_priority {
                        best_priority = msg.priority;
                        best_msg = Some(entry.key().clone());
                    }
                }
            }
            
            if let Some(conn_id) = best_msg {
                self.connection_queues.get_mut(&conn_id)
                    .and_then(|mut entry| entry.queue.pop_front())
            } else {
                None
            }
        }
    }
    
    /// Mark message as completed
    pub fn complete_message(&self, connection_id: u64) {
        if let Some(mut entry) = self.connection_queues.get_mut(&connection_id) {
            entry.active_permits = entry.active_permits.saturating_sub(1);
        }
        self.messages_completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    
    /// Get queue depth for a connection
    pub fn get_queue_depth(&self, connection_id: u64) -> usize {
        self.connection_queues.get(&connection_id)
            .map(|entry| entry.queue.len())
            .unwrap_or(0)
    }
    
    /// Get scheduler statistics
    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            messages_scheduled: self.messages_scheduled.load(std::sync::atomic::Ordering::Relaxed),
            messages_completed: self.messages_completed.load(std::sync::atomic::Ordering::Relaxed),
            messages_dropped: self.messages_dropped.load(std::sync::atomic::Ordering::Relaxed),
            available_permits: self.semaphore.available_permits() as u64,
            total_queued: self.connection_queues.iter()
                .map(|entry| entry.queue.len())
                .sum::<usize>() as u64,
        }
    }
    
    fn clone_ref(&self) -> Arc<Self> {
        // In real implementation, would store Arc<Self> internally
        Arc::new(Self {
            semaphore: self.semaphore.clone(),
            connection_queues: self.connection_queues.clone(),
            priority_queue: self.priority_queue.clone(),
            config: self.config.clone(),
            messages_scheduled: self.messages_scheduled.clone(),
            messages_completed: self.messages_completed.clone(),
            messages_dropped: self.messages_dropped.clone(),
        })
    }
}

/// Handle for a scheduled message with RAII permit management
pub struct MessageHandle {
    connection_id: u64,
    permit: Option<tokio::sync::OwnedSemaphorePermit>,
    scheduler: Arc<IpcScheduler>,
}

impl MessageHandle {
    /// Release the permit early
    pub fn release(mut self) {
        self.permit = None;
        self.scheduler.complete_message(self.connection_id);
    }
}

impl Drop for MessageHandle {
    fn drop(&mut self) {
        if self.permit.is_some() {
            self.scheduler.complete_message(self.connection_id);
        }
    }
}

#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub messages_scheduled: u64,
    pub messages_completed: u64,
    pub messages_dropped: u64,
    pub available_permits: u64,
    pub total_queued: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_message_scheduling() {
        let config = SchedulerConfig {
            max_concurrent_messages: 2,
            max_queued_per_connection: 10,
            ..Default::default()
        };
        
        let scheduler = IpcScheduler::new(config);
        
        // Schedule messages
        let handle1 = scheduler.schedule_message(1, vec![1, 2, 3], MessagePriority::Normal).await.unwrap();
        let handle2 = scheduler.schedule_message(1, vec![4, 5, 6], MessagePriority::High).await.unwrap();
        
        // Should block on third message
        let scheduler_clone = Arc::new(scheduler);
        let scheduler2 = scheduler_clone.clone();
        
        let task = tokio::spawn(async move {
            scheduler2.schedule_message(1, vec![7, 8, 9], MessagePriority::Normal).await
        });
        
        // Verify we're blocked
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!task.is_finished());
        
        // Release one permit
        handle1.release();
        
        // Now third message should proceed
        let result = tokio::time::timeout(Duration::from_secs(1), task).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_priority_ordering() {
        let scheduler = IpcScheduler::new(SchedulerConfig::default());
        
        // Add messages with different priorities
        scheduler.schedule_message(1, vec![1], MessagePriority::Low).await.unwrap();
        scheduler.schedule_message(1, vec![2], MessagePriority::Critical).await.unwrap();
        scheduler.schedule_message(1, vec![3], MessagePriority::Normal).await.unwrap();
        scheduler.schedule_message(1, vec![4], MessagePriority::High).await.unwrap();
        
        // Critical should be first
        let msg = scheduler.get_next_message().unwrap();
        assert_eq!(msg.priority, MessagePriority::Critical);
    }
}
