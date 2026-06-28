use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde::Deserialize;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

use crate::config;

const GITHUB_REPO: &str = "SabeeirSharrma/cpac";
const GITHUB_API: &str = "https://api.github.com";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GhRelease {
    tag_name: String,
    prerelease: bool,
    draft: bool,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[allow(dead_code)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub current_version: String,
    pub download_url: String,
    pub asset_name: String,
}

/// Compare two semver-like version strings (e.g. "0.8.0" vs "0.7.2").
/// Returns true if `latest` is newer than `current`.
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.trim_start_matches('v').parse().ok())
            .collect()
    };
    let lv = parse(latest);
    let cv = parse(current);
    lv > cv
}

/// Get the current CPAC version (from Cargo.toml at compile time).
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Detect the current platform: "x86_64" or "aarch64".
fn detect_platform() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "x86_64"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "aarch64"
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        "unknown"
    }
}

/// Fetch the latest release info from GitHub.
fn fetch_latest_release() -> Result<GhRelease> {
    let url = format!("{}/repos/{}/releases", GITHUB_API, GITHUB_REPO);
    let client = reqwest::blocking::Client::builder()
        .user_agent("cpac updater")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp = client.get(&url).send().context("Failed to connect to GitHub")?;

    if !resp.status().is_success() {
        bail!("GitHub API returned status {}", resp.status());
    }

    let releases: Vec<GhRelease> = resp.json().context("Failed to parse GitHub releases")?;

    // Find the first non-draft release
    let release = releases
        .into_iter()
        .find(|r| !r.draft)
        .context("No releases found on GitHub")?;

    Ok(release)
}

/// Check if a newer version is available. Returns UpdateInfo if so.
/// Caches the check for 24 hours.
pub fn check_for_update() -> Option<UpdateInfo> {
    let current = current_version();

    // Check if we recently checked (24h cache)
    if let Ok(cfg) = config::load() {
        let now = config::now_secs();
        if now.saturating_sub(cfg.last_update_check) < 86400 {
            // Use cached latest version if available
            if let Some(ref cached) = cfg.cached_latest_version {
                if is_newer(cached, current) {
                    // We know there's an update, but need to fetch download URL
                    // Return without URL — notice will just show version
                    return Some(UpdateInfo {
                        latest_version: cached.clone(),
                        current_version: current.to_string(),
                        download_url: String::new(),
                        asset_name: String::new(),
                    });
                }
            }
            return None;
        }
    }

    // Fetch from GitHub
    let release = match fetch_latest_release() {
        Ok(r) => r,
        Err(_) => return None,
    };

    let tag = release.tag_name.trim_start_matches('v').to_string();

    // Cache the result
    let _ = config::set_update_check(&tag);

    if !is_newer(&tag, current) {
        return None;
    }

    // Find the right asset for this platform
    let platform = detect_platform();
    let asset_name = format!("cpac-{}-linux", platform);

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .or_else(|| release.assets.iter().find(|a| a.name.contains(platform)))
        .or_else(|| release.assets.iter().find(|a| a.name.contains("linux")));

    let (download_url, asset_name) = match asset {
        Some(a) => (a.browser_download_url.clone(), a.name.clone()),
        None => (String::new(), String::new()),
    };

    Some(UpdateInfo {
        latest_version: tag,
        current_version: current.to_string(),
        download_url,
        asset_name,
    })
}

/// Print a notice if a newer version is available.
pub fn print_update_notice() {
    if !io::stdout().is_terminal() {
        return;
    }

    if let Some(info) = check_for_update() {
        println!();
        println!(
            "  {} A new version of CPAC is available: {} (current: {})",
            ">>".yellow().bold(),
            info.latest_version.green().bold(),
            info.current_version
        );
        println!(
            "  Run {} to upgrade.",
            "cpac upgrade".cyan().bold()
        );
        println!();
    }
}

/// Run the upgrade: download the latest binary and replace the current one.
pub fn run_upgrade() -> Result<()> {
    let current = current_version();
    println!(
        "Checking for updates... (current: {})",
        current.dimmed()
    );

    let release = fetch_latest_release().context("Failed to fetch latest release from GitHub")?;
    let tag = release.tag_name.trim_start_matches('v').to_string();

    if !is_newer(&tag, current) {
        println!(
            "{}",
            "Already up to date!".green().bold()
        );
        return Ok(());
    }

    println!(
        "New version available: {} → {}",
        current.yellow(),
        tag.green().bold()
    );

    // Find the right asset
    let platform = detect_platform();
    if platform == "unknown" {
        bail!(
            "Unsupported platform. CPAC upgrades are only available for x86_64 and aarch64 Linux."
        );
    }

    let asset_name = format!("cpac-{}-linux", platform);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .with_context(|| format!("No binary found for platform '{}' in release {}", platform, tag))?;

    println!(
        "Downloading {} ({} bytes)...",
        asset.name,
        asset.size
    );

    // Download the binary
    let client = reqwest::blocking::Client::builder()
        .user_agent("cpac updater")
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let mut resp = client
        .get(&asset.browser_download_url)
        .send()
        .context("Failed to download binary")?;

    if !resp.status().is_success() {
        bail!("Download failed with status {}", resp.status());
    }

    // Get the path of the current running binary
    let current_exe = std::env::current_exe().context("Could not determine current binary path")?;

    // Write to a temporary file first
    let tmp_path = current_exe.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)
        .with_context(|| format!("Failed to create temporary file at {}", tmp_path.display()))?;

    let mut total: u64 = 0;
    let mut buf = [0u8; 8192];
    loop {
        let bytes_read = resp.read(&mut buf)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buf[..bytes_read])?;
        total += bytes_read as u64;
    }
    drop(file);

    if total == 0 {
        fs::remove_file(&tmp_path)?;
        bail!("Downloaded binary is empty");
    }

    println!("Downloaded {} bytes.", total);

    // Verify checksum if sha256sums.txt is available
    if let Some(checksum_asset) = release
        .assets
        .iter()
        .find(|a| a.name == "sha256sums.txt")
    {
        println!("Verifying checksum...");
        match verify_checksum(&tmp_path, &checksum_asset.browser_download_url, &asset.name) {
            Ok(()) => println!("{}", "Checksum verified.".green()),
            Err(e) => {
                fs::remove_file(&tmp_path)?;
                bail!("Checksum verification failed: {}", e);
            }
        }
    }

    // Make the new binary executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&tmp_path, perms)?;
    }

    // Replace the current binary
    // On Linux, we can't rename over a running executable directly.
    // Strategy: rename current to .old, rename new to current, delete .old.
    let old_path = current_exe.with_extension("old");

    // Remove any leftover .old from previous upgrade
    let _ = fs::remove_file(&old_path);

    // Rename current binary to .old
    fs::rename(&current_exe, &old_path)
        .with_context(|| format!("Failed to rename current binary to {}", old_path.display()))?;

    // Rename new binary to current
    fs::rename(&tmp_path, &current_exe)
        .with_context(|| {
            // Try to restore the old binary if rename fails
            let _ = fs::rename(&old_path, &current_exe);
            format!("Failed to rename new binary to {}", current_exe.display())
        })?;

    // Clean up old binary
    let _ = fs::remove_file(&old_path);

    println!();
    println!(
        "{}",
        format!("Upgraded successfully! {} → {}", current, tag)
            .green()
            .bold()
    );
    println!(
        "Run {} to see what's new.",
        "cpac --version".cyan()
    );

    Ok(())
}

/// Verify the SHA-256 checksum of a downloaded file.
fn verify_checksum(file_path: &PathBuf, checksums_url: &str, asset_name: &str) -> Result<()> {
    use sha2::{Digest, Sha256};

    // Compute hash of downloaded file
    let contents = fs::read(file_path).context("Failed to read downloaded file")?;
    let mut hasher = Sha256::new();
    hasher.update(&contents);
    let hash = format!("{:x}", hasher.finalize());

    // Fetch checksums
    let client = reqwest::blocking::Client::builder()
        .user_agent("cpac updater")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let checksums_text = client
        .get(checksums_url)
        .send()?
        .text()?;

    // Parse the checksum line for our asset
    for line in checksums_text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1].contains(asset_name) {
            let expected = parts[0];
            if hash == expected {
                return Ok(());
            } else {
                bail!(
                    "Checksum mismatch: expected {}, got {}",
                    expected,
                    hash
                );
            }
        }
    }

    // If asset not found in checksums, warn but don't fail
    println!(
        "{}",
        "Warning: Asset not found in checksums file, skipping verification.".yellow()
    );
    Ok(())
}
