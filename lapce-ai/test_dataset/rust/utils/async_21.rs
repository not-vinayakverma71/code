use tokio::sync::RwLock;
use std::sync::Arc;

pub struct AsyncProcessor_21 {
    data: Arc<RwLock<Vec<String>>>,
}

impl AsyncProcessor_21 {
    pub async fn process_batch(&self, items: Vec<String>) -> Result<()> {
        let mut data = self.data.write().await;
        for item in items {
            data.push(item);
        }
        Ok(())
    }
    
    pub async fn get_results(&self) -> Vec<String> {
        let data = self.data.read().await;
        data.clone()
    }
}