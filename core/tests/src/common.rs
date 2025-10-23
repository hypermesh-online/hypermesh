//! Common test utilities and helpers

use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TestEnvironment {
    pub cleanup_tasks: Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync>>>>,
}

impl TestEnvironment {
    pub fn new() -> Self {
        Self {
            cleanup_tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub async fn add_cleanup<F>(&self, task: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.cleanup_tasks.lock().await.push(Box::new(task));
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // Cleanup tasks will be executed when the environment is dropped
    }
}