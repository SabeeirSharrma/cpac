use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::trust_db::{self, SnapshotEntry};

/// Result of comparing a PKGBUILD against known snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    /// Whether the hash matches the majority consensus.
    pub matches_consensus: bool,
    /// The computed hash of the local PKGBUILD.
    pub local_hash: String,
    /// Number of known snapshots for this package+version.
    pub known_snapshots: usize,
    /// Number of submissions for the matching hash (if any).
    pub matching_submissions: usize,
    /// Total submissions across all known hashes.
    pub total_submissions: usize,
    /// The majority hash (most common).
    pub majority_hash: Option<String>,
    /// Human-readable verdict.
    pub verdict: ConsensusVerdict,
    /// Brief explanation.
    pub explanation: String,
}

/// Quick verdict on consensus match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusVerdict {
    /// Hash matches the majority of submissions.
    Match,
    /// Hash matches a minority of submissions (divergence detected).
    Divergent,
    /// No snapshots exist for this package+version yet.
    Unknown,
    /// Hash matches, but with low confidence (few submissions).
    WeakMatch,
}

impl std::fmt::Display for ConsensusVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusVerdict::Match => write!(f, "Match"),
            ConsensusVerdict::Divergent => write!(f, "Divergent"),
            ConsensusVerdict::Unknown => write!(f, "Unknown"),
            ConsensusVerdict::WeakMatch => write!(f, "Weak Match"),
        }
    }
}

/// Compare a PKGBUILD's hash against known snapshots from the trust database.
///
/// This is a fast local check — no network requests, uses cached snapshot data.
/// Returns a ComparisonResult indicating whether the hash matches consensus.
pub fn compare_pkgbuild(content: &str, package: &str, version: &str) -> ComparisonResult {
    let local_hash = crate::sanitize::sha256_hash(content);

    // Look up snapshots for this package+version
    let snapshots = trust_db::lookup_snapshots_for_version(package, version)
        .unwrap_or_default();

    if snapshots.is_empty() {
        return ComparisonResult {
            matches_consensus: false,
            local_hash,
            known_snapshots: 0,
            matching_submissions: 0,
            total_submissions: 0,
            majority_hash: None,
            verdict: ConsensusVerdict::Unknown,
            explanation: format!("No community snapshots for {} {} yet", package, version),
        };
    }

    // Find the majority hash (most submissions)
    let majority = find_majority(&snapshots);
    let total_submissions: usize = snapshots.iter().map(|s| s.submitted_count as usize).sum();

    if let Some((ref majority_hash, majority_count)) = majority {
        if majority_hash == &local_hash {
            let confidence = if total_submissions < 5 {
                ConsensusVerdict::WeakMatch
            } else {
                ConsensusVerdict::Match
            };

            ComparisonResult {
                matches_consensus: true,
                local_hash,
                known_snapshots: snapshots.len(),
                matching_submissions: majority_count,
                total_submissions,
                majority_hash: Some(majority_hash.clone()),
                verdict: confidence,
                explanation: format!(
                    "Matches majority hash ({} of {} submissions)",
                    majority_count, total_submissions
                ),
            }
        } else {
            // Check if local hash exists in any snapshot
            let matching = snapshots.iter()
                .find(|s| s.sha256 == local_hash)
                .map(|s| s.submitted_count as usize)
                .unwrap_or(0);

            ComparisonResult {
                matches_consensus: false,
                local_hash,
                known_snapshots: snapshots.len(),
                matching_submissions: matching,
                total_submissions,
                majority_hash: Some(majority_hash.clone()),
                verdict: ConsensusVerdict::Divergent,
                explanation: if matching > 0 {
                    format!(
                        "Differs from majority (your hash: {} submissions, majority: {})",
                        matching, majority_count
                    )
                } else {
                    format!(
                        "Hash not in known snapshots (majority has {} submissions)",
                        majority_count
                    )
                },
            }
        }
    } else {
        ComparisonResult {
            matches_consensus: false,
            local_hash,
            known_snapshots: snapshots.len(),
            matching_submissions: 0,
            total_submissions,
            majority_hash: None,
            verdict: ConsensusVerdict::Unknown,
            explanation: "No clear consensus among snapshots".to_string(),
        }
    }
}

/// Find the majority hash (most submissions) from a list of snapshots.
fn find_majority(snapshots: &[SnapshotEntry]) -> Option<(String, usize)> {
    if snapshots.is_empty() {
        return None;
    }

    let mut best_hash = None;
    let mut best_count = 0;

    for snapshot in snapshots {
        if snapshot.submitted_count as usize > best_count {
            best_hash = Some(snapshot.sha256.clone());
            best_count = snapshot.submitted_count as usize;
        }
    }

    best_hash.map(|h| (h, best_count))
}

/// Get trust score adjustment based on consensus verdict.
pub fn consensus_adjustment(verdict: ConsensusVerdict) -> i32 {
    match verdict {
        ConsensusVerdict::Match => 5,        // Minor boost for consensus match
        ConsensusVerdict::WeakMatch => 2,    // Small boost, low confidence
        ConsensusVerdict::Divergent => -15,  // Significant penalty for divergence
        ConsensusVerdict::Unknown => 0,      // No adjustment
    }
}

/// Get recommendation floor based on consensus verdict.
pub fn consensus_floor(verdict: ConsensusVerdict) -> &'static str {
    match verdict {
        ConsensusVerdict::Divergent => "Caution",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_when_no_snapshots() {
        let result = compare_pkgbuild("pkgbuild content", "nonexistent-pkg", "1.0.0-1");
        assert_eq!(result.verdict, ConsensusVerdict::Unknown);
        assert_eq!(result.known_snapshots, 0);
    }

    #[test]
    fn consensus_adjustment_values() {
        assert_eq!(consensus_adjustment(ConsensusVerdict::Match), 5);
        assert_eq!(consensus_adjustment(ConsensusVerdict::Divergent), -15);
        assert_eq!(consensus_adjustment(ConsensusVerdict::Unknown), 0);
    }
}
