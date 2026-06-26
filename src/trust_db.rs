use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const TRUST_DB_BASE_URL: &str = "https://thecinderproject.qd.je/cpac-trust-db/api";

/// Meta information about the trust database state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustDbMeta {
    pub version: String,
    pub updated_at: String,
    pub advisory_count: u32,
    pub snapshot_package_count: u32,
    pub schema_version: u32,
}

/// Local meta state stored in ~/.cpac/trust-db/meta.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalMeta {
    pub version: String,
    pub last_sync: String,
    pub schema_version: u32,
}

/// An advisory from the trust database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advisory {
    pub package: String,
    pub severity: String,
    pub status: String,
    pub reported: String,
    pub updated: String,
    pub reported_by: String,
    #[serde(default)]
    pub cve: String,
    pub summary: String,
    pub description: String,
    #[serde(default)]
    pub affected_versions: Vec<String>,
    #[serde(default)]
    pub safe_versions: Vec<String>,
    #[serde(default)]
    pub reference_urls: Vec<String>,
}

/// A snapshot entry from the trust database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub version: String,
    pub sha256: String,
    pub submitted_count: u32,
    pub first_seen: String,
    pub last_seen: String,
}

/// Delta response containing changed records since a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaResponse {
    #[serde(default)]
    pub advisories: Vec<Advisory>,
    #[serde(default)]
    pub snapshots: Vec<SnapshotEntry>,
}

/// Get the path to the trust-db local cache directory.
pub fn trust_db_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not get home directory"))?;
    Ok(home.join(".cpac/trust-db"))
}

/// Get the path to the local meta.toml file.
fn meta_path() -> Result<PathBuf> {
    Ok(trust_db_dir()?.join("meta.toml"))
}

/// Get the path to the local advisories cache.
fn advisories_path() -> Result<PathBuf> {
    Ok(trust_db_dir()?.join("advisories.json"))
}

/// Get the path to the local snapshots cache.
fn snapshots_path() -> Result<PathBuf> {
    Ok(trust_db_dir()?.join("snapshots.json"))
}

/// Load local meta state, or return None if not initialized.
pub fn load_local_meta() -> Option<LocalMeta> {
    let path = meta_path().ok()?;
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

/// Save local meta state.
fn save_local_meta(meta: &LocalMeta) -> Result<()> {
    let dir = trust_db_dir()?;
    fs::create_dir_all(&dir)?;
    let path = meta_path()?;
    let content = toml::to_string_pretty(meta)?;
    fs::write(path, content)?;
    Ok(())
}

/// Fetch remote meta from /api/meta.
fn fetch_remote_meta() -> Result<TrustDbMeta> {
    let url = format!("{}/meta", TRUST_DB_BASE_URL);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client
        .get(&url)
        .send()
        .context("Failed to connect to trust-db server")?
        .error_for_status()
        .context("Trust-db server returned an error")?;

    let meta: TrustDbMeta = response.json().context("Failed to parse trust-db meta")?;
    Ok(meta)
}

/// Check if the local cache is stale by comparing version hashes.
/// Returns true if a sync is needed.
pub fn check_staleness() -> Result<bool> {
    let remote = match fetch_remote_meta() {
        Ok(meta) => meta,
        Err(e) => {
            // Network error — use local cache if available
            eprintln!("Warning: Could not reach trust-db server: {}", e);
            return Ok(false); // Not stale, just unavailable
        }
    };

    match load_local_meta() {
        Some(local) => Ok(local.version != remote.version),
        None => Ok(true), // No local cache — needs full sync
    }
}

/// Perform a delta sync if the local cache is stale.
/// This is called during `cpac update`.
pub fn sync() -> Result<SyncResult> {
    let remote = match fetch_remote_meta() {
        Ok(meta) => meta,
        Err(e) => {
            eprintln!("Warning: Could not reach trust-db server: {}", e);
            return Ok(SyncResult::Skipped);
        }
    };

    let local = load_local_meta();

    // Check if sync is needed
    if let Some(ref local) = local {
        if local.version == remote.version {
            return Ok(SyncResult::AlreadyCurrent);
        }
    }

    // Perform delta or full sync
    if let Some(ref local) = local {
        // Delta sync
        match delta_sync(&local.last_sync) {
            Ok(delta) => {
                merge_delta(&delta)?;
                save_local_meta(&LocalMeta {
                    version: remote.version,
                    last_sync: chrono::Utc::now().to_rfc3339(),
                    schema_version: remote.schema_version,
                })?;
                Ok(SyncResult::DeltaSynced {
                    advisories: delta.advisories.len(),
                    snapshots: delta.snapshots.len(),
                })
            }
            Err(e) => {
                eprintln!("Warning: Delta sync failed, falling back to full sync: {}", e);
                full_sync(&remote)
            }
        }
    } else {
        // Full sync
        full_sync(&remote)
    }
}

/// Perform a full sync of all data.
fn full_sync(remote: &TrustDbMeta) -> Result<SyncResult> {
    // Fetch all advisories
    let advisories = fetch_all_advisories()?;
    let snapshots = fetch_all_snapshots()?;

    // Save to local cache
    let advisories_data = serde_json::to_string(&advisories)?;
    fs::write(advisories_path()?, advisories_data)?;

    let snapshots_data = serde_json::to_string(&snapshots)?;
    fs::write(snapshots_path()?, snapshots_data)?;

    // Save meta
    save_local_meta(&LocalMeta {
        version: remote.version.clone(),
        last_sync: chrono::Utc::now().to_rfc3339(),
        schema_version: remote.schema_version,
    })?;

    Ok(SyncResult::FullSynced {
        advisories: advisories.len(),
        snapshots: snapshots.len(),
    })
}

/// Fetch delta changes since a timestamp.
fn delta_sync(since: &str) -> Result<DeltaResponse> {
    let url = format!("{}/delta?since={}", TRUST_DB_BASE_URL, since);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let response = client
        .get(&url)
        .send()
        .context("Failed to fetch delta from trust-db")?
        .error_for_status()?;

    let delta: DeltaResponse = response.json().context("Failed to parse delta response")?;
    Ok(delta)
}

/// Merge delta changes into local cache.
fn merge_delta(delta: &DeltaResponse) -> Result<()> {
    // Merge advisories
    if !delta.advisories.is_empty() {
        let mut local: Vec<Advisory> = fs::read(advisories_path()?)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default();

        for advisory in &delta.advisories {
            local.retain(|a| a.package != advisory.package);
            local.push(advisory.clone());
        }

        let data = serde_json::to_string(&local)?;
        fs::write(advisories_path()?, data)?;
    }

    // Merge snapshots
    if !delta.snapshots.is_empty() {
        let mut local: Vec<SnapshotEntry> = fs::read(snapshots_path()?)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default();

        for snapshot in &delta.snapshots {
            local.retain(|s| {
                !(s.version == snapshot.version && s.sha256 == snapshot.sha256)
            });
            local.push(snapshot.clone());
        }

        let data = serde_json::to_string(&local)?;
        fs::write(snapshots_path()?, data)?;
    }

    Ok(())
}

/// Fetch all advisories from the server.
fn fetch_all_advisories() -> Result<Vec<Advisory>> {
    let url = format!("{}/advisories", TRUST_DB_BASE_URL);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let response = client
        .get(&url)
        .send()
        .context("Failed to fetch advisories")?
        .error_for_status()?;

    let advisories: Vec<Advisory> = response.json().context("Failed to parse advisories")?;
    Ok(advisories)
}

/// Fetch all snapshots from the server.
fn fetch_all_snapshots() -> Result<Vec<SnapshotEntry>> {
    // Snapshots are per-package, so we'd need to know which packages to fetch.
    // For now, we'll fetch a list of packages with snapshots.
    let url = format!("{}/snapshots", TRUST_DB_BASE_URL);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let response = client
        .get(&url)
        .send()
        .context("Failed to fetch snapshots")?
        .error_for_status()?;

    let snapshots: Vec<SnapshotEntry> = response.json().context("Failed to parse snapshots")?;
    Ok(snapshots)
}

/// Look up an advisory for a specific package from local cache.
pub fn lookup_advisory(package: &str) -> Result<Option<Advisory>> {
    let path = advisories_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let data = fs::read(path)?;
    let advisories: Vec<Advisory> = serde_json::from_slice(&data)?;

    Ok(advisories.into_iter().find(|a| a.package == package))
}

/// Look up snapshots for a specific package from local cache.
#[allow(dead_code)]
pub fn lookup_snapshots(package: &str) -> Result<Vec<SnapshotEntry>> {
    let path = snapshots_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }

    let data = fs::read(path)?;
    let all: Vec<SnapshotEntry> = serde_json::from_slice(&data)?;

    Ok(all.into_iter().filter(|s| s.version == package).collect())
}

/// Get the trust penalty for an advisory based on severity.
pub fn advisory_penalty(advisory: &Advisory) -> i32 {
    match advisory.severity.as_str() {
        "critical" => -30,
        "high" => -20,
        "medium" => -10,
        "low" => -5,
        _ => {
            if advisory.status == "suspected" {
                -15
            } else if advisory.status == "resolved" {
                0
            } else {
                0
            }
        }
    }
}

/// Get the recommendation floor for an advisory based on severity.
#[allow(dead_code)]
pub fn advisory_floor(advisory: &Advisory) -> &'static str {
    match advisory.severity.as_str() {
        "critical" => "Danger",
        "high" => "Warning",
        "medium" => "Caution",
        "low" => "",
        _ => {
            if advisory.status == "suspected" {
                "Warning"
            } else {
                ""
            }
        }
    }
}

/// Result of a sync operation.
#[derive(Debug)]
pub enum SyncResult {
    AlreadyCurrent,
    Skipped,
    DeltaSynced { advisories: usize, snapshots: usize },
    FullSynced { advisories: usize, snapshots: usize },
}

impl std::fmt::Display for SyncResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncResult::AlreadyCurrent => write!(f, "Trust database is already up to date"),
            SyncResult::Skipped => write!(f, "Trust database sync skipped (server unavailable)"),
            SyncResult::DeltaSynced {
                advisories,
                snapshots,
            } => write!(
                f,
                "Trust database updated: {} advisories, {} snapshots",
                advisories, snapshots
            ),
            SyncResult::FullSynced {
                advisories,
                snapshots,
            } => write!(
                f,
                "Trust database synced: {} advisories, {} snapshots",
                advisories, snapshots
            ),
        }
    }
}
