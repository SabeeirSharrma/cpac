use anyhow::{bail, Context, Result};
use std::io::{self, Write};

use crate::{
    backends::install::{ensure_sudo, install_package, select_backend, update_databases},
    backends::{PackageInfo, PackageSource},
    cache::Cache,
    resolver,
    trust::{self, analyze_pkgbuild_diff, cache_pkgbuild, diff_to_signals, get_cached_pkgbuild},
};

/// Run the install command.
pub fn run(cache: &Cache, package: &str, force: bool, dry_run: bool) -> Result<()> {
    // Resolve the package
    let Some(pkg) = resolver::resolve(cache, package)? else {
        bail!(
            "Package '{}' not found in official repositories or AUR",
            package
        );
    };

    // Check if AUR is enabled for AUR packages
    if matches!(pkg.source, PackageSource::Aur) && !crate::config::is_aur_enabled() {
        bail!("AUR is disabled. Run 'cpac aur enable' to allow AUR packages.");
    }

    // Select backend
    let backend = select_backend(&pkg.source).ok_or_else(|| {
        anyhow::anyhow!(
            "No suitable backend found for package source: {:?}",
            pkg.source
        )
    })?;

    // Show trust analysis (unless forced)
    if !force {
        let mut report = trust::analyze(cache, &pkg);

        // For upgrades, check for PKGBUILD diff
        if resolver::is_installed(package)? {
            if let Ok(Some(cached_pkgbuild)) = get_cached_pkgbuild(cache, package) {
                if let Ok(Some(current_pkgbuild)) = fetch_pkgbuild_for_install(&pkg) {
                    let diff = analyze_pkgbuild_diff(&cached_pkgbuild, &current_pkgbuild);
                    if !diff.suspicious_patterns.is_empty() {
                        // Add diff signals to the report
                        let diff_signals = diff_to_signals(&diff);
                        report.signals.extend(diff_signals);
                        // Recalculate score
                        let total: i32 = report.signals.iter().map(|s| s.points).sum();
                        report.score = total.clamp(0, 100) as u32;
                        report.recommendation = trust::recommendation(report.score).to_string();
                    }
                }
            }
        }

        // Display trust report
        crate::display::print_trust_report(&pkg, &report);

        // Check if package is already installed
        if resolver::is_installed(package)? {
            println!(
                "\nPackage '{}' is already installed. This will be an upgrade.",
                package
            );
        }

        if dry_run {
            println!(
                "\n[DRY RUN] Would install '{}' using {} backend",
                package,
                backend.cmd()
            );
            return Ok(());
        }

        // Prompt for confirmation
        if !prompt_confirmation()? {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Dry run - just show what would happen
    if dry_run {
        println!(
            "\n[DRY RUN] Would install '{}' using {} backend",
            package,
            backend.cmd()
        );
        return Ok(());
    }

    // Update databases first
    println!("Updating package databases...");
    ensure_sudo().context("Failed to request sudo credentials for package installation")?;
    update_databases().context("Failed to update package databases")?;

    // Install the package
    println!("Installing '{}' using {}...", package, backend.cmd());
    install_package(backend, package).context("Package installation failed")?;

    // Cache the PKGBUILD for future diffing
    if let Ok(Some(pkgbuild)) = fetch_pkgbuild_for_install(&pkg) {
        let _ = cache_pkgbuild(cache, package, &pkgbuild);
    }

    println!("Successfully installed '{}'", package);
    Ok(())
}

/// Fetch PKGBUILD for installation (from AUR or official source).
fn fetch_pkgbuild_for_install(pkg: &PackageInfo) -> Result<Option<String>> {
    match &pkg.source {
        PackageSource::Aur => crate::backends::aur::fetch_pkgbuild(&pkg.name),
        PackageSource::Official { .. } | PackageSource::ThirdParty => {
            // For official packages, we could fetch from ABS but it's not available by default
            Ok(None)
        }
        PackageSource::Unknown => Ok(None),
    }
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
