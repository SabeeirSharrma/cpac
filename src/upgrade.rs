use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde::Deserialize;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::process::Command;

use crate::config;

const GITHUB_REPO: &str = "SabeeirSharrma/cpac";
const GITHUB_API: &str = "https://api.github.com";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GhRelease {
    tag_name: String,
    prerelease: bool,
    draft: bool,
}

#[allow(dead_code)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub current_version: String,
}

/// Compare two semver-like version strings (e.g. "0.8.1" vs "0.8.0").
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

/// Fetch the latest release tag from GitHub.
fn fetch_latest_tag() -> Result<String> {
    let url = format!("{}/repos/{}/releases/latest", GITHUB_API, GITHUB_REPO);
    let client = reqwest::blocking::Client::builder()
        .user_agent("cpac updater")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp = client.get(&url).send().context("Failed to connect to GitHub")?;

    if resp.status().is_success() {
        let release: GhRelease = resp.json().context("Failed to parse GitHub release")?;
        return Ok(release.tag_name);
    }

    // Fallback: list releases and find first non-draft
    let url = format!("{}/repos/{}/releases", GITHUB_API, GITHUB_REPO);
    let resp = client.get(&url).send()?;
    let releases: Vec<GhRelease> = resp.json()?;
    releases
        .into_iter()
        .find(|r| !r.draft)
        .map(|r| r.tag_name)
        .context("No releases found on GitHub")
}

/// Check if a newer version is available. Returns UpdateInfo if so.
/// Caches the check for 24 hours.
pub fn check_for_update() -> Option<UpdateInfo> {
    let current = current_version();

    // Check if we recently checked (24h cache)
    if let Ok(cfg) = config::load() {
        let now = config::now_secs();
        if now.saturating_sub(cfg.last_update_check) < 86400 {
            if let Some(ref cached) = cfg.cached_latest_version {
                if is_newer(cached, current) {
                    return Some(UpdateInfo {
                        latest_version: cached.clone(),
                        current_version: current.to_string(),
                    });
                }
            }
            return None;
        }
    }

    // Fetch from GitHub
    let tag = match fetch_latest_tag() {
        Ok(t) => t,
        Err(_) => return None,
    };

    let version = tag.trim_start_matches('v').to_string();

    // Cache the result
    let _ = config::set_update_check(&version);

    if !is_newer(&version, current) {
        return None;
    }

    Some(UpdateInfo {
        latest_version: version,
        current_version: current.to_string(),
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

/// Run the upgrade: clone repo at latest tag, build from source, replace binary.
pub fn run_upgrade() -> Result<()> {
    let current = current_version();
    println!(
        "Checking for updates... (current: {})",
        current.dimmed()
    );

    // Check for concurrent CPAC processes
    let _ = check_concurrent_process();

    let tag = fetch_latest_tag().context("Failed to fetch latest release from GitHub")?;
    let version = tag.trim_start_matches('v').to_string();

    if !is_newer(&version, current) {
        println!("{}", "Already up to date!".green().bold());
        return Ok(());
    }

    println!(
        "New version available: {} → {}",
        current.yellow(),
        version.green().bold()
    );

    // Check prerequisites
    if !command_exists("git") {
        bail!("git is required for upgrades. Please install git and try again.");
    }
    if !command_exists("cargo") {
        bail!("cargo is required for upgrades. Please install Rust (https://rustup.rs) and try again.");
    }

    // Create temp build directory
    let build_dir = std::env::temp_dir().join(format!("cpac-upgrade-{}", &version));
    if build_dir.exists() {
        fs::remove_dir_all(&build_dir)?;
    }
    fs::create_dir_all(&build_dir)?;

    // Clone the repo at the target tag
    println!();
    println!("{}", format!("── Cloning CPAC {} ──", tag).cyan());
    let repo_url = format!("https://github.com/{}.git", GITHUB_REPO);

    let status = Command::new("git")
        .args(["clone", "--depth", "1", "--branch", &tag, &repo_url, "cpac"])
        .current_dir(&build_dir)
        .status()
        .context("Failed to run git clone")?;

    if !status.success() {
        fs::remove_dir_all(&build_dir)?;
        bail!(
            "git clone failed — tag '{}' may not exist on GitHub. \
             Check https://github.com/{}/releases for available versions.",
            tag, GITHUB_REPO
        );
    }

    let repo_dir = build_dir.join("cpac");

    // Build release binary
    println!();
    println!("{}", "── Building release binary ──".cyan());
    println!("This may take a few minutes...");
    println!();

    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&repo_dir)
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() {
        fs::remove_dir_all(&build_dir)?;
        bail!("cargo build failed");
    }

    let binary = repo_dir.join("target/release/cpac");
    if !binary.exists() {
        fs::remove_dir_all(&build_dir)?;
        bail!("Build succeeded but binary not found at {}", binary.display());
    }

    // Find the current binary path
    let current_exe = std::env::current_exe().context("Could not determine current binary path")?;
    let install_dir = current_exe
        .parent()
        .context("Could not determine install directory")?;

    // Replace the binary
    println!();
    println!("{}", "── Installing ──".cyan());

    let needs_sudo = !is_writable(install_dir);

    if needs_sudo {
        println!("Installing to {} (requires sudo)...", install_dir.display());

        // Copy to a temp location first, then sudo cp
        let tmp_binary = build_dir.join("cpac-new");
        fs::copy(&binary, &tmp_binary)?;

        let status = Command::new("sudo")
            .args(["cp", tmp_binary.to_str().unwrap(), current_exe.to_str().unwrap()])
            .status()
            .context("Failed to copy binary with sudo")?;

        if !status.success() {
            fs::remove_dir_all(&build_dir)?;
            bail!("Failed to install binary (sudo cp failed)");
        }

        // Make executable
        let _ = Command::new("sudo")
            .args(["chmod", "755", current_exe.to_str().unwrap()])
            .status();
    } else {
        println!("Installing to {}...", current_exe.display());

        // Atomic replacement: write to temp file, then rename over the target.
        // On Linux, rename() is atomic on the same filesystem (ext4/xfs).
        let tmp_new = current_exe.with_extension("tmp");
        let old_path = current_exe.with_extension("old");
        let _ = fs::remove_file(&tmp_new);
        let _ = fs::remove_file(&old_path);

        // Copy new binary to temp file
        fs::copy(&binary, &tmp_new).with_context(|| {
            format!("Failed to copy new binary to {}", tmp_new.display())
        })?;

        // Set executable permissions on temp file before rename
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&tmp_new, perms)?;
        }

        // Atomic rename: temp -> target (this is atomic on Linux ext4/xfs)
        if let Err(e) = fs::rename(&tmp_new, &current_exe) {
            // Rollback: try to restore from .old if it exists
            let _ = fs::remove_file(&tmp_new);
            bail!("Failed to replace binary: {}. Current binary is unchanged.", e);
        }

        // Clean up old backup
        let _ = fs::remove_file(&old_path);
    }

    // Clean up build directory
    fs::remove_dir_all(&build_dir)?;

    // Verify
    println!();
    let status = Command::new(&current_exe)
        .args(["--version"])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!();
            println!(
                "{}",
                format!("Upgraded successfully! {} → {}", current, version)
                    .green()
                    .bold()
            );
        }
        _ => {
            println!(
                "{}",
                format!("Upgraded {} → {}", current, version)
                    .green()
                    .bold()
            );
            println!(
                "{}",
                "Warning: could not verify new binary. Run 'cpac --version' to check.".yellow()
            );
        }
    }

    println!(
        "Your config at {} was not affected.",
        "~/.cpac/".dimmed()
    );
    println!();

    Ok(())
}

/// Check if a command exists on the system.
fn command_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if another cpac process might be running (best-effort).
fn check_concurrent_process() -> Result<()> {
    // Check /proc for other cpac processes (Linux-specific)
    if let Ok(entries) = fs::read_dir("/proc") {
        let my_pid = std::process::id();
        for entry in entries.flatten() {
            let name = entry.file_name();
            if let Some(pid_str) = name.to_str() {
                if let Ok(pid) = pid_str.parse::<u32>() {
                    if pid == my_pid || pid == 0 {
                        continue;
                    }
                    // Check if this PID is a cpac process
                    let cmdline_path = entry.path().join("cmdline");
                    if let Ok(cmdline) = fs::read(&cmdline_path) {
                        let cmdline_str = String::from_utf8_lossy(&cmdline);
                        if cmdline_str.contains("cpac") && !cmdline_str.contains("cpac upgrade") {
                            // Found another cpac process (not this upgrade)
                            // Don't block, just warn
                            eprintln!("Warning: Another CPAC process (PID {}) appears to be running.", pid);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Check if a directory is writable (without writing anything).
fn is_writable(dir: &Path) -> bool {
    // Try to check by seeing if we can stat and what the permissions are
    use std::os::unix::fs::MetadataExt;

    match fs::metadata(dir) {
        Ok(meta) => {
            let mode = meta.mode();
            // Owner write bit
            (mode & 0o200) != 0
        }
        Err(_) => false,
    }
}
