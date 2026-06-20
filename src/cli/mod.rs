use anyhow::{bail, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::io::{self, IsTerminal, Write};

use crate::{audit, cache, config, diff, display, install, remove, resolver, trust, update};

const TAGLINE: &str = "A package trust layer for Arch-based Linux";

fn cache_ref() -> Result<&'static cache::Cache> {
    static CACHE: once_cell::sync::Lazy<cache::Cache> =
        once_cell::sync::Lazy::new(|| cache::init(None).expect("Failed to initialize cache"));
    Ok(&CACHE)
}

#[derive(Debug, Parser)]
#[command(
    name = "cpac",
    version = concat!(env!("CARGO_PKG_VERSION"), " — A package trust layer for Arch-based Linux"),
    about = format!("CPAC — {}", TAGLINE),
    long_about = format!(r#"
    ═════════════════════════════════════════════════════════════════
    CPAC — Trust Through Transparency
    Made by The Cinder Project (https://thecinderproject.qd.je)
    Source: https://github.com/SabeeirSharrma/cpac
    Documentation: https://thecinderproject.qd.je/cpac/docs
    ═════════════════════════════════════════════════════════════════

    {}
    It analyzes packages from both official repositories and the AUR,
    providing detailed trust scores before you install anything.
    For more information, visit our documentation."#, TAGLINE)
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Search official repositories and the AUR.
    Search {
        /// Package name or keyword to search for.
        query: String,

        /// Show all results instead of the top 25.
        #[arg(long)]
        all: bool,
    },

    /// Show an explainable trust report for a package.
    Trust {
        /// Exact package name to analyze.
        package: String,
    },

    /// System-wide trust audit.
    Audit {
        /// Optional package name for a focused audit.
        package: Option<String>,
    },

    /// Install a package after trust analysis.
    Install {
        /// Package name to install.
        package: String,

        /// Skip trust analysis and confirmation prompt.
        #[arg(long)]
        force: bool,

        /// Dry run - show what would be installed without actually installing.
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove a package.
    Remove {
        /// Package name to remove.
        package: String,

        /// Also remove dependencies that are no longer needed.
        #[arg(long)]
        recursive: bool,

        /// Skip confirmation prompt.
        #[arg(long)]
        force: bool,
    },

    /// Update package metadata and sources.
    Update {
        /// Force AUR update even if AUR is disabled.
        #[arg(long)]
        aur: bool,
    },

    /// Show PKGBUILD diff (local or crowdsourced).
    Diff {
        /// Package name to diff.
        package: String,
    },

    /// Change crowdsourcing/consent preferences. Coming in v0.5.
    Config,

    /// Configure AUR usage. Coming in v0.4.
    Aur {
        /// AUR setting to change.
        action: AurAction,
    },

    /// Clear the local metadata cache.
    ClearCache,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AurAction {
    Enable,
    Disable,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    let Some(command) = cli.command else {
        Cli::parse_from(["cpac", "--help"]);
        return Ok(());
    };

    match command {
        Commands::Search { query, all } => {
            let results = resolver::search(cache_ref()?, &query)?;
            display::print_search_results(&results, all);
        }
        Commands::Trust { package } => {
            let Some(pkg) = resolver::resolve(cache_ref()?, &package)? else {
                bail!(
                    "Package '{}' was not found in official repositories or the AUR",
                    package
                );
            };
            let report = trust::analyze(cache_ref()?, &pkg);
            display::print_trust_report(&pkg, &report);
        }
        Commands::Audit { package } => {
            if let Some(package) = package {
                let Some((pkg, report)) = audit::audit_package(cache_ref()?, &package)? else {
                    bail!("Package '{}' is not installed", package);
                };
                display::print_trust_report(&pkg, &report);
            } else {
                let audit = audit::audit_system(cache_ref()?)?;
                display::print_system_audit(&audit);
                prompt_audit_details(&audit)?;
            }
        }
        Commands::Install {
            package,
            force,
            dry_run,
        } => {
            install::run(cache_ref()?, &package, force, dry_run)?;
        }
        Commands::Remove {
            package,
            recursive,
            force,
        } => {
            remove::run(cache_ref()?, &package, recursive, force)?;
        }
        Commands::Update { aur } => {
            update::run(cache_ref()?, aur)?;
        }
        Commands::Diff { package } => {
            diff::run(cache_ref()?, &package)?;
        }
        Commands::Config => {
            println!("cpac config is coming in v0.5");
        }
        Commands::Aur { action } => match action {
            AurAction::Enable => {
                config::set_aur_enabled(true)?;
                println!("AUR support enabled.");
            }
            AurAction::Disable => {
                config::set_aur_enabled(false)?;
                println!("AUR support disabled.");
            }
        },
        Commands::ClearCache => {
            cache::clear_cache()?;
            println!("Cache cleared successfully.");
        }
    }

    Ok(())
}

fn prompt_audit_details(audit: &audit::SystemAudit) -> Result<()> {
    if audit.warnings.is_empty() || !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(());
    }

    print!("View Details? [Y/n] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim();

    if choice.eq_ignore_ascii_case("n") || choice.eq_ignore_ascii_case("no") {
        return Ok(());
    }

    if !choice.is_empty()
        && !choice.eq_ignore_ascii_case("y")
        && !choice.eq_ignore_ascii_case("yes")
    {
        return Ok(());
    }

    println!();

    for warning in &audit.warnings {
        if let Some((pkg, report)) = audit::audit_package(cache_ref()?, &warning.package_name)? {
            display::print_trust_report(&pkg, &report);
        }
    }

    Ok(())
}
