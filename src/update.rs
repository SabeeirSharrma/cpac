use anyhow::{Context, Result};
use std::process::Command;

use crate::{
    backends::install::{ensure_sudo, update_databases},
    cache::Cache,
    config,
    trust_db,
};

/// Run the update command.
pub fn run(cache: &Cache, force_aur: bool) -> Result<()> {
    let aur_enabled = config::is_aur_enabled();
    let should_update_aur = force_aur || aur_enabled;

    // Update official repositories
    println!("Updating official package databases...");
    ensure_sudo().context("Failed to request sudo credentials for package update")?;
    update_databases().context("Failed to update official package databases")?;

    // Update AUR if enabled or explicitly requested
    if should_update_aur {
        if !aur_enabled && force_aur {
            println!("AUR is disabled but --aur was passed. Forcing AUR update...");
        }
        println!("Updating AUR package databases...");
        update_aur_databases().context("Failed to update AUR databases")?;
    }

    // Repository state changed, so cached package metadata is no longer trustworthy.
    cache
        .clear_metadata()
        .context("Failed to clear cached package metadata after update")?;

    println!("Cleared cached package metadata.");

    // Sync trust database (prefer delta, fall back to full)
    println!("Syncing trust database...");
    let sync_result = if trust_db::load_local_meta().is_some() {
        trust_db::sync_delta()
    } else {
        trust_db::sync()
    };

    match sync_result {
        Ok(result) => println!("{}", result),
        Err(e) => {
            eprintln!("Warning: Trust database sync failed: {}", e);
            eprintln!("Continuing with local cache.");
        }
    }

    // Flush pending snapshot submissions
    match trust_db::flush_pending_queue() {
        Ok(0) => {}
        Ok(n) => println!("Submitted {} snapshot(s) to trust database.", n),
        Err(e) => eprintln!("Warning: Failed to submit pending snapshots: {}", e),
    }

    // Check for advisories on installed packages (best-effort, after sync)
    check_advisory_warnings();

    println!("Update complete.");
    Ok(())
}

/// Scan local advisory cache and warn about affected packages.
fn check_advisory_warnings() {
    use colored::Colorize;

    let advisories_dir = match trust_db::trust_db_dir() {
        Ok(dir) => dir.join("advisories.json"),
        Err(_) => return,
    };

    if !advisories_dir.exists() {
        return;
    }

    let data = match std::fs::read(&advisories_dir) {
        Ok(d) => d,
        Err(_) => return,
    };

    let advisories: Vec<trust_db::Advisory> = match serde_json::from_slice(&data) {
        Ok(a) => a,
        Err(_) => return,
    };

    let active: Vec<&trust_db::Advisory> = advisories
        .iter()
        .filter(|a| a.status != "resolved")
        .collect();

    if active.is_empty() {
        return;
    }

    println!("\n  {} active advisories in trust DB:", active.len());
    for adv in &active {
        let severity_color = match adv.severity.as_str() {
            "critical" => adv.severity.red().bold(),
            "high" => adv.severity.red(),
            "medium" => adv.severity.yellow(),
            _ => adv.severity.normal(),
        };
        println!(
            "    {} {} ({}) — {}",
            severity_color,
            adv.package,
            adv.severity,
            adv.summary
        );
    }
    println!();
}

/// Update AUR package databases using the available AUR helper.
fn update_aur_databases() -> Result<()> {
    // Check for available AUR helpers
    if Command::new("paru").arg("--version").output().is_ok() {
        let status = Command::new("paru")
            .args(["-Sy"])
            .status()
            .context("Failed to run paru -Sy")?;
        if !status.success() {
            anyhow::bail!("paru -Sy failed with exit code: {}. Check your network connection and try again.", status);
        }
    } else if Command::new("yay").arg("--version").output().is_ok() {
        let status = Command::new("yay")
            .args(["-Sy"])
            .status()
            .context("Failed to run yay -Sy")?;
        if !status.success() {
            anyhow::bail!("yay -Sy failed with exit code: {}. Check your network connection and try again.", status);
        }
    } else {
        anyhow::bail!("No AUR helper (paru or yay) found. Install paru: https://github.com/Morganamilo/paru#installation");
    }

    Ok(())
}
