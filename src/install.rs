use anyhow::{bail, Context, Result};

use crate::{
    backends::install::{ensure_sudo, install_package, select_backend, update_databases},
    backends::{PackageInfo, PackageSource},
    cache::Cache,
    compare,
    prompt,
    resolver,
    trust::{self, analyze_pkgbuild_diff, cache_pkgbuild, diff_to_signals, get_cached_pkgbuild},
    trust_db,
};

/// Run the install command.
pub fn run(cache: &Cache, package: &str, force: bool, dry_run: bool) -> Result<()> {
    // Check if trust database is stale (lightweight meta check)
    if trust_db::check_staleness()? {
        eprintln!("Note: Trust database is out of date. Run 'cpac update' to sync latest advisories.");
    }

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

    // Dry run - just show what would happen (handle early before trust analysis)
    if dry_run {
        println!(
            "\n[DRY RUN] Would install '{}' using {} backend",
            package,
            backend.cmd()
        );
        return Ok(());
    }

    // Show trust analysis (unless forced)
    let mut submit_snapshot: Option<(String, String, String)> = None; // (package, version, hash)
    if !force {
        let mut report = trust::analyze(cache, &pkg);

        // Full pre-flight check against trust DB
        if let Ok(Some(pkgbuild)) = fetch_pkgbuild_for_install(&pkg) {
            let preflight = compare::preflight_check(&pkg.name, &pkg.version, &pkgbuild);

            // Show pre-flight report
            println!("{}", compare::format_report(&preflight));

            // Apply score adjustment from pre-flight
            if preflight.score_adjustment != 0 {
                report.signals.push(trust::TrustSignal {
                    name: "Trust DB".to_string(),
                    points: preflight.score_adjustment,
                    max_points: 0,
                    detail: preflight.explanation,
                });
                let total: i32 = report.signals.iter().map(|s| s.points).sum();
                report.score = total.clamp(0, 100) as u32;
                report.recommendation = trust::recommendation(report.score).to_string();
            }

            // Queue snapshot submission for after successful install
            if preflight.should_submit {
                submit_snapshot = Some((
                    preflight.package.clone(),
                    preflight.incoming_version.clone(),
                    preflight.incoming_hash.clone(),
                ));
            }
        }

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

        // Prompt for confirmation
        if !prompt::prompt_confirmation()? {
            println!("Aborted.");
            return Ok(());
        }
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

    // Submit snapshot to trust DB (after successful install, best-effort)
    if let Some((pkg, ver, hash)) = submit_snapshot {
        match trust_db::submit_snapshot(&pkg, &ver, &hash) {
            Ok(()) => println!("Snapshot submitted to trust DB."),
            Err(e) => eprintln!("Note: Snapshot submission failed (non-critical): {}", e),
        }
    }

    println!("Successfully installed '{}'", package);
    Ok(())
}

/// Fetch PKGBUILD for installation (from AUR or official source).
fn fetch_pkgbuild_for_install(pkg: &PackageInfo) -> Result<Option<String>> {
    resolver::fetch_pkgbuild_for_package(pkg)
}
