use anyhow::{bail, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::io::{self, IsTerminal, Write};

use crate::{
    audit, cache, config,
    backends::PackageInfo,
    config::{CacheInterval, ConsentLevel},
    diff, display, install, remove, resolver, trust, trust_db, update, upgrade,
};

const TAGLINE: &str = "A package trust layer for Arch-based Linux";

fn cache_ref() -> Result<&'static cache::Cache> {
    static CACHE: once_cell::sync::Lazy<cache::Cache> =
        once_cell::sync::Lazy::new(|| cache::init(None).expect("Failed to initialize cache"));
    Ok(&CACHE)
}

#[derive(Debug, Parser)]
#[command(
    name = "cpac",
    version = concat!(env!("CARGO_PKG_VERSION"), " (", "Sentinel", ") — ", "A package trust layer for Arch-based Linux"),
    about = format!("CPAC — {}", TAGLINE),
    long_about = format!(r#"
    ═════════════════════════════════════════════════════════════════
    CPAC — Trust Through Transparency
    Made by The Cinder Project (https://thecinderproject.qd.je)
    Source: https://github.com/SabeeirSharrma/cpac
    Documentation: https://thecinderproject.qd.je/cpac/docs
    Donate: https://thecinderproject.qd.je/donate/
    ═════════════════════════════════════════════════════════════════

    {}
    It analyzes packages from both official repositories and the AUR,
    providing detailed trust scores before you install anything.

    AUR helpers: Paru is preferred, but yay is also supported.
    For more information, visit our documentation."#, TAGLINE)
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Skip checking for CPAC updates on this run.
    #[arg(long, global = true)]
    no_check_updates: bool,

    /// Disable colored output.
    #[arg(long, global = true)]
    no_color: bool,
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

    /// View or change CPAC configuration.
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Clear the local metadata cache.
    ClearCache,

    /// Upgrade CPAC to the latest version from GitHub.
    Upgrade,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Show current configuration values.
    Show,

    /// Set a configuration value.
    #[command(subcommand)]
    Set(SetCommand),

    /// Reset configuration to defaults.
    Reset,

    /// Show the path to the config file.
    Path,
}

#[derive(Debug, Subcommand)]
enum SetCommand {
    /// Enable or disable AUR package search.
    ///
    /// Examples:
    ///   cpac config set aur on
    ///   cpac config set aur off
    Aur {
        /// "on" to enable, "off" to disable.
        value: OnOff,
    },

    /// Set crowdsourced data sharing level.
    ///
    /// Examples:
    ///   cpac config set consent none
    ///   cpac config set consent hash
    ///   cpac config set consent full
    Consent {
        /// "none", "hash", or "full".
        value: ConsentLevel,
    },

    /// Set automatic cache clearing interval.
    ///
    /// Examples:
    ///   cpac config set cache daily
    ///   cpac config set cache weekly
    ///   cpac config set cache monthly
    Cache {
        /// "daily", "weekly", or "monthly".
        value: CacheInterval,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OnOff {
    On,
    Off,
}

/// Show the first-run consent prompt for crowdsourced data sharing.
fn first_run_prompt() -> Result<()> {
    if config::is_first_run_done() {
        return Ok(());
    }

    // Only prompt in interactive terminals
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(());
    }

    println!();
    println!("  Welcome to CPAC!");
    println!("  Before you get started, we need to know about your privacy preferences.");
    println!();
    println!("  CPAC can compare packages against anonymized data from other users");
    println!("  to help detect tampered PKGBUILDs. Participation is completely optional.");
    println!();
    println!("  [1] No, don't submit anything");
    println!("  [2] Yes, hash/signature only  (default)");
    println!("  [3] Yes, full PKGBUILD");
    println!();

    print!("  Choice (Default: 2): ");
    io::stdout().flush()?;

    let mut input = String::new();
    let bytes_read = io::stdin().read_line(&mut input)?;

    let trimmed = input.trim();
    let consent = if trimmed.is_empty() || bytes_read == 0 {
        ConsentLevel::default()
    } else {
        let choice: u8 = trimmed.parse().unwrap_or(2);
        ConsentLevel::from_number(choice).unwrap_or_default()
    };

    config::set_consent(consent)?;
    config::mark_first_run_done()?;

    // Generate anonymous client token for future submissions
    let _ = crate::trust_db::get_client_token();

    println!("  Crowdsourced submission set to: {}", consent);
    println!();

    Ok(())
}

/// Check and perform automatic cache clearing based on configured interval.
fn auto_cache_clear() -> Result<()> {
    let _ = config::maybe_clear_cache()?;
    Ok(())
}

pub fn run() -> Result<()> {
    // Handle color control: --no-color flag or NO_COLOR env var
    // (colored v2 does not auto-respect NO_COLOR)
    let cli = Cli::parse();
    let no_color = cli.no_color || std::env::var("NO_COLOR").is_ok();
    if no_color {
        colored::control::set_override(false);
    }

    // First-run consent prompt (only shows once, only in interactive terminals)
    first_run_prompt()?;

    // Automatic cache clearing based on configured interval
    auto_cache_clear()?;

    let Some(command) = cli.command else {
        Cli::parse_from(["cpac", "--help"]);
        return Ok(());
    };

    // Skip update check for certain commands
    let skip_update_check = matches!(
        &command,
        Commands::Upgrade | Commands::Config(_) | Commands::ClearCache
    );

    match command {
        Commands::Search { query, all } => {
            let results = resolver::search(cache_ref()?, &query)?;
            display::print_search_results(&results, all);
        }
        Commands::Trust { package } => {
            let _ = trust_db::check_and_sync_if_stale();
            if let Some(pkg) = resolver::resolve(cache_ref()?, &package)? {
                let report = trust::analyze(cache_ref()?, &pkg);
                display::print_trust_report(&pkg, &report);
            } else {
                // Package not in any synced repo — check trust DB directly
                match trust_db::lookup_advisory(&package) {
                    Ok(Some(advisory)) => {
                        use crate::backends::PackageSource;
                        let pkg = PackageInfo {
                            name: advisory.package.clone(),
                            version: "unknown".to_string(),
                            description: advisory.summary.clone(),
                            source: PackageSource::Unknown,
                            maintainer: Some(advisory.reported_by.clone()),
                            votes: None,
                            popularity: None,
                            first_submitted: None,
                            last_modified: None,
                            out_of_date: false,
                            orphan: false,
                            url: None,
                            licenses: vec![],
                            depends: vec![],
                            install_size: None,
                        };
                        let report = trust::analyze(cache_ref()?, &pkg);
                        display::print_trust_report(&pkg, &report);
                        println!(
                            "  Note: '{}' is not in any synced repository. Trust data comes from the trust DB only.",
                            package
                        );
                    }
                    _ => {
                        bail!(
                            "Package '{}' not found in any repository or trust database. Try 'cpac search {}' to find the correct name.",
                            package, package
                        );
                    }
                }
            }
        }
        Commands::Audit { package } => {
            let _ = trust_db::check_and_sync_if_stale();
            if let Some(package) = package {
                let Some((pkg, report)) = audit::audit_package(cache_ref()?, &package)? else {
                    bail!(
                        "Package '{}' is not installed. Run 'pacman -Qs {}' to check.",
                        package, package
                    );
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
        Commands::Config(cmd) => match cmd {
            ConfigCommand::Show => config_show()?,
            ConfigCommand::Set(set_cmd) => config_set(set_cmd)?,
            ConfigCommand::Reset => config_reset()?,
            ConfigCommand::Path => config_path()?,
        },
        Commands::ClearCache => {
            cache::clear_cache()?;
            println!("Cache cleared successfully.");
        }
        Commands::Upgrade => {
            upgrade::run_upgrade()?;
        }
    }

    // Show update notice after command completes (unless skipped)
    if !cli.no_check_updates && !skip_update_check {
        upgrade::print_update_notice();
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

// ── Config subcommands ──────────────────────────────────────────────

fn config_show() -> Result<()> {
    let cfg = config::load()?;

    println!("Current configuration:");
    println!();
    println!("  AUR support:           {}", if cfg.aur_enabled { "on" } else { "off" });
    println!("  Crowdsourced data:     {}", cfg.consent);
    println!("  Auto-clear cache:      {}", cfg.cache_interval);
    println!("  Config file:           {}", config::path()?.display());

    Ok(())
}

fn config_set(cmd: SetCommand) -> Result<()> {
    match cmd {
        SetCommand::Aur { value } => {
            let enabled = matches!(value, OnOff::On);
            config::set_aur_enabled(enabled)?;
            println!("AUR support {}.", if enabled { "enabled" } else { "disabled" });
        }
        SetCommand::Consent { value } => {
            config::set_consent(value)?;
            println!("Crowdsourced data set to: {}", value);
        }
        SetCommand::Cache { value } => {
            config::set_cache_interval(value)?;
            println!("Auto-clear cache interval set to: {}", value);
        }
    }
    Ok(())
}

fn config_reset() -> Result<()> {
    let default = config::Config::default();
    config::save(&default)?;
    println!("Configuration reset to defaults.");
    println!();
    println!("  AUR support:           {}", if default.aur_enabled { "on" } else { "off" });
    println!("  Crowdsourced data:     {}", default.consent);
    println!("  Auto-clear cache:      {}", default.cache_interval);

    Ok(())
}

fn config_path() -> Result<()> {
    let path = config::path()?;
    println!("{}", path.display());
    Ok(())
}
