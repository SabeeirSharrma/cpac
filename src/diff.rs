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

    // Check if PKGBUILD diff is supported for this package source
    let source_desc = match &pkg.source {
        crate::backends::PackageSource::Aur => "AUR",
        crate::backends::PackageSource::Official { repo } => {
            bail!(
                "PKGBUILD diff is not supported for official repository packages ({}).\n\
                Official Arch packages don't publish PKGBUILDs in a readily accessible format.\n\
                This command currently only supports AUR packages.",
                repo
            );
        }
        crate::backends::PackageSource::ThirdParty => {
            bail!(
                "PKGBUILD diff is not supported for third-party repository packages.\n\
                Third-party repos typically don't expose PKGBUILDs via their RPC interfaces.\n\
                This command currently only supports AUR packages."
            );
        }
        crate::backends::PackageSource::Unknown => {
            bail!("Cannot determine package source for '{}'", package);
        }
    };

    // Check if we have a cached PKGBUILD
    let cached_pkgbuild = get_cached_pkgbuild(cache, package)?;

    // Fetch current PKGBUILD (only works for AUR)
    let current_pkgbuild = crate::backends::aur::fetch_pkgbuild(&pkg.name)?;

    match (cached_pkgbuild, current_pkgbuild) {
        (Some(ref old), Some(ref new)) => {
            let diff = analyze_pkgbuild_diff(old, new);
            print_diff(&diff, package);
        }
        (None, Some(ref new)) => {
            println!("No cached PKGBUILD found for '{}'. Showing current PKGBUILD from {}:\n", package, source_desc);
            println!("{}", new);
        }
        (Some(ref old), None) => {
            println!("Cached PKGBUILD for '{}', but unable to fetch current version from {}:\n", package, source_desc);
            println!("{}", old);
        }
        (None, None) => {
            bail!("No cached PKGBUILD found and unable to fetch current PKGBUILD from {} for '{}'", source_desc, package);
        }
    }

    Ok(())
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