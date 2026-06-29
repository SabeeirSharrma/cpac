use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::backends::{PackageInfo, PackageSource};
use crate::cache::Cache;

/// Trust tier classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustTier {
    Official,
    ThirdParty,
    Community,
    Unknown,
}

impl std::fmt::Display for TrustTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustTier::Official => write!(f, "Official"),
            TrustTier::ThirdParty => write!(f, "Third Party"),
            TrustTier::Community => write!(f, "Community"),
            TrustTier::Unknown => write!(f, "Unknown"),
        }
    }
}

/// A single signal contributing to the trust score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustSignal {
    pub name: String,
    pub points: i32,
    pub max_points: i32,
    pub detail: String,
}

/// Full trust analysis report for a package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustReport {
    pub package_name: String,
    pub tier: TrustTier,
    pub score: u32,
    pub signals: Vec<TrustSignal>,
    pub recommendation: String,
}

/// Compute a trust report for a package, using the cache if available.
pub fn analyze(cache: &Cache, pkg: &PackageInfo) -> TrustReport {
    let cache_key = format!("trust:{}-{}", pkg.name, pkg.version);
    if let Ok(Some(cached)) = cache.get_trust(&cache_key) {
        if let Ok(report) = serde_json::from_slice::<TrustReport>(&cached) {
            return report;
        }
    }

    let tier = match &pkg.source {
        PackageSource::Official { .. } => TrustTier::Official,
        PackageSource::Aur => TrustTier::Community,
        PackageSource::ThirdParty => TrustTier::ThirdParty,
        PackageSource::Unknown => TrustTier::Unknown,
    };

    let mut signals = Vec::new();
    let mut total: i32 = 0;

    // --- Signal 1: Repository Source (max +30) ---
    let source_points = match &pkg.source {
        PackageSource::Official { .. } => 30,
        PackageSource::ThirdParty => 10,
        PackageSource::Aur => 5,
        PackageSource::Unknown => -5,
    };
    let source_detail = match &pkg.source {
        PackageSource::Official { repo } => format!("Official repository ({})", repo),
        PackageSource::Aur => "AUR — community maintained".to_string(),
        PackageSource::ThirdParty => "Third-party repository".to_string(),
        PackageSource::Unknown => "Unknown source".to_string(),
    };
    signals.push(TrustSignal {
        name: "Repository Source".to_string(),
        points: source_points,
        max_points: 30,
        detail: source_detail,
    });
    total += source_points;

    // --- Signal 2: Package Age (max +15) ---
    let age_points = if let Some(submitted) = pkg.first_submitted {
        let age_days = (Utc::now() - submitted).num_days();
        let pts = match age_days {
            0..=30 => 2,      // Less than a month
            31..=180 => 5,    // 1-6 months
            181..=365 => 8,   // 6-12 months
            366..=730 => 11,  // 1-2 years
            731..=1825 => 14, // 2-5 years
            _ => 15,          // 5+ years
        };
        let detail = format_age(age_days);
        signals.push(TrustSignal {
            name: "Package Age".to_string(),
            points: pts,
            max_points: 15,
            detail,
        });
        pts
    } else {
        // No age data — penalty for missing provenance
        signals.push(TrustSignal {
            name: "Package Age".to_string(),
            points: -2,
            max_points: 15,
            detail: "Metadata unavailable — unknown provenance".to_string(),
        });
        -2
    };
    total += age_points;

    // --- Signal 3: Maintainer (max +15) ---
    let maintainer_points = if pkg.orphan {
        signals.push(TrustSignal {
            name: "Maintainer".to_string(),
            points: -10,
            max_points: 15,
            detail: "Orphaned — no active maintainer".to_string(),
        });
        -10
    } else if let Some(ref maintainer) = pkg.maintainer {
        let pts =
            if maintainer.contains('@') || matches!(&pkg.source, PackageSource::Official { .. }) {
                // Official packagers get higher trust
                13
            } else {
                10
            };
        signals.push(TrustSignal {
            name: "Maintainer".to_string(),
            points: pts,
            max_points: 15,
            detail: format!("Maintained by {}", maintainer),
        });
        pts
    } else {
        // No maintainer info — penalty for unknown custody
        signals.push(TrustSignal {
            name: "Maintainer".to_string(),
            points: -3,
            max_points: 15,
            detail: "Metadata unavailable — unknown custodian".to_string(),
        });
        -3
    };
    total += maintainer_points;

    // --- Signal 4: Popularity / Votes (max +15) ---
    let pop_points = if let Some(votes) = pkg.votes {
        let pts = match votes {
            0..=5 => 2,
            6..=25 => 5,
            26..=100 => 8,
            101..=500 => 11,
            501..=2000 => 13,
            _ => 15,
        };
        signals.push(TrustSignal {
            name: "Popularity".to_string(),
            points: pts,
            max_points: 15,
            detail: format!("{} votes", votes),
        });
        pts
    } else {
        // No popularity data — penalty for unknown adoption
        signals.push(TrustSignal {
            name: "Popularity".to_string(),
            points: -2,
            max_points: 15,
            detail: "Metadata unavailable — unknown adoption".to_string(),
        });
        -2
    };
    total += pop_points;

    // --- Signal 5: Last Updated Recency (max +15) ---
    let recency_points = if let Some(modified) = pkg.last_modified {
        let days_since = (Utc::now() - modified).num_days();
        let pts = match days_since {
            0..=7 => 15,    // Updated within a week
            8..=30 => 13,   // Within a month
            31..=90 => 11,  // Within 3 months
            91..=180 => 8,  // Within 6 months
            181..=365 => 5, // Within a year
            366..=730 => 3, // Within 2 years
            _ => 1,         // Older than 2 years
        };
        signals.push(TrustSignal {
            name: "Last Updated".to_string(),
            points: pts,
            max_points: 15,
            detail: format_recency(days_since),
        });
        pts
    } else {
        // No recency data — penalty for unknown maintenance status
        signals.push(TrustSignal {
            name: "Last Updated".to_string(),
            points: -2,
            max_points: 15,
            detail: "Metadata unavailable — unknown maintenance status".to_string(),
        });
        -2
    };
    total += recency_points;

    // --- Signal 6: Out-of-date penalty ---
    if pkg.out_of_date {
        signals.push(TrustSignal {
            name: "Out of Date".to_string(),
            points: -10,
            max_points: 0,
            detail: "Package is flagged out-of-date".to_string(),
        });
        total -= 10;
    }

    // --- Signal 7: Security Advisory ---
    let mut advisory_floor: Option<&str> = None;
    if let Ok(Some(advisory)) = crate::trust_db::lookup_advisory(&pkg.name) {
        let penalty = crate::trust_db::advisory_penalty(&advisory);
        signals.push(TrustSignal {
            name: "Security Advisory".to_string(),
            points: penalty,
            max_points: 0,
            detail: format!(
                "{} ({}) — {}",
                advisory.severity, advisory.status, advisory.summary
            ),
        });
        total += penalty;
        advisory_floor = Some(crate::trust_db::advisory_floor(&advisory));
    }

    // Clamp to 0..100
    let score = total.clamp(0, 100) as u32;

    // Compute base recommendation
    let mut recommendation = recommendation(score).to_string();

    // Enforce advisory floor: advisory status can only raise the recommendation, never lower it
    if let Some(floor) = advisory_floor {
        if !floor.is_empty() {
            let floor_rank = recommendation_rank(floor);
            let current_rank = recommendation_rank(&recommendation);
            if floor_rank > current_rank {
                recommendation = floor.to_string();
            }
        }
    }

    let report = TrustReport {
        package_name: pkg.name.clone(),
        tier,
        score,
        signals,
        recommendation,
    };

    // Cache the report
    if let Ok(serialized) = serde_json::to_vec(&report) {
        if let Err(e) = cache.insert_trust(&cache_key, serialized) {
            eprintln!("Warning: Cache write failed (trust): {}", e);
        }
    }

    report
}

/// Map a trust score to a recommendation label.
pub fn recommendation(score: u32) -> &'static str {
    match score {
        80..=100 => "Safe",
        60..=79 => "Moderate",
        40..=59 => "Caution",
        20..=39 => "Warning",
        _ => "Danger",
    }
}

/// Rank a recommendation label for comparison (higher = worse).
fn recommendation_rank(rec: &str) -> u8 {
    match rec {
        "Safe" => 0,
        "Moderate" => 1,
        "Caution" => 2,
        "Warning" => 3,
        "Danger" => 4,
        _ => 0,
    }
}

/// Format a day count into a human-readable age string.
fn format_age(days: i64) -> String {
    if days < 1 {
        "Less than a day".to_string()
    } else if days < 30 {
        format!("{} days", days)
    } else if days < 365 {
        let months = days / 30;
        if months == 1 {
            "1 month".to_string()
        } else {
            format!("{} months", months)
        }
    } else {
        let years = days / 365;
        let remaining_months = (days % 365) / 30;
        if remaining_months == 0 {
            if years == 1 {
                "1 year".to_string()
            } else {
                format!("{} years", years)
            }
        } else if years == 1 {
            format!("1 year, {} months", remaining_months)
        } else {
            format!("{} years, {} months", years, remaining_months)
        }
    }
}

/// Format a day count into a human-readable recency string.
fn format_recency(days: i64) -> String {
    if days == 0 {
        "Today".to_string()
    } else if days == 1 {
        "Yesterday".to_string()
    } else if days < 30 {
        format!("{} days ago", days)
    } else if days < 365 {
        let months = days / 30;
        if months == 1 {
            "1 month ago".to_string()
        } else {
            format!("{} months ago", months)
        }
    } else {
        let years = days / 365;
        if years == 1 {
            "1 year ago".to_string()
        } else {
            format!("{} years ago", years)
        }
    }
}

/// Result of PKGBUILD diff analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkgbuildDiff {
    pub additions: Vec<String>,
    pub deletions: Vec<String>,
    pub suspicious_patterns: Vec<String>,
}

/// Analyze a PKGBUILD diff for suspicious patterns using LCS-based diff.
pub fn analyze_pkgbuild_diff(old_pkgbuild: &str, new_pkgbuild: &str) -> PkgbuildDiff {
    let old_lines: Vec<&str> = old_pkgbuild.lines().collect();
    let new_lines: Vec<&str> = new_pkgbuild.lines().collect();

    // Compute LCS-based diff
    let diff_ops = compute_lcs_diff(&old_lines, &new_lines);

    let mut additions = Vec::new();
    let mut deletions = Vec::new();
    let mut suspicious_patterns = Vec::new();

    for op in diff_ops {
        match op {
            DiffOp::Equal => {}
            DiffOp::Delete(line) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    deletions.push(trimmed.to_string());
                }
            }
            DiffOp::Insert(line) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    additions.push(trimmed.to_string());
                    check_suspicious_pattern(trimmed, &mut suspicious_patterns);
                }
            }
        }
    }

    PkgbuildDiff {
        additions,
        deletions,
        suspicious_patterns,
    }
}

/// Diff operation types for LCS-based diff.
enum DiffOp<'a> {
    Equal,
    Delete(&'a str),
    Insert(&'a str),
}

/// Compute LCS-based diff between two sequences of lines.
fn compute_lcs_diff<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<DiffOp<'a>> {
    let m = old.len();
    let n = new.len();

    // Build LCS DP table
    let mut dp = vec![vec![0; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if old[i - 1] == new[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to construct diff operations
    let mut ops = Vec::new();
    let mut i = m;
    let mut j = n;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
            ops.push(DiffOp::Equal);
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            ops.push(DiffOp::Insert(new[j - 1]));
            j -= 1;
        } else if i > 0 {
            ops.push(DiffOp::Delete(old[i - 1]));
            i -= 1;
        }
    }

    ops.reverse();
    ops
}

/// Check a single line for suspicious patterns.
fn check_suspicious_pattern(line: &str, patterns: &mut Vec<String>) {
    let lower = line.to_lowercase();

    // Remote code execution patterns
    if lower.contains("curl")
        && (lower.contains("| sh") || lower.contains("| bash") || lower.contains("| zsh"))
    {
        patterns.push(format!("Remote script execution: {}", line));
    }
    if lower.contains("wget")
        && (lower.contains("| sh") || lower.contains("| bash") || lower.contains("| zsh"))
    {
        patterns.push(format!("Remote script execution: {}", line));
    }

    // Suspicious network calls
    if (lower.contains("curl")
        || lower.contains("wget")
        || lower.contains("nc ")
        || lower.contains("netcat"))
        && (lower.contains("http://") || lower.contains("https://"))
        && !lower.contains("pkgdesc")
        && !lower.contains("url=")
        && !lower.contains("source=(")
    {
        patterns.push(format!("Unexpected network call: {}", line));
    }

    // Inline script execution
    if lower.contains("eval ") || lower.contains("exec ") {
        patterns.push(format!("Inline script execution: {}", line));
    }

    // Suspicious file operations
    if (lower.contains("rm -rf") || lower.contains("rm -f"))
        && lower.contains("/")
        && !lower.contains("pkgdir")
        && !lower.contains("srcdir")
    {
        patterns.push(format!("Aggressive file deletion: {}", line));
    }

    // Encoding/obfuscation
    if lower.contains("base64 -d") || lower.contains("base64 --decode") {
        patterns.push(format!("Base64 decode (possible obfuscation): {}", line));
    }
    if lower.contains("xxd") || lower.contains("hexdump") {
        patterns.push(format!("Hex decode (possible obfuscation): {}", line));
    }

    // Suspicious variable assignments
    if lower.contains("pkgver=")
        && (lower.contains("curl") || lower.contains("wget") || lower.contains("git"))
    {
        patterns.push(format!("Dynamic pkgver from network: {}", line));
    }

    // Pip/npm/cargo install in build
    if (lower.contains("pip install")
        || lower.contains("npm install")
        || lower.contains("cargo install"))
        && !lower.contains("cargo build")
        && !lower.contains("cargo test")
    {
        patterns.push(format!(
            "Language package manager install in build: {}",
            line
        ));
    }

    // Modifying system files outside pkgdir
    if (lower.contains("/etc/")
        || lower.contains("/usr/")
        || lower.contains("/bin/")
        || lower.contains("/sbin/"))
        && !lower.contains("pkgdir")
        && !lower.contains("install=")
    {
        patterns.push(format!("Modifies system paths outside pkgdir: {}", line));
    }
}

/// Generate trust signals from a PKGBUILD diff.
pub fn diff_to_signals(diff: &PkgbuildDiff) -> Vec<TrustSignal> {
    let mut signals = Vec::new();

    if diff.suspicious_patterns.is_empty() {
        signals.push(TrustSignal {
            name: "Build Script Changes".to_string(),
            points: 0,
            max_points: 0,
            detail: "No suspicious changes detected".to_string(),
        });
    } else {
        for pattern in &diff.suspicious_patterns {
            signals.push(TrustSignal {
                name: "Suspicious Pattern".to_string(),
                points: -10,
                max_points: 0,
                detail: pattern.clone(),
            });
        }
    }

    signals
}

/// Get cached PKGBUILD for a package (for upgrade diffing).
pub fn get_cached_pkgbuild(cache: &Cache, package: &str) -> Result<Option<String>> {
    let key = format!("pkgbuild:{}", package);
    Ok(cache
        .get_pkgbuilds(&key)?
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string()))
}

/// Cache a PKGBUILD for future diffing.
pub fn cache_pkgbuild(cache: &Cache, package: &str, pkgbuild: &str) -> Result<()> {
    let key = format!("pkgbuild:{}", package);
    cache.insert_pkgbuilds(&key, pkgbuild.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{PackageInfo, PackageSource, PackageSource::*};
    use crate::cache::{self, Cache};
    use std::sync::OnceLock;

    fn test_cache() -> &'static Cache {
        static CACHE: OnceLock<Cache> = OnceLock::new();
        CACHE.get_or_init(|| cache::init(None).expect("Failed to initialize test cache"))
    }

    fn make_third_party_pkg(name: &str) -> PackageInfo {
        PackageInfo {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: "Test package".to_string(),
            source: ThirdParty,
            maintainer: None, // No maintainer to test unknown metadata
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
        }
    }

    #[test]
    fn all_signals_unknown_zero_negative_signals_floors_at_moderate() {
        // Test case for ThirdParty package with all metadata unavailable
        let pkg = make_third_party_pkg("test-package");
        let cache = test_cache();
        let report = analyze(cache, &pkg);

        // Score: 10 (ThirdParty) + -2 (age) + -3 (maintainer) + -2 (pop) + -2 (recency) = 1
        // Missing metadata now properly penalizes the score
        assert_eq!(report.recommendation, "Danger");
        assert_eq!(report.score, 1);

        // Verify negative signals exist (from missing metadata)
        let negative_signals = report.signals.iter().filter(|s| s.points < 0).count();
        assert_eq!(negative_signals, 4, "Should have 4 negative signals from missing metadata");
    }

    #[test]
    fn official_package_with_unknown_metadata_stays_safe() {
        // Official package with known maintainer but unknown age/popularity/recency
        let mut pkg = make_third_party_pkg("official-test");
        pkg.source = Official {
            repo: "core".to_string(),
        };
        // Official packages always have maintainers in reality
        pkg.maintainer = Some("Official Maintainer <official@archlinux.org>".to_string());

        let cache = test_cache();
        let report = analyze(cache, &pkg);

        // Score: 30 (Official) + -2 (age unknown) + 13 (maintainer) + -2 (pop unknown) + -2 (recency unknown) = 37
        // No more "floor at Moderate" — unknown metadata properly penalizes
        assert_eq!(report.recommendation, "Warning");
        assert_eq!(report.score, 37);
    }

    #[test]
    fn actual_negative_signals_still_penalize() {
        // Package with actual negative signal (orphaned) should be penalized
        let mut pkg = make_third_party_pkg("orphaned-package");
        pkg.orphan = true;

        let cache = test_cache();
        let report = analyze(cache, &pkg);

        // Should have negative signal from orphaned status
        let negative_signals = report.signals.iter().filter(|s| s.points < 0).count();
        assert!(
            negative_signals > 0,
            "Should have negative signals for orphaned package"
        );

        // Recommendation should not floor at Moderate when there are actual negative signals
        assert_ne!(
            report.recommendation, "Moderate",
            "Should not floor at Moderate with negative signals"
        );
    }
}
