use anyhow::{bail, Result};
use clap::{Parser, Subcommand, ValueEnum};

use crate::{display, resolver, trust};

#[derive(Debug, Parser)]
#[command(
    name = "cpac",
    version,
    about = "Community Package Analysis Client (CPAC)",
    long_about = r#"
    ═════════════════════════════════════════════════════════════════
    CPAC - Trust Through Transparency.
    Made by The Cinder Project (https://thecinderproject.qd.je)
    GitHub: https://thecinderproject.qd.je/cpac
    Documentation: https://thecinderproject.qd.je/cpac/docs
    ═════════════════════════════════════════════════════════════════

    A package trust layer for Arch-based Linux distributions.
It analyzes packages from both official repositories and the AUR,
providing detailed trust scores before you install anything.
For more information, visit our documentation."#
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Search official repositories and the AUR.
    Search {
        /// Package name or keyword to search for.
        query: String,
    },

    /// Show an explainable trust report for a package.
    Trust {
        /// Exact package name to analyze.
        package: String,
    },

    /// System-wide trust audit. Coming in Week 2.
    Audit {
        /// Optional package name for a focused audit.
        package: Option<String>,
    },

    /// Install a package after trust analysis. Coming in Week 3.
    Install {
        /// Package name to install.
        package: String,
    },

    /// Remove a package. Coming later.
    Remove {
        /// Package name to remove.
        package: String,
    },

    /// Update package metadata and sources. Coming later.
    Update,

    /// Configure AUR usage. Coming in Week 4.
    Aur {
        /// AUR setting to change.
        action: AurAction,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AurAction {
    Enable,
    Disable,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search { query } => {
            let results = resolver::search(&query)?;
            display::print_search_results(&results);
        }
        Commands::Trust { package } => {
            let Some(pkg) = resolver::resolve(&package)? else {
                bail!(
                    "Package '{}' was not found in official repositories or the AUR",
                    package
                );
            };
            let report = trust::analyze(&pkg);
            display::print_trust_report(&pkg, &report);
        }
        Commands::Audit { package } => {
            if let Some(package) = package {
                println!("cpac audit {} is coming in Week 2", package);
            } else {
                println!("cpac audit is coming in Week 2");
            }
        }
        Commands::Install { package } => {
            println!("cpac install {} is coming in Week 3", package);
        }
        Commands::Remove { package } => {
            println!("cpac remove {} is coming later", package);
        }
        Commands::Update => {
            println!("cpac update is coming later");
        }
        Commands::Aur { action } => match action {
            AurAction::Enable => println!("cpac aur enable is coming in Week 4"),
            AurAction::Disable => println!("cpac aur disable is coming in Week 4"),
        },
    }

    Ok(())
}
