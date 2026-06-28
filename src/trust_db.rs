use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const API_URL: &str = "https://api.thecinderproject.qd.je/cpac-trust-db/api";

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
    pub affected_versions: serde_json::Value,
    #[serde(default)]
    pub safe_versions: serde_json::Value,
    #[serde(default)]
    pub reference_urls: serde_json::Value,
}

/// A snapshot entry from the trust database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub package: String,
    pub version: String,
    pub sha256: String,
    pub submitted_count: i64,
    pub first_seen: String,
    pub last_seen: String,
    #[serde(default)]
    pub pkgbuild_text: Option<String>,
}

/// Local meta state stored in ~/.cpac/trust-db/meta.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalMeta {
    pub version: String,
    pub last_sync: String,
    pub advisory_count: usize,
    pub snapshot_count: usize,
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

/// Build a reqwest client with Supabase headers.
fn supabase_client(timeout: Duration) -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .context("Failed to create HTTP client")
}

/// Load local meta state.
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

/// Compute a simple hash from advisory + snapshot data.
fn compute_version(advisories: &[Advisory], snapshots: &[SnapshotEntry]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for a in advisories {
        a.package.hash(&mut hasher);
        a.severity.hash(&mut hasher);
        a.status.hash(&mut hasher);
        a.updated.hash(&mut hasher);
    }
    for s in snapshots {
        s.package.hash(&mut hasher);
        s.version.hash(&mut hasher);
        s.sha256.hash(&mut hasher);
        s.submitted_count.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// Fetch all advisories from the trust-db API.
fn fetch_advisories() -> Result<Vec<Advisory>> {
    let url = format!("{}/advisories", API_URL);
    let client = supabase_client(Duration::from_secs(15))?;

    let response = client
        .get(&url)
        .header("Content-Type", "application/json")
        .send()
        .context("Failed to connect to trust-db server")?
        .error_for_status()
        .context("Trust-db server returned an error")?;

    let advisories: Vec<Advisory> = response.json().context("Failed to parse advisories")?;
    Ok(advisories)
}

/// Fetch all snapshots from the trust-db API.
fn fetch_snapshots() -> Result<Vec<SnapshotEntry>> {
    let url = format!("{}/snapshots", API_URL);
    let client = supabase_client(Duration::from_secs(15))?;

    let response = client
        .get(&url)
        .header("Content-Type", "application/json")
        .send()
        .context("Failed to connect to trust-db server")?
        .error_for_status()
        .context("Trust-db server returned an error")?;

    let snapshots: Vec<SnapshotEntry> = response.json().context("Failed to parse snapshots")?;
    Ok(snapshots)
}

/// Check if the local cache is stale and sync if needed.
/// Returns true if a sync was performed.
pub fn check_and_sync_if_stale() -> Result<bool> {
    let advisories = match fetch_advisories() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Warning: Could not reach trust-db server: {}", e);
            return Ok(false);
        }
    };

    let local = load_local_meta();
    let needs_sync = match local {
        Some(ref meta) => meta.advisory_count != advisories.len(),
        None => true,
    };

    if needs_sync {
        eprintln!("Trust database is out of date, syncing...");
        let result = if local.is_some() {
            sync_delta()
        } else {
            sync()
        }?;
        match result {
            SyncResult::Synced { advisories, snapshots } => {
                eprintln!("Synced {} advisories, {} snapshots", advisories, snapshots);
            }
            SyncResult::Skipped => {}
        }
        return Ok(true);
    }

    Ok(false)
}

/// Perform a full sync from Supabase to local cache.
pub fn sync() -> Result<SyncResult> {
    let advisories = match fetch_advisories() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Warning: Could not reach trust-db server: {}", e);
            return Ok(SyncResult::Skipped);
        }
    };

    let snapshots = fetch_snapshots().unwrap_or_default();

    // Save to local cache
    let dir = trust_db_dir()?;
    fs::create_dir_all(&dir)?;

    let advisories_data = serde_json::to_string(&advisories)?;
    fs::write(advisories_path()?, advisories_data)?;

    let snapshots_data = serde_json::to_string(&snapshots)?;
    fs::write(snapshots_path()?, snapshots_data)?;

    // Save meta
    let version = compute_version(&advisories, &snapshots);
    save_local_meta(&LocalMeta {
        version,
        last_sync: chrono::Utc::now().to_rfc3339(),
        advisory_count: advisories.len(),
        snapshot_count: snapshots.len(),
    })?;

    Ok(SyncResult::Synced {
        advisories: advisories.len(),
        snapshots: snapshots.len(),
    })
}

/// Fetch advisories updated since a given timestamp (for delta sync).
fn fetch_advisories_since(since: &str) -> Result<Vec<Advisory>> {
    let url = format!(
        "{}/advisories?updated_at=gt.{}",
        API_URL, since
    );
    let client = supabase_client(Duration::from_secs(15))?;

    let response = client
        .get(&url)
        .header("Content-Type", "application/json")
        .send()
        .context("Failed to connect to trust-db server for delta")?
        .error_for_status()
        .context("Trust-db server returned an error")?;

    let advisories: Vec<Advisory> = response.json().context("Failed to parse advisories")?;
    Ok(advisories)
}

/// Fetch snapshots updated since a given timestamp (for delta sync).
fn fetch_snapshots_since(since: &str) -> Result<Vec<SnapshotEntry>> {
    let url = format!(
        "{}/snapshots?last_seen=gt.{}",
        API_URL, since
    );
    let client = supabase_client(Duration::from_secs(15))?;

    let response = client
        .get(&url)
        .header("Content-Type", "application/json")
        .send()
        .context("Failed to connect to trust-db server for delta")?
        .error_for_status()
        .context("Trust-db server returned an error")?;

    let snapshots: Vec<SnapshotEntry> = response.json().context("Failed to parse snapshots")?;
    Ok(snapshots)
}

/// Perform a delta sync (only fetch changed records since last sync).
pub fn sync_delta() -> Result<SyncResult> {
    let local = load_local_meta();
    let since = match local {
        Some(ref meta) => &meta.last_sync,
        None => return sync(), // No local cache — full sync needed
    };

    let new_advisories = fetch_advisories_since(since).unwrap_or_default();
    let new_snapshots = fetch_snapshots_since(since).unwrap_or_default();

    if new_advisories.is_empty() && new_snapshots.is_empty() {
        return Ok(SyncResult::Skipped);
    }

    // Merge into local cache
    let dir = trust_db_dir()?;
    fs::create_dir_all(&dir)?;

    // Load existing data
    let existing_advisories: Vec<Advisory> = {
        let path = advisories_path()?;
        if path.exists() {
            let data = fs::read(&path)?;
            serde_json::from_slice(&data)?
        } else {
            vec![]
        }
    };

    let existing_snapshots: Vec<SnapshotEntry> = {
        let path = snapshots_path()?;
        if path.exists() {
            let data = fs::read(&path)?;
            serde_json::from_slice(&data)?
        } else {
            vec![]
        }
    };

    // Merge advisories (replace existing by package name)
    let mut merged_advisories = existing_advisories;
    for new_adv in &new_advisories {
        merged_advisories.retain(|a| a.package != new_adv.package);
        merged_advisories.push(new_adv.clone());
    }

    // Merge snapshots (replace existing by package+version+sha256)
    let mut merged_snapshots = existing_snapshots;
    for new_snap in &new_snapshots {
        merged_snapshots.retain(|s| {
            !(s.package == new_snap.package
                && s.version == new_snap.version
                && s.sha256 == new_snap.sha256)
        });
        merged_snapshots.push(new_snap.clone());
    }

    // Save merged data
    let advisories_data = serde_json::to_string(&merged_advisories)?;
    fs::write(advisories_path()?, advisories_data)?;

    let snapshots_data = serde_json::to_string(&merged_snapshots)?;
    fs::write(snapshots_path()?, snapshots_data)?;

    // Update meta
    let version = compute_version(&merged_advisories, &merged_snapshots);
    save_local_meta(&LocalMeta {
        version,
        last_sync: chrono::Utc::now().to_rfc3339(),
        advisory_count: merged_advisories.len(),
        snapshot_count: merged_snapshots.len(),
    })?;

    Ok(SyncResult::Synced {
        advisories: new_advisories.len(),
        snapshots: new_snapshots.len(),
    })
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

    Ok(all.into_iter().filter(|s| s.package == package).collect())
}

/// Look up snapshots for a specific package+version from local cache.
#[allow(dead_code)]
pub fn lookup_snapshots_for_version(package: &str, version: &str) -> Result<Vec<SnapshotEntry>> {
    let path = snapshots_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }

    let data = fs::read(path)?;
    let all: Vec<SnapshotEntry> = serde_json::from_slice(&data)?;

    Ok(all.into_iter()
        .filter(|s| s.package == package && s.version == version)
        .collect())
}

/// Submit a snapshot to the trust database via the API proxy.
///
/// POSTs to the API proxy which forwards to Supabase.
/// Uses anonymous client token for rate limiting.
/// On conflict (same package+version+sha256), increments submitted_count.
/// If `pkgbuild_text` is provided (consent=full), it's included in the submission.
pub fn submit_snapshot(package: &str, version: &str, sha256: &str, pkgbuild_text: Option<&str>) -> Result<()> {
    let url = format!("{}/snapshots", API_URL);
    let client = supabase_client(Duration::from_secs(10))?;
    let token = get_client_token().unwrap_or_default();

    let mut body = serde_json::json!({
        "package": package,
        "version": version,
        "sha256": sha256,
        "submitted_count": 1,
        "first_seen": chrono::Utc::now().to_rfc3339(),
        "last_seen": chrono::Utc::now().to_rfc3339(),
    });

    if let Some(text) = pkgbuild_text {
        body["pkgbuild_text"] = serde_json::Value::String(text.to_string());
    }

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Prefer", "resolution=merge-duplicates")
        .header("X-Client-Token", &token)
        .json(&body)
        .send()
        .context("Failed to connect to trust-db server for submission")?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        anyhow::bail!("Trust-db submission failed ({}): {}", status, body_text)
    }
}

/// Get the trust score impact for an advisory based on status.
///
/// Statuses are bidirectional trust attestations:
///   safe        → +10 (positive attestation, package verified clean)
///   suspicious  → -15 (under investigation, proceed with caution)
///   warning     → -20 (credible concern, not yet confirmed)
///   malicious   → -30 (confirmed malicious)
///   resolved    →   0 (was malicious/suspicious, now clean — neutral)
pub fn advisory_penalty(advisory: &Advisory) -> i32 {
    match advisory.status.as_str() {
        "safe" => 10,
        "suspicious" => -15,
        "warning" => -20,
        "malicious" => -30,
        "resolved" => 0,
        // Backwards compat for old status values in local cache
        "confirmed" => -20, // old "confirmed" maps to "warning"
        "suspected" => -15, // old "suspected" maps to "suspicious"
        _ => 0,
    }
}

/// Get the recommendation floor for an advisory based on status.
#[allow(dead_code)]
pub fn advisory_floor(advisory: &Advisory) -> &'static str {
    match advisory.status.as_str() {
        "malicious" => "Danger",
        "warning" => "Warning",
        "suspicious" => "Caution",
        "safe" => "",
        "resolved" => "",
        // Backwards compat
        "confirmed" => "Warning",
        "suspected" => "Caution",
        _ => "",
    }
}

// ── Local Submission Queue ──

/// A pending snapshot waiting to be submitted on next `cpac update`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSnapshot {
    pub package: String,
    pub version: String,
    pub sha256: String,
    pub queued_at: String,
    #[serde(default)]
    pub pkgbuild_text: Option<String>,
}

/// Path to the local pending submissions queue.
fn pending_queue_path() -> Result<PathBuf> {
    Ok(trust_db_dir()?.join("pending_snapshots.json"))
}

/// Path to the local client token file.
fn token_path() -> Result<PathBuf> {
    Ok(trust_db_dir()?.join("token"))
}

/// Get or create an anonymous client token for rate limiting.
/// The token is a UUID generated on first run and stored locally.
/// It's not linked to any user identity — used only for rate limiting.
pub fn get_client_token() -> Result<String> {
    let path = token_path()?;
    if path.exists() {
        let token = fs::read_to_string(&path)?;
        let token = token.trim().to_string();
        if !token.is_empty() {
            return Ok(token);
        }
    }

    // Generate new UUID token
    let token = uuid::Uuid::new_v4().to_string();
    let dir = trust_db_dir()?;
    fs::create_dir_all(&dir)?;
    fs::write(&path, &token)?;
    Ok(token)
}

/// Queue a snapshot for later submission (called during install).
#[allow(dead_code)]
pub fn queue_snapshot(package: &str, version: &str, sha256: &str, pkgbuild_text: Option<String>) -> Result<()> {
    let dir = trust_db_dir()?;
    fs::create_dir_all(&dir)?;

    let mut pending = load_pending_queue().unwrap_or_default();
    pending.push(PendingSnapshot {
        package: package.to_string(),
        version: version.to_string(),
        sha256: sha256.to_string(),
        queued_at: chrono::Utc::now().to_rfc3339(),
        pkgbuild_text,
    });

    let data = serde_json::to_string_pretty(&pending)?;
    fs::write(pending_queue_path()?, data)?;
    Ok(())
}

/// Load the local pending queue.
fn load_pending_queue() -> Result<Vec<PendingSnapshot>> {
    let path = pending_queue_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = fs::read(path)?;
    Ok(serde_json::from_slice(&data)?)
}

/// Send all pending snapshots in batch (called during `cpac update`).
/// Returns the number successfully submitted and clears the queue.
pub fn flush_pending_queue() -> Result<usize> {
    let pending = load_pending_queue()?;
    if pending.is_empty() {
        return Ok(0);
    }

    let mut success_count = 0;
    for snapshot in &pending {
        let pkgbuild = snapshot.pkgbuild_text.as_deref();
        match submit_snapshot(&snapshot.package, &snapshot.version, &snapshot.sha256, pkgbuild) {
            Ok(()) => success_count += 1,
            Err(e) => eprintln!("Warning: Failed to submit snapshot for {}: {}", snapshot.package, e),
        }
    }

    // Clear the queue after submission attempt
    fs::write(pending_queue_path()?, "[]")?;

    Ok(success_count)
}

/// Result of a sync operation.
#[derive(Debug)]
pub enum SyncResult {
    Skipped,
    Synced { advisories: usize, snapshots: usize },
}

impl std::fmt::Display for SyncResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncResult::Skipped => write!(f, "Trust database sync skipped (server unavailable)"),
            SyncResult::Synced {
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
