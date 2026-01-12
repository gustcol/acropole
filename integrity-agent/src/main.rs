mod monitor;
#[cfg(target_os = "linux")]
mod fanotify_monitor;

use clap::Parser;
use integrity_common::{Baseline, FileIntegrityEntry, Result, IntegrityError};
use monitor::Monitor;
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tracing::{info, error, warn};
use walkdir::{DirEntry, WalkDir};

#[derive(Parser, Debug)]
#[command(name = "integrity-agent")]
#[command(about = "Golden Image Integrity Agent", long_about = None)]
struct Args {
    #[arg(long, default_value = "/")]
    scan_path: PathBuf,

    #[arg(long)]
    image_id: String,

    #[arg(long, default_value = "http://localhost:8080")]
    metadata_url: String,

    #[arg(long, value_enum, default_value = "scan")]
    mode: RunMode,

    #[arg(long, value_delimiter = ',', default_value = "/bin,/sbin,/usr/bin,/usr/sbin,/etc")]
    watch_paths: Vec<PathBuf>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum RunMode {
    /// Run a one-time scan and compare with baseline
    Scan,
    /// Monitor filesystem events in real-time
    Monitor,
}

/// Directories to exclude from scanning
const EXCLUDED_DIRS: &[&str] = &[
    "/proc", "/sys", "/dev", "/run", "/tmp", "/var/tmp", "/var/log",
];

fn should_exclude(entry: &DirEntry) -> bool {
    let path = entry.path();

    // Skip if it's a directory and matches excluded paths
    if path.is_dir() {
        let path_str = path.to_string_lossy();
        return EXCLUDED_DIRS.iter().any(|&excluded| path_str.starts_with(excluded));
    }

    // Skip special files (devices, sockets, etc.)
    if let Ok(metadata) = entry.metadata() {
        use std::os::unix::fs::FileTypeExt;
        let file_type = metadata.file_type();
        if file_type.is_block_device() || file_type.is_char_device() || file_type.is_fifo() || file_type.is_socket() {
            return true;
        }
    }

    false
}

fn compute_sha512(path: &Path) -> Result<String> {
    let mut hasher = Sha512::new();
    let mut file = fs::File::open(path)?;
    std::io::copy(&mut file, &mut hasher)?;
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

fn scan_filesystem(root_path: &Path) -> Result<HashMap<String, FileIntegrityEntry>> {
    info!("Starting filesystem scan from: {:?}", root_path);

    let mut entries = HashMap::new();
    let walker = WalkDir::new(root_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !should_exclude(e));

    for entry in walker {
        let entry = entry.map_err(|e| IntegrityError::Walkdir(e.to_string()))?;
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Get relative path from root
        let relative_path = path.strip_prefix(root_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Skip if path is empty (shouldn't happen, but safety check)
        if relative_path.is_empty() {
            continue;
        }

        match entry.metadata() {
            Ok(metadata) => {
                match compute_sha512(path) {
                    Ok(sha512) => {
                        let file_entry = FileIntegrityEntry {
                            path: relative_path.clone(),
                            sha512,
                            mode: metadata.mode() & 0o7777, // Get permission bits
                            uid: metadata.uid(),
                            gid: metadata.gid(),
                        };
                        entries.insert(relative_path, file_entry);

                        if entries.len() % 1000 == 0 {
                            info!("Scanned {} files...", entries.len());
                        }
                    }
                    Err(e) => {
                        warn!("Failed to hash file {:?}: {}", path, e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to get metadata for {:?}: {}", path, e);
            }
        }
    }

    info!("Scan complete. Found {} files", entries.len());
    Ok(entries)
}

async fn fetch_baseline(metadata_url: &str, image_id: &str) -> Result<Baseline> {
    let client = reqwest::Client::new();
    let url = format!("{}/baselines/{}", metadata_url, image_id);

    info!("Fetching baseline from: {}", url);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| integrity_common::IntegrityError::Storage(e.to_string()))?;

    if response.status().is_success() {
        let baseline: Baseline = response
            .json()
            .await
            .map_err(|e| integrity_common::IntegrityError::Storage(e.to_string()))?;
        info!("Baseline fetched successfully ({} files)", baseline.entries.len());
        Ok(baseline)
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        error!("Failed to fetch baseline: {}", error_text);
        Err(integrity_common::IntegrityError::BaselineNotFound(format!("Fetch failed: {}", error_text)))
    }
}

fn compare_filesystems(baseline: &Baseline, current: &HashMap<String, FileIntegrityEntry>) -> Vec<String> {
    let mut anomalies = Vec::new();
    let baseline_map: HashMap<String, &FileIntegrityEntry> = baseline.entries
        .iter()
        .map(|entry| (entry.path.clone(), entry))
        .collect();

    // Check for modified/deleted files
    for (path, baseline_entry) in &baseline_map {
        match current.get(path) {
            Some(current_entry) => {
                // File exists, check for modifications
                if current_entry.sha512 != baseline_entry.sha512 {
                    anomalies.push(format!("MODIFIED: {} (hash mismatch: {} != {})",
                        path, baseline_entry.sha512, current_entry.sha512));
                }
                if current_entry.mode != baseline_entry.mode {
                    anomalies.push(format!("PERMISSION_CHANGED: {} ({} != {})",
                        path, format!("{:o}", baseline_entry.mode), format!("{:o}", current_entry.mode)));
                }
                if current_entry.uid != baseline_entry.uid {
                    anomalies.push(format!("UID_CHANGED: {} ({} != {})",
                        path, baseline_entry.uid, current_entry.uid));
                }
                if current_entry.gid != baseline_entry.gid {
                    anomalies.push(format!("GID_CHANGED: {} ({} != {})",
                        path, baseline_entry.gid, current_entry.gid));
                }
            }
            None => {
                // File deleted
                anomalies.push(format!("DELETED: {}", path));
            }
        }
    }

    // Check for added files
    for (path, _) in current {
        if !baseline_map.contains_key(path) {
            anomalies.push(format!("ADDED: {}", path));
        }
    }

    anomalies
}

async fn verify_file(path: &Path, baseline_map: &HashMap<String, &FileIntegrityEntry>) -> Option<String> {
    let relative_path = path.strip_prefix("/").unwrap_or(path).to_string_lossy().to_string();

    match baseline_map.get(&relative_path) {
        Some(baseline_entry) => {
            // File exists in baseline, check integrity
            match fs::metadata(path) {
                Ok(metadata) => {
                    // Check permissions
                    if metadata.mode() & 0o7777 != baseline_entry.mode {
                        return Some(format!("PERMISSION_CHANGED: {} ({} != {})",
                            relative_path, format!("{:o}", baseline_entry.mode), format!("{:o}", metadata.mode() & 0o7777)));
                    }
                    if metadata.uid() != baseline_entry.uid {
                        return Some(format!("UID_CHANGED: {} ({} != {})",
                            relative_path, baseline_entry.uid, metadata.uid()));
                    }
                    if metadata.gid() != baseline_entry.gid {
                        return Some(format!("GID_CHANGED: {} ({} != {})",
                            relative_path, baseline_entry.gid, metadata.gid()));
                    }

                    // Check hash
                    match compute_sha512(path) {
                        Ok(sha512) => {
                            if sha512 != baseline_entry.sha512 {
                                return Some(format!("MODIFIED: {} (hash mismatch: {} != {})",
                                    relative_path, baseline_entry.sha512, sha512));
                            }
                        }
                        Err(e) => {
                            return Some(format!("ERROR_HASHING: {} ({})", relative_path, e));
                        }
                    }
                }
                Err(e) => {
                    return Some(format!("DELETED: {} ({})", relative_path, e));
                }
            }
        }
        None => {
            // File not in baseline, this is an addition
            return Some(format!("ADDED: {}", relative_path));
        }
    }
    None
}

async fn run_monitor_mode(
    args: &Args,
    baseline: &Baseline,
) -> Result<()> {
    info!("Starting integrity agent in MONITOR mode");
    info!("Watch paths: {:?}", args.watch_paths);

    let baseline_map: HashMap<String, &FileIntegrityEntry> = baseline.entries
        .iter()
        .map(|entry| (entry.path.clone(), entry))
        .collect();

    // Create monitor based on OS
    #[cfg(target_os = "linux")]
    let mut monitor = {
        use crate::fanotify_monitor::FanotifyMonitor;
        FanotifyMonitor::new(args.watch_paths.clone())
    };

    #[cfg(not(target_os = "linux"))]
    let mut monitor = {
        crate::monitor::MockMonitor::new(5) // 5 second interval for testing
    };

    let mut event_rx = monitor.start().await.map_err(|e| {
        IntegrityError::Storage(format!("Failed to start monitor: {}", e))
    })?;
    info!("Monitor started, waiting for events...");

    let mut consecutive_anomalies = 0;
    const MAX_CONSECUTIVE_ANOMALIES: usize = 5;

    while let Some(event) = event_rx.recv().await {
        tracing::debug!("Received event: {:?}", event);

        if let Some(anomaly) = verify_file(&event.path, &baseline_map).await {
            warn!("ANOMALY DETECTED: {}", anomaly);
            consecutive_anomalies += 1;

            if consecutive_anomalies >= MAX_CONSECUTIVE_ANOMALIES {
                error!("Too many consecutive anomalies detected ({}). Triggering fail-closed.", consecutive_anomalies);
                // In a real implementation, this would trigger emergency mode or shutdown
                // For now, we just exit with an error
                std::process::exit(1);
            }
        } else {
            consecutive_anomalies = 0; // Reset on successful verification
        }
    }

    info!("Monitor event channel closed");
    monitor.stop().await.map_err(|e| {
        IntegrityError::Storage(format!("Failed to stop monitor: {}", e))
    })?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Starting integrity agent");
    info!("Mode: {:?}", args.mode);
    info!("Scan path: {:?}", args.scan_path);
    info!("Image ID: {}", args.image_id);
    info!("Metadata service URL: {}", args.metadata_url);

    // Validate scan path exists
    if !args.scan_path.exists() {
        error!("Scan path does not exist: {:?}", args.scan_path);
        return Err(integrity_common::IntegrityError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Scan path does not exist",
        )));
    }

    // Fetch baseline from metadata service
    let baseline = fetch_baseline(&args.metadata_url, &args.image_id).await?;

    match args.mode {
        RunMode::Scan => {
            info!("Running in SCAN mode");
            // Scan current filesystem
            let current_state = scan_filesystem(&args.scan_path)?;

            // Compare and report anomalies
            let anomalies = compare_filesystems(&baseline, &current_state);

            if anomalies.is_empty() {
                info!("No anomalies detected. System integrity verified.");
            } else {
                warn!("Integrity check failed! Found {} anomalies:", anomalies.len());
                for anomaly in &anomalies {
                    warn!("  {}", anomaly);
                }

                // Exit with error code if anomalies found
                std::process::exit(1);
            }
        }
        RunMode::Monitor => {
            run_monitor_mode(&args, &baseline).await?;
        }
    }

    Ok(())
}
