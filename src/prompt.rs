use anyhow::Result;
use std::io::{self, Write};

/// Prompt for user confirmation.
/// Returns true if user confirms (Y/yes/empty), false otherwise.
pub fn prompt_confirmation() -> Result<bool> {
    print!("Continue? [Y/n] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim();

    Ok(choice.is_empty() || choice.eq_ignore_ascii_case("y") || choice.eq_ignore_ascii_case("yes"))
}
