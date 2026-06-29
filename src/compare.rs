use serde::{Deserialize, Serialize};

use crate::trust_db::{self, Advisory, SnapshotEntry};

/// Full pre-flight check result for a package.
///
/// This is the single call that tells CPAC everything it needs to know
/// before deciding whether to install a package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreFlightReport {
    /// Package name checked.
    pub package: String,
    /// Version that CPAC is about to install.
    pub incoming_version: String,
    /// Hash of the incoming PKGBUILD.
    pub incoming_hash: String,

    // ── Version Intelligence ──
    /// Latest version known in the trust DB.
    pub latest_known_version: Option<String>,
    /// All versions known in the trust DB (sorted).
    pub known_versions: Vec<String>,
    /// Whether the incoming version is the latest known.
    pub is_latest: bool,
    /// Whether the incoming version is older than what's known.
    pub is_outdated: bool,
    /// Whether the incoming version is unknown to the DB.
    pub is_unknown_version: bool,

    // ── Hash Intelligence ──
    /// Whether this exact hash is already in the DB.
    pub hash_known: bool,
    /// Number of submissions for this hash.
    pub hash_submissions: usize,
    /// The majority hash for this version (if any).
    pub majority_hash: Option<String>,
    /// Whether the hash matches the majority consensus.
    pub matches_consensus: bool,
    /// How many submissions the majority hash has.
    pub majority_submissions: usize,
    /// Total submissions for this version.
    pub total_submissions: usize,

    // ── Advisory Check ──
    /// Active advisory for this package (if any).
    pub advisory: Option<Advisory>,
    /// Whether this version is in the affected list.
    pub version_affected: bool,
    /// Whether this version is in the safe list.
    pub version_safe: bool,

    // ── Verdict ──
    /// Overall safety verdict.
    pub verdict: Verdict,
    /// Trust score adjustment based on this check.
    pub score_adjustment: i32,
    /// Human-readable explanation.
    pub explanation: String,
    /// Whether the DB already has this hash (used to skip submission).
    pub should_submit: bool,
}

/// Overall safety verdict from the pre-flight check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// Hash matches consensus, no advisory, latest version. Safe to proceed.
    Clean,
    /// Hash matches consensus but version is affected by advisory.
    AdvisoryHit,
    /// Hash diverges from consensus — possible tampering.
    Divergent,
    /// Version is outdated compared to what's known.
    Outdated,
    /// No data in DB yet — first time seeing this package/version.
    Unknown,
    /// Multiple concerns (e.g. outdated + advisory).
    Mixed,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verdict::Clean => write!(f, "Clean"),
            Verdict::AdvisoryHit => write!(f, "Advisory"),
            Verdict::Divergent => write!(f, "Divergent"),
            Verdict::Outdated => write!(f, "Outdated"),
            Verdict::Unknown => write!(f, "Unknown"),
            Verdict::Mixed => write!(f, "Mixed"),
        }
    }
}

/// Run a full pre-flight check for a package.
///
/// This is the main entry point. Given a package name, version, and PKGBUILD
/// content, it queries the local trust DB cache and returns a comprehensive
/// report telling CPAC everything it needs to know.
pub fn preflight_check(
    package: &str,
    version: &str,
    pkgbuild_content: &str,
    source: &crate::backends::PackageSource,
) -> PreFlightReport {
    let hash = crate::sanitize::sha256_hash(pkgbuild_content);

    // ── Gather data from local cache ──
    let snapshots = trust_db::lookup_snapshots(package).unwrap_or_default();
    let advisory = trust_db::lookup_advisory(package).ok().flatten();

    // ── Version intelligence ──
    let mut known_versions: Vec<String> = snapshots.iter().map(|s| s.version.clone()).collect();
    known_versions.sort();
    known_versions.dedup();

    let latest_known_version = known_versions.last().cloned();
    let is_latest = latest_known_version.as_deref() == Some(version);
    let is_outdated = if let Some(ref latest) = latest_known_version {
        version != latest.as_str()
    } else {
        false
    };
    let is_unknown_version = !known_versions.contains(&version.to_string());

    // ── Hash intelligence ──
    let version_snapshots: Vec<&SnapshotEntry> = snapshots
        .iter()
        .filter(|s| s.version == version)
        .collect();

    let total_submissions: usize = version_snapshots.iter().map(|s| s.submitted_count as usize).sum();

    // Find majority hash
    let majority = version_snapshots
        .iter()
        .max_by_key(|s| s.submitted_count)
        .map(|s| (s.sha256.clone(), s.submitted_count as usize));

    let (majority_hash, majority_submissions) = majority.unwrap_or_default();
    let matches_consensus = majority_hash == hash;

    let hash_known = version_snapshots.iter().any(|s| s.sha256 == hash);
    let hash_submissions = version_snapshots
        .iter()
        .filter(|s| s.sha256 == hash)
        .map(|s| s.submitted_count as usize)
        .sum();

    // ── Advisory check ──
    let (version_affected, version_safe) = if let Some(ref adv) = advisory {
        let affected = advisory_affected_versions(adv);
        let safe = advisory_safe_versions(adv);
        (
            affected.contains(&version.to_string()),
            safe.contains(&version.to_string()),
        )
    } else {
        (false, false)
    };

    // ── Verdict & scoring ──
    let (verdict, score_adjustment, explanation) = compute_verdict(
        &hash,
        matches_consensus,
        majority_submissions,
        total_submissions,
        hash_known,
        hash_submissions,
        is_latest,
        is_outdated,
        is_unknown_version,
        version_affected,
        version_safe,
        &advisory,
        package,
        version,
        source,
    );

    // ── Should submit? ──
    // Don't submit if this version already has snapshots (someone else contributed it)
    let version_has_snapshots = !version_snapshots.is_empty();
    // Don't submit if hash matches the latest known PKGBUILD for this package (no change)
    let matches_latest = matches_consensus;
    // Don't submit if hash is already well-known (>10 submissions)
    let hash_well_known = hash_known && hash_submissions >= 10;
    let should_submit = !version_has_snapshots && !matches_latest && !hash_well_known;

    PreFlightReport {
        package: package.to_string(),
        incoming_version: version.to_string(),
        incoming_hash: hash,

        latest_known_version,
        known_versions,
        is_latest,
        is_outdated,
        is_unknown_version,

        hash_known,
        hash_submissions,
        majority_hash: if majority_hash.is_empty() { None } else { Some(majority_hash) },
        matches_consensus,
        majority_submissions,
        total_submissions,

        advisory,
        version_affected,
        version_safe,

        verdict,
        score_adjustment,
        explanation,
        should_submit,
    }
}

/// Extract affected versions from an advisory.
fn advisory_affected_versions(adv: &Advisory) -> Vec<String> {
    match &adv.affected_versions {
        serde_json::Value::Array(arr) => {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        }
        _ => vec![],
    }
}

/// Extract safe versions from an advisory.
fn advisory_safe_versions(adv: &Advisory) -> Vec<String> {
    match &adv.safe_versions {
        serde_json::Value::Array(arr) => {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        }
        _ => vec![],
    }
}

/// Compute the final verdict, score adjustment, and explanation.
#[allow(clippy::too_many_arguments)]
fn compute_verdict(
    _hash: &str,
    matches_consensus: bool,
    majority_submissions: usize,
    total_submissions: usize,
    hash_known: bool,
    hash_submissions: usize,
    _is_latest: bool,
    is_outdated: bool,
    is_unknown_version: bool,
    version_affected: bool,
    version_safe: bool,
    advisory: &Option<Advisory>,
    package: &str,
    version: &str,
    source: &crate::backends::PackageSource,
) -> (Verdict, i32, String) {
    let mut concerns = Vec::new();
    let mut adjustments = Vec::new();

    // Advisory check
    if version_affected {
        if let Some(ref adv) = advisory {
            let penalty = trust_db::advisory_penalty(adv);
            adjustments.push(penalty);
            concerns.push(format!(
                "Version {} is affected by {} advisory ({})",
                package, adv.severity, adv.summary
            ));
        }
    } else if version_safe {
        adjustments.push(10);
    }

    // Consensus check
    if total_submissions > 0 {
        if matches_consensus {
            if total_submissions >= 5 {
                adjustments.push(5);
            } else {
                adjustments.push(2);
            }
        } else {
            adjustments.push(-15);
            if hash_known {
                concerns.push(format!(
                    "Your hash has {} submissions, majority has {}",
                    hash_submissions, majority_submissions
                ));
            } else {
                concerns.push("Your PKGBUILD hash is not in any known snapshots".to_string());
            }
        }
    }

    // Version outdated check — only penalize for AUR/third-party packages
    // Official packages should not be penalized for having newer community versions in DB
    if is_outdated && !matches!(source, crate::backends::PackageSource::Official { .. }) {
        adjustments.push(-5);
        concerns.push("Newer version is known in the trust DB".to_string());
    }

    // Determine verdict
    let advisory_concern = version_affected;
    let consensus_concern = total_submissions > 0 && !matches_consensus;
    // Only flag as outdated for non-official packages (official packages use upstream versioning)
    let outdated_concern = is_outdated && !matches!(source, crate::backends::PackageSource::Official { .. });

    let verdict = match (
        advisory_concern,
        consensus_concern,
        outdated_concern,
        is_unknown_version && total_submissions == 0,
    ) {
        (true, _, _, _) => Verdict::AdvisoryHit,
        (_, true, _, _) => Verdict::Divergent,
        (_, _, true, _) => Verdict::Outdated,
        (false, false, false, true) => Verdict::Unknown,
        (false, false, false, false) => Verdict::Clean,
    };

    let total_adj: i32 = adjustments.iter().sum();

    let explanation = if concerns.is_empty() {
        if total_submissions == 0 {
            format!(
                "No community data yet for {} {} — first submission",
                package, version
            )
        } else if matches_consensus {
            format!(
                "Matches community consensus ({} submissions for this hash)",
                hash_submissions
            )
        } else {
            format!("Version {} — no concerns detected", version)
        }
    } else {
        concerns.join("; ")
    };

    (verdict, total_adj, explanation)
}

/// Format the pre-flight report for terminal display.
pub fn format_report(report: &PreFlightReport) -> String {
    use colored::Colorize;

    let mut lines = Vec::new();

    // Header
    lines.push(format!(
        "\n  {} {} {}",
        "Pre-flight Check:".cyan().bold(),
        report.package.white().bold(),
        report.incoming_version.dimmed()
    ));

    // Version intelligence
    if let Some(ref latest) = report.latest_known_version {
        if report.is_latest {
            lines.push(format!(
                "  {} {}",
                "Version:".cyan(),
                "Latest known (matches DB)".green()
            ));
        } else {
            lines.push(format!(
                "  {} {} → latest is {}",
                "Version:".cyan(),
                report.incoming_version.yellow(),
                latest.yellow()
            ));
        }
    } else {
        lines.push(format!(
            "  {} {}",
            "Version:".cyan(),
            "Not yet in trust DB".dimmed()
        ));
    }

    // Hash intelligence
    if report.total_submissions > 0 {
        if report.matches_consensus {
            lines.push(format!(
                "  {} {} ({}/{} submissions)",
                "Hash:".cyan(),
                "Matches consensus".green(),
                report.hash_submissions,
                report.total_submissions
            ));
        } else {
            lines.push(format!(
                "  {} {} (your: {} subs, majority: {} subs)",
                "Hash:".cyan(),
                "DIVERGES from consensus".red().bold(),
                report.hash_submissions,
                report.majority_submissions
            ));
        }
    } else {
        lines.push(format!(
            "  {} {}",
            "Hash:".cyan(),
            "No community data yet".dimmed()
        ));
    }

    // Advisory
    if report.version_affected {
        if let Some(ref adv) = report.advisory {
            lines.push(format!(
                "  {} {} ({})",
                "Advisory:".cyan(),
                format!("AFFECTED — {}", adv.severity).red().bold(),
                adv.summary
            ));
        }
    } else if report.version_safe {
        lines.push(format!(
            "  {} {}",
            "Advisory:".cyan(),
            "Version is marked safe".green()
        ));
    } else if report.advisory.is_some() {
        lines.push(format!(
            "  {} {}",
            "Advisory:".cyan(),
            "Package has advisory, but this version is not affected".yellow()
        ));
    }

    // Verdict
    let verdict_color = match report.verdict {
        Verdict::Clean => "green",
        Verdict::AdvisoryHit => "red",
        Verdict::Divergent => "red",
        Verdict::Outdated => "yellow",
        Verdict::Unknown => "dimmed",
        Verdict::Mixed => "yellow",
    };

    let verdict_text = format!("{:?}", report.verdict).to_uppercase();
    lines.push(format!(
        "  {} {}",
        "Verdict:".cyan().bold(),
        match verdict_color {
            "red" => verdict_text.red().bold(),
            "yellow" => verdict_text.yellow().bold(),
            "green" => verdict_text.green().bold(),
            _ => verdict_text.dimmed(),
        }
    ));

    lines.push(format!(
        "  {} {}",
        "Adjustment:".cyan(),
        if report.score_adjustment >= 0 {
            format!("+{}", report.score_adjustment).green()
        } else {
            format!("{}", report.score_adjustment).red()
        }
    ));

    lines.push(format!(
        "  {} {}",
        "Explanation:".cyan(),
        report.explanation
    ));

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_package_gets_unknown_verdict() {
        let report = preflight_check("nonexistent-pkg", "1.0.0-1", "some pkgbuild content", &crate::backends::PackageSource::Unknown);
        assert_eq!(report.verdict, Verdict::Unknown);
        assert!(!report.hash_known);
        assert!(report.should_submit);
    }

    #[test]
    fn verdict_display() {
        assert_eq!(format!("{}", Verdict::Clean), "Clean");
        assert_eq!(format!("{}", Verdict::Divergent), "Divergent");
    }
}
