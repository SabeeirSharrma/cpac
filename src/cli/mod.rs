use anyhow::{bail, Result};
use clap::{Parser, Subcommand, ValueEnum};

use crate::{display, resolver, trust};

const TAGLINE: &str = "A package trust layer for Arch-based Linux";

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

    /// System-wide trust audit. Coming in v0.2.
    Audit {
        /// Optional package name for a focused audit.
        package: Option<String>,
    },

    /// Install a package after trust analysis. Coming in v0.5.
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

    /// Show PKGBUILD diff (local or crowdsourced). Coming in v0.4.
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
            let results = resolver::search(&query)?;
            display::print_search_results(&results, all);
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
                println!("cpac audit {} is coming in v0.2", package);
            } else {
                println!("cpac audit is coming in v0.2");
            }
        }
        Commands::Install { package } => {
            println!("cpac install {} is coming in v0.5", package);
        }
        Commands::Remove { package } => {
            println!("cpac remove {} is coming later", package);
        }
        Commands::Update => {
            println!("cpac update is coming later");
        }
        Commands::Diff { package } => {
            println!("cpac diff {} is coming in v0.4", package);
        }
        Commands::Config => {
            println!("cpac config is coming in v0.5");
        }
        Commands::Aur { action } => match action {
            AurAction::Enable => println!("cpac aur enable is coming in v0.4"),
            AurAction::Disable => println!("cpac aur disable is coming in v0.4"),
        },
    }

    Ok(())
}
