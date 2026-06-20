use anyhow::{bail, Result};
use colored::Colorize;

use crate::{
    cache::Cache,
    resolver,
    trust::{analyze_pkgbuild_diff, get_cached_pkgbuild, PkgbuildDiff},
};

/// Run the diff command.
pub fn run(cache: &Cache, package: &str) -> Result<()> {
    // Resolve the package
    let Some(pkg) = resolver::resolve(cache, package)? else {
        bail!("Package '{}' not found in official repositories or AUR", package);
    };

    // Check if we have a cached PKGBUILD
    let cached_pkgbuild = get_cached_pkgbuild(cache, package)?;

    // Fetch current PKGBUILD
    let current_pkgbuild = fetch_current_pkgbuild(&pkg)?;

    match (cached_pkgbuild, current_pkgbuild) {
        (Some(ref old), Some(ref new)) => {
            let diff = analyze_pkgbuild_diff(old, new);
            print_diff(&diff, package);
        }
        (None, Some(ref new)) => {
            println!("No cached PKGBUILD found for '{}'. Showing current PKGBUILD:\n", package);
            println!("{}", new);
        }
        (Some(ref old), None) => {
            println!("Cached PKGBUILD for '{}', but unable to fetch current version:\n", package);
            println!("{}", old);
        }
        (None, None) => {
            bail!("No cached PKGBUILD found and unable to fetch current PKGBUILD for '{}'", package);
        }
    }

    Ok(())
}

/// Fetch current PKGBUILD for a package.
fn fetch_current_pkgbuild(pkg: &crate::backends::PackageInfo) -> Result<Option<String>> {
    match &pkg.source {
        crate::backends::PackageSource::Aur => crate::backends::aur::fetch_pkgbuild(&pkg.name),
        crate::backends::PackageSource::Official { .. } | crate::backends::PackageSource::ThirdParty => {
            // For official packages, we'd need ABS or similar
            Ok(None)
        }
        crate::backends::PackageSource::Unknown => Ok(None),
    }
}

/// Print the PKGBUILD diff in a readable format.
fn print_diff(diff: &PkgbuildDiff, package: &str) {
    println!("PKGBUILD diff for '{}':", package);
    println!();

    if diff.additions.is_empty() && diff.deletions.is_empty() {
        println!("No changes detected.");
        return;
    }

    if !diff.additions.is_empty() {
        println!("{}", "Additions (+):".green().bold());
        for addition in &diff.additions {
            println!("  + {}", addition);
        }
        println!();
    }

    if !diff.deletions.is_empty() {
        println!("{}", "Deletions (-):".red().bold());
        for deletion in &diff.deletions {
            println!("  - {}", deletion);
        }
        println!();
    }

    if !diff.suspicious_patterns.is_empty() {
        println!("{}", "⚠️  Suspicious patterns detected:".yellow().bold());
        for pattern in &diff.suspicious_patterns {
            println!("  ! {}", pattern);
        }
        println!();
    }
}