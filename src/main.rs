mod audit;
mod backends;
mod cache;
mod cli;
mod config;
mod diff;
mod display;
mod install;
mod prompt;
mod remove;
mod resolver;
mod trust;
mod update;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
