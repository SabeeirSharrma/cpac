use regex::Regex;
use std::net::IpAddr;

/// Result of sanitizing a PKGBUILD.
#[derive(Debug, Clone)]
pub struct SanitizedPkgbuild {
    /// The sanitized PKGBUILD text (safe for submission).
    pub text: String,
    /// Number of redactions made.
    pub redaction_count: usize,
    /// List of what was redacted (for logging, not submitted).
    pub redactions: Vec<String>,
}

/// Sanitize a PKGBUILD for submission (Pass 1: Structural Redaction).
///
/// This strips or replaces privacy-sensitive content:
/// - Local paths (current user's home directory)
/// - Hostname
/// - Local IP addresses (RFC 1918, loopback)
/// - Non-public email addresses
///
/// Redacted segments are replaced with placeholders to preserve diff structure.
pub fn sanitize_pkgbuild(content: &str) -> SanitizedPkgbuild {
    let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string());
    let hostname = hostname::get()
        .ok()
        .map(|h| h.to_string_lossy().to_string());

    let mut result = content.to_string();
    let mut redactions = Vec::new();
    let mut count = 0;

    // Pass 1a: Redact home directory paths
    if let Some(ref home_dir) = home {
        let escaped = regex::escape(home_dir);
        if let Ok(re) = Regex::new(&format!("{}[/\\w.-]*", escaped)) {
            let before = result.clone();
            result = re.replace_all(&result, "[REDACTED:path]").to_string();
            if result != before {
                redactions.push(format!("home path ({})", home_dir));
                count += 1;
            }
        }
    }

    // Pass 1b: Redact hostname
    if let Some(ref host) = hostname {
        let escaped = regex::escape(host);
        if let Ok(re) = Regex::new(&format!(r"\b{}\b", escaped)) {
            let before = result.clone();
            result = re.replace_all(&result, "[REDACTED:hostname]").to_string();
            if result != before {
                redactions.push(format!("hostname ({})", host));
                count += 1;
            }
        }
    }

    // Pass 1c: Redact local/private IP addresses
    if let Ok(re) = Regex::new(r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\b") {
        let mut new_result = String::with_capacity(result.len());
        let mut last_end = 0;

        for mat in re.find_iter(&result) {
            let ip_str = mat.as_str();
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                if is_private_ip(&ip) {
                    new_result.push_str(&result[last_end..mat.start()]);
                    new_result.push_str("[REDACTED:ip]");
                    last_end = mat.end();
                    if !redactions.iter().any(|r| r.contains("IP")) {
                        redactions.push("private IP addresses".to_string());
                    }
                }
            }
        }
        new_result.push_str(&result[last_end..]);
        if new_result != result {
            count += 1;
            result = new_result;
        }
    }

    // Pass 1d: Redact email addresses (except common public ones)
    if let Ok(re) = Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}") {
        let before = result.clone();
        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                let email = &caps[0];
                // Allow common public/maintainer emails (archlinux.org, github, etc.)
                if is_public_email(email) {
                    email.to_string()
                } else {
                    if !redactions.iter().any(|r| r.contains("email")) {
                        redactions.push("email addresses".to_string());
                    }
                    "[REDACTED:email]".to_string()
                }
            })
            .to_string();
        if result != before {
            count += 1;
        }
    }

    SanitizedPkgbuild {
        text: result,
        redaction_count: count,
        redactions,
    }
}

/// Check if an IP address is private/internal (RFC 1918, loopback, link-local).
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()           // 127.x.x.x
                || v4.is_link_local()  // 169.254.x.x
                || v4.is_private()     // 10.x.x.x, 172.16-31.x.x, 192.168.x.x
        }
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unicast_link_local()
        }
    }
}

/// Check if an email is from a public/maintainer domain (safe to keep).
fn is_public_email(email: &str) -> bool {
    let public_domains = [
        "archlinux.org",
        "github.com",
        "github.io",
        "gitlab.com",
        "aur.archlinux.org",
        "manjaro.org",
        "endeavouros.com",
        "cachyos.org",
        "garudalinux.org",
    ];
    
    let lower = email.to_lowercase();
    public_domains.iter().any(|d| lower.ends_with(d))
}

/// Compute SHA-256 hash of content.
pub fn sha256_hash(content: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn redacts_home_path() {
        let home = dirs::home_dir().unwrap_or(PathBuf::from("/home/testuser"));
        let home_str = home.to_string_lossy().to_string();
        let input = format!("source=({}/file.tar.gz)", home_str);
        let result = sanitize_pkgbuild(&input);
        assert!(result.text.contains("[REDACTED:path]"));
        assert!(!result.text.contains(&home_str));
    }

    #[test]
    fn redacts_private_ip() {
        let input = "mirror=http://192.168.1.100/packages";
        let result = sanitize_pkgbuild(input);
        assert!(result.text.contains("[REDACTED:ip]"));
        assert!(!result.text.contains("192.168.1.100"));
    }

    #[test]
    fn redacts_localhost() {
        let input = "server=http://127.0.0.1:8080";
        let result = sanitize_pkgbuild(input);
        assert!(result.text.contains("[REDACTED:ip]"));
    }

    #[test]
    fn keeps_public_urls() {
        let input = "url=https://archlinux.org/packages";
        let result = sanitize_pkgbuild(input);
        assert!(result.text.contains("https://archlinux.org/packages"));
    }

    #[test]
    fn redacts_private_email() {
        let input = "maintainer=secret@privateserver.com";
        let result = sanitize_pkgbuild(input);
        assert!(result.text.contains("[REDACTED:email]"));
    }

    #[test]
    fn keeps_public_email() {
        let input = "maintainer=dev@archlinux.org";
        let result = sanitize_pkgbuild(input);
        assert!(result.text.contains("dev@archlinux.org"));
    }
}
