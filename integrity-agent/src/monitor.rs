use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Represents a file system event that requires integrity checking.
#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: PathBuf,
    pub event_type: EventType,
}

#[derive(Debug, Clone)]
pub enum EventType {
    Modified,
    Created,
    Deleted,
    Accessed, // For execution events
}

/// Trait for file system monitors.
#[async_trait]
pub trait Monitor: Send + Sync {
    /// Starts the monitor and returns a receiver channel for file events.
    async fn start(&mut self) -> Result<mpsc::Receiver<FileEvent>, Box<dyn std::error::Error + Send + Sync>>;

    /// Stops the monitor.
    async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Mock monitor for development/testing on non-Linux systems.
/// Generates synthetic events for testing.
pub struct MockMonitor {
    interval_secs: u64,
}

impl MockMonitor {
    pub fn new(interval_secs: u64) -> Self {
        Self { interval_secs }
    }
}

#[async_trait]
impl Monitor for MockMonitor {
    async fn start(&mut self) -> Result<mpsc::Receiver<FileEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let (tx, rx) = mpsc::channel(100);
        let interval = self.interval_secs;

        tokio::spawn(async move {
            let test_paths = vec![
                PathBuf::from("/etc/passwd"),
                PathBuf::from("/bin/ls"),
                PathBuf::from("/usr/bin/python3"),
            ];

            let mut counter = 0;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;

                if let Some(path) = test_paths.get(counter % test_paths.len()) {
                    let event = FileEvent {
                        path: path.clone(),
                        event_type: EventType::Modified,
                    };

                    if tx.send(event).await.is_err() {
                        break; // Receiver dropped
                    }
                }
                counter += 1;
            }
        });

        Ok(rx)
    }

    async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!("MockMonitor stopped");
        Ok(())
    }
}
