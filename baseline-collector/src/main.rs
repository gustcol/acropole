use chrono;
use clap::Parser;
use integrity_common::{Baseline, FileIntegrityEntry, Result, IntegrityError};
use sha2::{Digest, Sha512};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tracing::{info, error, warn};
use walkdir::{DirEntry, WalkDir};

#[derive(Parser, Debug)]
#[command(name = "baseline-collector")]
#[command(about = "Golden Image Baseline Collector", long_about = None)]
struct Args {
    #[arg(long, default_value = "/")]
    scan_path: PathBuf,

    #[arg(long)]
    image_id: String,

    #[arg(long, default_value = "http://localhost:8080")]
    metadata_url: String,
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

fn scan_filesystem(root_path: &Path, image_id: &str) -> Result<Baseline> {
    info!("Starting filesystem scan from: {:?}", root_path);
    info!("Image ID: {}", image_id);

    let mut entries = Vec::new();
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
                            path: relative_path,
                            sha512,
                            mode: metadata.mode() & 0o7777, // Get permission bits
                            uid: metadata.uid(),
                            gid: metadata.gid(),
                        };
                        entries.push(file_entry);

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

    let timestamp = chrono::Utc::now().to_rfc3339();
    let baseline = Baseline {
        image_id: image_id.to_string(),
        timestamp,
        entries,
    };

    info!("Scan complete. Found {} files", baseline.entries.len());
    Ok(baseline)
}

async fn upload_baseline(baseline: &Baseline, metadata_url: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/baselines", metadata_url);

    info!("Uploading baseline to: {}", url);

    let response = client
        .post(&url)
        .json(baseline)
        .send()
        .await
        .map_err(|e| integrity_common::IntegrityError::Storage(e.to_string()))?;

    if response.status().is_success() {
        info!("Baseline uploaded successfully");
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        error!("Failed to upload baseline: {}", error_text);
        Err(integrity_common::IntegrityError::Storage(format!("Upload failed: {}", error_text)))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Starting baseline collector");
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

    // Scan filesystem
    let baseline = scan_filesystem(&args.scan_path, &args.image_id)?;

    // Upload to metadata service
    upload_baseline(&baseline, &args.metadata_url).await?;

    info!("Baseline collection completed successfully");
    Ok(())
}
