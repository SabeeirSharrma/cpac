mod audit;
mod backends;
mod cache;
mod cli;
#[cfg(feature = "trust-db")]
mod compare;
mod config;
mod diff;
mod display;
mod install;
mod prompt;
mod remove;
mod resolver;
mod sanitize;
mod trust;
#[cfg(feature = "trust-db")]
mod trust_db;
mod update;
mod upgrade;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
