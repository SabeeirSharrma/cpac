use anyhow::Result;
use std::io::{self, Write};

/// Prompt for user confirmation.
/// Returns true if user confirms (Y/yes/empty), false otherwise.
/// Returns false on EOF to prevent auto-confirmation in piped contexts.
pub fn prompt_confirmation() -> Result<bool> {
    print!("Continue? [Y/n] ");
    io::stdout().flush()?;

    let mut input = String::new();
    let bytes_read = io::stdin().read_line(&mut input)?;

    // Handle EOF (piped input or redirected stdin)
    if bytes_read == 0 {
        return Ok(false);
    }

    let choice = input.trim();
    Ok(choice.is_empty() || choice.eq_ignore_ascii_case("y") || choice.eq_ignore_ascii_case("yes"))
}
