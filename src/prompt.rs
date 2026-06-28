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

/// Prompt for one-time contribution of an unknown package.
/// default_yes: true when consent is None or Hash (opt-in default).
/// Returns true if user agrees.
pub fn prompt_contribute_package(default_yes: bool) -> Result<bool> {
    let hint = if default_yes { "Y/n" } else { "y/N" };
    print!(
        "Would you like to share anonymous information (redacted PKGBUILD and hash) for ONLY THIS PACKAGE? [{}] ",
        hint
    );
    io::stdout().flush()?;

    let mut input = String::new();
    let bytes_read = io::stdin().read_line(&mut input)?;

    if bytes_read == 0 {
        return Ok(default_yes);
    }

    let choice = input.trim();
    if choice.is_empty() {
        return Ok(default_yes);
    }
    Ok(choice.eq_ignore_ascii_case("y") || choice.eq_ignore_ascii_case("yes"))
}
