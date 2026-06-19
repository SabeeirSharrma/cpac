mod audit;
mod backends;
mod cache;
mod cli;
mod display;
mod resolver;
mod trust;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
