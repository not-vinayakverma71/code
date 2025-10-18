use std::path::PathBuf;
use tokio::sync::mpsc;
use notify::{Watcher, RecursiveMode, recommended_watcher, Event};
use anyhow::Result;

pub struct FileWatcher {
    tx: mpsc::UnboundedSender<FileEvent>,
    rx: Option<mpsc::UnboundedReceiver<FileEvent>>,
}

#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: PathBuf,
    pub kind: String,
}

impl FileWatcher {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        Ok(Self {
            tx,
            rx: Some(rx),
        })
    }
    
    pub async fn watch(&mut self, paths: Vec<PathBuf>) -> Result<mpsc::UnboundedReceiver<FileEvent>> {
        let tx = self.tx.clone();
        
        let mut watcher = recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                for path in event.paths {
                    let _ = tx.send(FileEvent {
                        path,
                        kind: format!("{:?}", event.kind),
                    });
                }
            }
        })?;
        
        for path in paths {
            watcher.watch(&path, RecursiveMode::Recursive)?;
        }
        
        // Keep watcher alive
        std::mem::forget(watcher);
        
        self.rx.take().ok_or_else(|| anyhow::anyhow!("Receiver already taken"))
    }
}
