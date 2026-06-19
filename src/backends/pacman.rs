use anyhow::{Context, Result};
use std::process::Command;

use super::{PackageInfo, PackageSource};

/// Search official repositories via `pacman -Ss`.
pub fn search(query: &str) -> Result<Vec<PackageInfo>> {
    let output = Command::new("pacman")
        .args(["-Ss", query])
        .output()
        .context("Failed to run pacman. Is pacman installed?")?;

    // pacman -Ss returns exit code 1 when no results found — that's not an error
    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_search_output(&stdout)
}

/// Get detailed info for a specific package via `pacman -Si`.
pub fn info(package: &str) -> Result<Option<PackageInfo>> {
    let output = Command::new("pacman")
        .args(["-Si", package])
        .output()
        .context("Failed to run pacman. Is pacman installed?")?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_info_output(&stdout)
}

/// Parse the output of `pacman -Ss`.
///
/// Format:
/// ```text
/// repo/package-name version (group)
///     Description text here
/// ```
fn parse_search_output(output: &str) -> Result<Vec<PackageInfo>> {
    let mut packages = Vec::new();
    let lines: Vec<&str> = output.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let header = lines[i].trim();
        if header.is_empty() {
            i += 1;
            continue;
        }

        // Header line: "repo/name version [group]"
        // Check it starts with a repo prefix (contains '/')
        if let Some(slash_pos) = header.find('/') {
            let repo = header[..slash_pos].to_string();
            let rest = &header[slash_pos + 1..];

            // Split into name and version
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();

                // Next line is the description (indented)
                let description = if i + 1 < lines.len() {
                    lines[i + 1].trim().to_string()
                } else {
                    String::new()
                };

                packages.push(PackageInfo {
                    name,
                    version,
                    description,
                    source: PackageSource::Official { repo },
                    maintainer: None,
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
                });

                i += 2; // Skip header + description
                continue;
            }
        }

        i += 1;
    }

    Ok(packages)
}

/// Parse the output of `pacman -Si`.
fn parse_info_output(output: &str) -> Result<Option<PackageInfo>> {
    let mut name = String::new();
    let mut version = String::new();
    let mut description = String::new();
    let mut repo = String::new();
    let mut url = None;
    let mut licenses = Vec::new();
    let mut depends = Vec::new();
    let mut maintainer = None;
    let mut install_size = None;

    for line in output.lines() {
        if let Some((key, value)) = parse_pacman_field(line) {
            match key {
                "Name" => name = value.to_string(),
                "Version" => version = value.to_string(),
                "Description" => description = value.to_string(),
                "Repository" => repo = value.to_string(),
                "URL" => url = Some(value.to_string()),
                "Licenses" => {
                    licenses = value.split_whitespace().map(|s| s.to_string()).collect();
                }
                "Depends On" => {
                    if value != "None" {
                        depends = value
                            .split_whitespace()
                            .filter(|s| {
                                !s.starts_with(">=")
                                    && !s.starts_with("<=")
                                    && !s.starts_with('>')
                                    && !s.starts_with('<')
                                    && !s.starts_with('=')
                            })
                            .map(|s| s.to_string())
                            .collect();
                    }
                }
                "Packager" => maintainer = Some(value.to_string()),
                "Installed Size" => install_size = Some(value.to_string()),
                _ => {}
            }
        }
    }

    if name.is_empty() {
        return Ok(None);
    }

    Ok(Some(PackageInfo {
        name,
        version,
        description,
        source: PackageSource::Official { repo },
        maintainer,
        votes: None,
        popularity: None,
        first_submitted: None,
        last_modified: None,
        out_of_date: false,
        orphan: false,
        url,
        licenses,
        depends,
        install_size,
    }))
}

/// Parse a single "Key : Value" line from pacman output.
fn parse_pacman_field(line: &str) -> Option<(&str, &str)> {
    let colon_pos = line.find(':')?;

    // pacman uses "Key            : Value" format with aligned colons
    let key = line[..colon_pos].trim();
    let value = line[colon_pos + 1..].trim();

    if key.is_empty() {
        return None;
    }

    Some((key, value))
}
