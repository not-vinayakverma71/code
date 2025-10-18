/// Backpressure handling for message processing
/// DAY 7 H3-4: Port backpressure handling

use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore, mpsc};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Backpressure strategy
#[derive(Debug, Clone)]
pub enum BackpressureStrategy {
    /// Drop oldest messages when buffer is full
    DropOldest,
    /// Drop newest messages when buffer is full
    DropNewest,
    /// Block until space is available
    Block,
    /// Apply exponential backoff
    ExponentialBackoff {
        initial_delay_ms: u64,
        max_delay_ms: u64,
        multiplier: f64,
    },
}

/// Backpressure metrics
#[derive(Debug, Clone, Default)]
pub struct BackpressureMetrics {
    pub messages_processed: u64,
    pub messages_dropped: u64,
    pub messages_delayed: u64,
    pub buffer_overflows: u64,
    pub average_processing_time_ms: f64,
    pub peak_buffer_size: usize,
}

/// Backpressure controller
pub struct BackpressureController {
    strategy: BackpressureStrategy,
    max_buffer_size: usize,
    buffer: Arc<RwLock<VecDeque<Message>>>,
    metrics: Arc<RwLock<BackpressureMetrics>>,
    semaphore: Arc<Semaphore>,
    processing_times: Arc<RwLock<VecDeque<Duration>>>,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub payload: Vec<u8>,
    pub timestamp: Instant,
    pub priority: MessagePriority,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl BackpressureController {
    pub fn new(strategy: BackpressureStrategy, max_buffer_size: usize) -> Self {
        Self {
            strategy,
            max_buffer_size,
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(max_buffer_size))),
            metrics: Arc::new(RwLock::new(BackpressureMetrics::default())),
            semaphore: Arc::new(Semaphore::new(max_buffer_size)),
            processing_times: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
        }
    }
    
    /// Submit a message for processing
    pub async fn submit(&self, message: Message) -> Result<(), String> {
        let mut buffer = self.buffer.write().await;
        let mut metrics = self.metrics.write().await;
        
        // Update peak buffer size
        if buffer.len() > metrics.peak_buffer_size {
            metrics.peak_buffer_size = buffer.len();
        }
        
        // Check if buffer is full
        if buffer.len() >= self.max_buffer_size {
            metrics.buffer_overflows += 1;
            
            match &self.strategy {
                BackpressureStrategy::DropOldest => {
                    buffer.pop_front();
                    metrics.messages_dropped += 1;
                    buffer.push_back(message);
                    Ok(())
                }
                BackpressureStrategy::DropNewest => {
                    metrics.messages_dropped += 1;
                    Err("Buffer full, message dropped".to_string())
                }
                BackpressureStrategy::Block => {
                    // Release locks and wait for space
                    drop(buffer);
                    drop(metrics);
                    
                    // Acquire semaphore permit
                    let _permit = self.semaphore.acquire().await
                        .map_err(|e| format!("Failed to acquire semaphore: {}", e))?;
                    
                    // Re-acquire locks and add message
                    let mut buffer = self.buffer.write().await;
                    buffer.push_back(message);
                    Ok(())
                }
                BackpressureStrategy::ExponentialBackoff { initial_delay_ms, max_delay_ms, multiplier } => {
                    drop(buffer);
                    drop(metrics);
                    
                    // Apply backoff
                    let mut delay = *initial_delay_ms;
                    let mut retry_count = 0;
                    
                    loop {
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        
                        let mut buffer = self.buffer.write().await;
                        if buffer.len() < self.max_buffer_size {
                            buffer.push_back(message);
                            self.metrics.write().await.messages_delayed += 1;
                            return Ok(());
                        }
                        
                        retry_count += 1;
                        delay = ((delay as f64) * multiplier).min(*max_delay_ms as f64) as u64;
                        
                        if retry_count > 10 {
                            self.metrics.write().await.messages_dropped += 1;
                            return Err("Max retries exceeded".to_string());
                        }
                    }
                }
            }
        } else {
            buffer.push_back(message);
            Ok(())
        }
    }
    
    /// Process next message from buffer
    pub async fn process_next<F>(&self, processor: F) -> Option<()>
    where
        F: FnOnce(Message) -> futures::future::BoxFuture<'static, Result<(), String>>,
    {
        let message = {
            let mut buffer = self.buffer.write().await;
            buffer.pop_front()
        };
        
        if let Some(msg) = message {
            let start_time = Instant::now();
            
            // Process message
            match processor(msg).await {
                Ok(_) => {
                    let processing_time = start_time.elapsed();
                    
                    // Update metrics
                    let mut metrics = self.metrics.write().await;
                    metrics.messages_processed += 1;
                    
                    // Update average processing time
                    let mut times = self.processing_times.write().await;
                    times.push_back(processing_time);
                    if times.len() > 100 {
                        times.pop_front();
                    }
                    
                    let avg_ms = times.iter()
                        .map(|d| d.as_millis() as f64)
                        .sum::<f64>() / times.len() as f64;
                    metrics.average_processing_time_ms = avg_ms;
                    
                    // Release semaphore if using blocking strategy
                    if matches!(self.strategy, BackpressureStrategy::Block) {
                        self.semaphore.add_permits(1);
                    }
                    
                    Some(())
                }
                Err(_) => None
            }
        } else {
            None
        }
    }
    
    /// Get current buffer size
    pub async fn buffer_size(&self) -> usize {
        self.buffer.read().await.len()
    }
    
    /// Get metrics
    pub async fn get_metrics(&self) -> BackpressureMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Clear buffer
    pub async fn clear_buffer(&self) {
        self.buffer.write().await.clear();
    }
}

/// Adaptive backpressure controller that adjusts strategy based on load
pub struct AdaptiveBackpressureController {
    controller: BackpressureController,
    load_threshold_high: f64,
    load_threshold_low: f64,
    monitoring_interval_ms: u64,
}

impl AdaptiveBackpressureController {
    pub fn new(
        initial_strategy: BackpressureStrategy,
        max_buffer_size: usize,
        load_threshold_high: f64,
        load_threshold_low: f64,
    ) -> Self {
        Self {
            controller: BackpressureController::new(initial_strategy, max_buffer_size),
            load_threshold_high,
            load_threshold_low,
            monitoring_interval_ms: 1000,
        }
    }
    
    /// Start adaptive monitoring
    pub async fn start_monitoring(&self) {
        let monitoring_interval = self.monitoring_interval_ms;
        let mut controller = self.controller.clone();
        let high_threshold = self.load_threshold_high;
        let low_threshold = self.load_threshold_low;
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(monitoring_interval)).await;
                
                let metrics = controller.get_metrics().await;
                let buffer_size = controller.buffer_size().await;
                let load_factor = buffer_size as f64 / controller.max_buffer_size as f64;
                
                // Adjust strategy based on load
                if load_factor > high_threshold {
                    // High load - switch to dropping strategy
                    controller.strategy = BackpressureStrategy::DropOldest;
                } else if load_factor < low_threshold {
                    // Low load - switch to blocking strategy
                    controller.strategy = BackpressureStrategy::Block;
                }
                
                // Log metrics
                println!("Backpressure metrics - Load: {:.2}%, Processed: {}, Dropped: {}, Avg time: {:.2}ms",
                    load_factor * 100.0,
                    metrics.messages_processed,
                    metrics.messages_dropped,
                    metrics.average_processing_time_ms
                );
            }
        });
    }
}

/// Message channel with backpressure
pub struct BackpressureChannel<T> {
    sender: mpsc::Sender<T>,
    receiver: Arc<RwLock<mpsc::Receiver<T>>>,
    max_capacity: usize,
}

impl<T> BackpressureChannel<T> 
where
    T: Send + 'static,
{
    pub fn new(max_capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(max_capacity);
        Self {
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
            max_capacity,
        }
    }
    
    /// Send with backpressure
    pub async fn send(&self, value: T) -> Result<(), String> {
        self.sender.send(value).await
            .map_err(|e| format!("Channel send failed: {}", e))
    }
    
    /// Try send without blocking
    pub fn try_send(&self, value: T) -> Result<(), String> {
        self.sender.try_send(value)
            .map_err(|e| format!("Channel try_send failed: {}", e))
    }
    
    /// Receive from channel
    pub async fn recv(&self) -> Option<T> {
        self.receiver.write().await.recv().await
    }
    
    /// Get channel capacity
    pub fn capacity(&self) -> usize {
        self.max_capacity
    }
}

// Clone implementation for BackpressureController
impl Clone for BackpressureController {
    fn clone(&self) -> Self {
        Self {
            strategy: self.strategy.clone(),
            max_buffer_size: self.max_buffer_size,
            buffer: self.buffer.clone(),
            metrics: self.metrics.clone(),
            semaphore: self.semaphore.clone(),
            processing_times: self.processing_times.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_drop_oldest_strategy() {
        let controller = BackpressureController::new(
            BackpressureStrategy::DropOldest,
            3
        );
        
        // Fill buffer
        for i in 0..4 {
            let msg = Message {
                id: format!("msg_{}", i),
                payload: vec![],
                timestamp: Instant::now(),
                priority: MessagePriority::Normal,
            };
            controller.submit(msg).await.unwrap();
        }
        
        let metrics = controller.get_metrics().await;
        assert_eq!(metrics.messages_dropped, 1);
        assert_eq!(controller.buffer_size().await, 3);
    }
    
    #[tokio::test]
    async fn test_drop_newest_strategy() {
        let controller = BackpressureController::new(
            BackpressureStrategy::DropNewest,
            3
        );
        
        // Fill buffer
        for i in 0..3 {
            let msg = Message {
                id: format!("msg_{}", i),
                payload: vec![],
                timestamp: Instant::now(),
                priority: MessagePriority::Normal,
            };
            controller.submit(msg).await.unwrap();
        }
        
        // This should be dropped
        let msg = Message {
            id: "msg_3".to_string(),
            payload: vec![],
            timestamp: Instant::now(),
            priority: MessagePriority::Normal,
        };
        
        assert!(controller.submit(msg).await.is_err());
        
        let metrics = controller.get_metrics().await;
        assert_eq!(metrics.messages_dropped, 1);
    }
    
    #[tokio::test]
    async fn test_backpressure_channel() {
        let channel: BackpressureChannel<i32> = BackpressureChannel::new(3);
        
        // Send some values
        channel.send(1).await.unwrap();
        channel.send(2).await.unwrap();
        channel.send(3).await.unwrap();
        
        // Receive values
        assert_eq!(channel.recv().await, Some(1));
        assert_eq!(channel.recv().await, Some(2));
        assert_eq!(channel.recv().await, Some(3));
    }
}
