use anyhow::{Context, Result};
use std::process::Command;

use crate::{
    backends::install::{ensure_sudo, update_databases},
    cache::Cache,
    config,
};

/// Run the update command.
pub fn run(cache: &Cache, update_aur: bool) -> Result<()> {
    // Update official repositories
    println!("Updating official package databases...");
    ensure_sudo().context("Failed to request sudo credentials for package update")?;
    update_databases().context("Failed to update official package databases")?;

    // Update AUR if requested and enabled
    if update_aur && config::is_aur_enabled() {
        println!("Updating AUR package databases...");
        update_aur_databases().context("Failed to update AUR databases")?;
    } else if update_aur && !config::is_aur_enabled() {
        println!("AUR is disabled. Run 'cpac aur enable' to allow AUR updates.");
    }

    // Repository state changed, so cached package metadata is no longer trustworthy.
    cache
        .clear_metadata()
        .context("Failed to clear cached package metadata after update")?;

    println!("Cleared cached package metadata.");
    println!("Update complete.");
    Ok(())
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
            anyhow::bail!("paru -Sy failed with exit code: {}", status);
        }
    } else if Command::new("yay").arg("--version").output().is_ok() {
        let status = Command::new("yay")
            .args(["-Sy"])
            .status()
            .context("Failed to run yay -Sy")?;
        if !status.success() {
            anyhow::bail!("yay -Sy failed with exit code: {}", status);
        }
    } else {
        anyhow::bail!("No AUR helper (paru or yay) found");
    }

    Ok(())
}
