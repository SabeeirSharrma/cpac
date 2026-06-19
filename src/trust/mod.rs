use chrono::Utc;

use crate::backends::{PackageInfo, PackageSource};

/// Trust tier classification.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
pub struct TrustSignal {
    pub name: String,
    pub points: i32,
    pub max_points: i32,
    pub detail: String,
}

/// Full trust analysis report for a package.
#[derive(Debug, Clone)]
pub struct TrustReport {
    pub package_name: String,
    pub tier: TrustTier,
    pub score: u32,
    pub signals: Vec<TrustSignal>,
    pub recommendation: String,
}

/// Compute a trust report for a package.
pub fn analyze(pkg: &PackageInfo) -> TrustReport {
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
        PackageSource::ThirdParty => 15,
        PackageSource::Aur => 10,
        PackageSource::Unknown => 0,
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
        let pts = match &pkg.source {
            PackageSource::Official { .. } => 13,
            _ => 0,
        };
        let detail = match &pkg.source {
            PackageSource::Official { .. } => {
                "Unknown (not tracked for official packages)".to_string()
            }
            _ => "Unknown".to_string(),
        };
        signals.push(TrustSignal {
            name: "Package Age".to_string(),
            points: pts,
            max_points: 15,
            detail,
        });
        pts
    };
    total += age_points;

    // --- Signal 3: Maintainer (max +15) ---
    let maintainer_points = if pkg.orphan {
        signals.push(TrustSignal {
            name: "Maintainer".to_string(),
            points: -5,
            max_points: 15,
            detail: "Orphaned — no active maintainer".to_string(),
        });
        -5
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
        signals.push(TrustSignal {
            name: "Maintainer".to_string(),
            points: 0,
            max_points: 15,
            detail: "Unknown".to_string(),
        });
        0
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
        let pts = match &pkg.source {
            PackageSource::Official { .. } => 12,
            _ => 0,
        };
        let detail = match &pkg.source {
            PackageSource::Official { .. } => {
                "Unknown (not tracked for official packages)".to_string()
            }
            _ => "Unknown".to_string(),
        };
        signals.push(TrustSignal {
            name: "Popularity".to_string(),
            points: pts,
            max_points: 15,
            detail,
        });
        pts
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
        let pts = match &pkg.source {
            PackageSource::Official { .. } => 12,
            _ => 0,
        };
        let detail = match &pkg.source {
            PackageSource::Official { .. } => {
                "Unknown (not tracked for official packages)".to_string()
            }
            _ => "Unknown".to_string(),
        };
        signals.push(TrustSignal {
            name: "Last Updated".to_string(),
            points: pts,
            max_points: 15,
            detail,
        });
        pts
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

    // Clamp to 0..100
    let score = total.clamp(0, 100) as u32;

    let recommendation = recommendation(score).to_string();

    TrustReport {
        package_name: pkg.name.clone(),
        tier,
        score,
        signals,
        recommendation,
    }
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
