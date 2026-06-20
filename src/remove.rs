use anyhow::{bail, Context, Result};
use std::io::{self, Write};

use crate::{
    backends::install::{ensure_sudo, remove_package},
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
    ensure_sudo().context("Failed to request sudo credentials for package removal")?;
    remove_package(package, recursive)?;

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
