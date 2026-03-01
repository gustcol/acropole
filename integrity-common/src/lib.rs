use chrono::{DateTime, Utc};
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

/// Health status of an agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentHealth {
    Healthy,
    Warning,
    Critical,
}

/// Agent information for dashboard display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub id: String,
    pub hostname: String,
    pub ip_address: String,
    pub status: AgentHealth,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub alert_count: u64,
    pub image_id: String,
}

/// Heartbeat sent by agents periodically
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Heartbeat {
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub status: AgentHealth,
    pub image_id: String,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Alert generated when anomalies are detected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub id: String,
    pub agent_id: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub severity: AlertSeverity,
}

/// Dashboard summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub total_agents: u64,
    pub healthy_agents: u64,
    pub warning_agents: u64,
    pub critical_agents: u64,
    pub alerts_today: u64,
    pub alerts_this_week: u64,
    pub alerts_this_month: u64,
}

/// Request body for agent self-registration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAgent {
    pub hostname: String,
    pub ip_address: String,
    pub image_id: String,
}

/// Registration response with assigned agent_id
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAgentResponse {
    pub agent_id: String,
}

/// Request body for posting alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostAlert {
    pub agent_id: String,
    pub message: String,
    pub severity: AlertSeverity,
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
