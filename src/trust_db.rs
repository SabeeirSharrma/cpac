use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const SUPABASE_URL: &str = "https://qzhhsyucnlswmsvpssdh.supabase.co";
const SUPABASE_ANON_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InF6aGhzeXVjbmxzd21zdnBzc2RoIiwicm9sZSI6ImFub24iLCJpYXQiOjE3ODI0NzE1NDQsImV4cCI6MjA5ODA0NzU0NH0.sIQobt0xfnMsgthGLJUb1S8f1yisDcavtRoWzi7y4OA";

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

/// Fetch all advisories from Supabase.
fn fetch_advisories() -> Result<Vec<Advisory>> {
    let url = format!("{}/rest/v1/advisories?select=*", SUPABASE_URL);
    let client = supabase_client(Duration::from_secs(15))?;

    let response = client
        .get(&url)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", SUPABASE_ANON_KEY))
        .send()
        .context("Failed to connect to trust-db server")?
        .error_for_status()
        .context("Trust-db server returned an error")?;

    let advisories: Vec<Advisory> = response.json().context("Failed to parse advisories")?;
    Ok(advisories)
}

/// Fetch all snapshots from Supabase.
fn fetch_snapshots() -> Result<Vec<SnapshotEntry>> {
    let url = format!("{}/rest/v1/snapshots?select=*", SUPABASE_URL);
    let client = supabase_client(Duration::from_secs(15))?;

    let response = client
        .get(&url)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", SUPABASE_ANON_KEY))
        .send()
        .context("Failed to connect to trust-db server")?
        .error_for_status()
        .context("Trust-db server returned an error")?;

    let snapshots: Vec<SnapshotEntry> = response.json().context("Failed to parse snapshots")?;
    Ok(snapshots)
}

/// Check if the local cache is stale.
/// Returns true if a sync is needed.
pub fn check_staleness() -> Result<bool> {
    // Quick check: try fetching advisories with a short timeout
    let advisories = match fetch_advisories() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Warning: Could not reach trust-db server: {}", e);
            return Ok(false);
        }
    };

    let local = load_local_meta();
    match local {
        Some(meta) => {
            // Check if advisory count changed
            Ok(meta.advisory_count != advisories.len())
        }
        None => Ok(true), // No local cache — needs sync
    }
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
