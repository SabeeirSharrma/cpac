mod audit;
mod backends;
mod cli;
mod display;
mod resolver;
mod trust;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
