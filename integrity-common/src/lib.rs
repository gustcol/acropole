use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a single file's integrity data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileIntegrityEntry {
    /// Relative to root, e.g., "/etc/passwd"
    pub path: String,
    /// Hex encoded SHA512 hash
    pub sha512: String,
    /// Unix permissions (e.g., 0o644)
    pub mode: u32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
}

/// Represents the full baseline for an image.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Baseline {
    /// Unique identifier (e.g., "ubuntu-2204-hardened-v1")
    pub image_id: String,
    /// ISO8601 creation time
    pub timestamp: String,
    /// List of file integrity entries
    pub entries: Vec<FileIntegrityEntry>,
}

/// Custom error types for the integrity system.
#[derive(Debug, thiserror::Error)]
pub enum IntegrityError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Walkdir error: {0}")]
    Walkdir(String),
    #[error("Baseline not found: {0}")]
    BaselineNotFound(String),
    #[error("Storage error: {0}")]
    Storage(String),
}

/// Result type alias for the integrity system.
pub type Result<T> = std::result::Result<T, IntegrityError>;

impl fmt::Display for FileIntegrityEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FileIntegrityEntry {{ path: {}, sha512: {}, mode: {:o}, uid: {}, gid: {} }}",
            self.path, self.sha512, self.mode, self.uid, self.gid
        )
    }
}

impl fmt::Display for Baseline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Baseline {{ image_id: {}, timestamp: {}, entries: {} files }}",
            self.image_id,
            self.timestamp,
            self.entries.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_integrity_entry_display() {
        let entry = FileIntegrityEntry {
            path: "/etc/passwd".to_string(),
            sha512: "abc123".to_string(),
            mode: 0o644,
            uid: 0,
            gid: 0,
        };
        let display = format!("{}", entry);
        assert!(display.contains("/etc/passwd"));
        assert!(display.contains("abc123"));
        assert!(display.contains("644"));
    }

    #[test]
    fn test_baseline_display() {
        let baseline = Baseline {
            image_id: "test-image".to_string(),
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            entries: vec![
                FileIntegrityEntry {
                    path: "/etc/passwd".to_string(),
                    sha512: "abc123".to_string(),
                    mode: 0o644,
                    uid: 0,
                    gid: 0,
                },
                FileIntegrityEntry {
                    path: "/etc/shadow".to_string(),
                    sha512: "def456".to_string(),
                    mode: 0o600,
                    uid: 0,
                    gid: 0,
                },
            ],
        };
        let display = format!("{}", baseline);
        assert!(display.contains("test-image"));
        assert!(display.contains("2023-01-01T00:00:00Z"));
        assert!(display.contains("2 files"));
    }
}
