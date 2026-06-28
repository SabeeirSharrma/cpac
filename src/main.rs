mod audit;
mod backends;
mod cache;
mod cli;
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
mod trust_db;
mod update;
mod upgrade;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
