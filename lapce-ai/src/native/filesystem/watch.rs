use std::sync::Arc;
use std::path::PathBuf;
use std::collections::VecDeque;
use anyhow::Result;
use tokio::sync::{mpsc, Mutex};
use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event, Config};
use futures::stream::{Stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;
use serde_json::{json, Value};

// FileWatcher from lines 204-224
#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: PathBuf,
    pub kind: String,
    pub timestamp: std::time::SystemTime,
}
pub struct FileWatcher {
    watcher: Arc<Mutex<RecommendedWatcher>>,
    events: Arc<Mutex<VecDeque<FileEvent>>>,
}

impl FileWatcher {
    pub fn new() -> Result<Self> {
        let (tx, _rx) = mpsc::channel(100);
        
        let watcher = notify::recommended_watcher(move |event: Result<Event, _>| {
            if let Ok(event) = event {
                let _ = tx.blocking_send(event);
            }
        })?;
        Ok(Self {
            watcher: Arc::new(Mutex::new(watcher)),
            events: Arc::new(Mutex::new(VecDeque::new())),
        })
    }
    
    pub async fn watch(&self, paths: Vec<PathBuf>) -> Result<Box<dyn Stream<Item = FileEvent> + Send + Unpin>> {
        let (tx, rx) = mpsc::channel(100);
        let tx_clone = tx.clone();
        
        let mut watcher = notify::recommended_watcher(move |event: Result<Event, _>| {
            if let Ok(event) = event {
                for path in event.paths {
                    let file_event = FileEvent {
                        path: path.clone(),
                        kind: format!("{:?}", event.kind),
                        timestamp: std::time::SystemTime::now(),
                    };
                    let _ = tx_clone.blocking_send(file_event);
                }
            }
        })?;
        
        for path in paths {
            watcher.watch(&path, RecursiveMode::Recursive)?;
        }
        
        // Store watcher to keep it alive
        *self.watcher.lock().await = watcher;
        
        Ok(Box::new(ReceiverStream::new(rx)))
    }
    
    pub async fn stop(&self) -> Result<()> {
        // Watcher stops automatically when dropped
        Ok(())
    }
    
    pub async fn get_recent_events(&self, count: usize) -> Vec<FileEvent> {
        let events = self.events.lock().await;
        events.iter().rev().take(count).cloned().collect()
    }
}
