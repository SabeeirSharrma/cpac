use anyhow::{bail, Context, Result};
use std::io::{self, Write};

use crate::{
    backends::install::remove_package,
    cache::Cache,
    resolver,
    trust::analyze,
};

/// Run the remove command.
pub fn run(cache: &Cache, package: &str, recursive: bool, force: bool) -> Result<()> {
    // Check if package is installed
    let installed = resolver::is_installed(package)?;
    if !installed {
        bail!("Package '{}' is not installed", package);
    }

    // Get package info for trust analysis
    let Some(pkg) = resolver::resolve(cache, package)? else {
        bail!("Package '{}' not found in repositories", package);
    };

    // Show trust analysis (unless forced)
    if !force {
        let report = analyze(cache, &pkg);
        crate::display::print_trust_report(&pkg, &report);

        // Warn about removing packages that other packages depend on
        if !recursive {
            println!("\nNote: Use --recursive to also remove unneeded dependencies.");
        }

        if !prompt_confirmation()? {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Dry run not implemented for remove yet, but could be added

    // Remove the package
    if recursive {
        let status = std::process::Command::new("pacman")
            .args(["-Rs", "--noconfirm", package])
            .status()
            .context("Failed to run pacman -Rs")?;

        if !status.success() {
            bail!("pacman -Rs failed with exit code: {}", status);
        }
    } else {
        remove_package(package)?;
    }

    println!("Successfully removed '{}'", package);
    Ok(())
}

/// Prompt for user confirmation.
fn prompt_confirmation() -> Result<bool> {
    print!("Continue? [Y/n] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim();

    Ok(choice.is_empty() || choice.eq_ignore_ascii_case("y") || choice.eq_ignore_ascii_case("yes"))
}
