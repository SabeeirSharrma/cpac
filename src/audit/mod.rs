use std::collections::HashMap;

use crate::cache::Cache;
use anyhow::Result;

use crate::{
    backends::{aur, classify_repo, pacman, PackageInfo, PackageSource},
    trust::{self, TrustReport, TrustTier},
};

const AUR_BATCH_SIZE: usize = 50;

#[derive(Debug, Clone, Default)]
pub struct AuditCounts {
    pub official: usize,
    pub third_party: usize,
    pub community: usize,
    pub unknown: usize,
}

impl AuditCounts {
    pub fn total(&self) -> usize {
        self.official + self.third_party + self.community + self.unknown
    }
}

#[derive(Debug, Clone)]
pub struct AuditWarning {
    pub package_name: String,
    pub tier: TrustTier,
    pub score: u32,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct OfficialPackageNotice {
    pub package_name: String,
    pub repo: String,
}

#[derive(Debug, Clone)]
pub struct SystemAudit {
    pub counts: AuditCounts,
    pub official_notices: Vec<OfficialPackageNotice>,
    pub warnings: Vec<AuditWarning>,
}

/// Audit all installed packages on the system.
pub fn audit_system(cache: &Cache) -> Result<SystemAudit> {
    let packages = pacman::installed()?;
    let repo_map = pacman::repo_map()?;
    let foreign_names: Vec<String> = packages
        .iter()
        .filter(|pkg| !repo_map.contains_key(&pkg.name))
        .map(|pkg| pkg.name.clone())
        .collect();

    let mut aur_map = HashMap::new();
    for chunk in foreign_names.chunks(AUR_BATCH_SIZE) {
        let refs: Vec<&str> = chunk.iter().map(|name| name.as_str()).collect();
        if let Ok(packages) = aur::info_multi(&refs) {
            for pkg in packages {
                aur_map.insert(pkg.name.clone(), pkg);
            }
        }
    }

    let mut counts = AuditCounts::default();
    let mut official_notices = Vec::new();
    let mut warnings = Vec::new();

    for pkg in packages {
        let pkg = hydrate_package(pkg, &repo_map, &aur_map);
        let report = trust::analyze(cache, &pkg);
        increment_counts(&mut counts, &report.tier);

        if let PackageSource::Official { repo } = &pkg.source {
            if is_distro_specific_repo(repo) {
                official_notices.push(OfficialPackageNotice {
                    package_name: report.package_name.clone(),
                    repo: repo.clone(),
                });
            }
        }

        if report.score < 60 || matches!(report.tier, TrustTier::Unknown) {
            warnings.push(AuditWarning {
                package_name: report.package_name.clone(),
                tier: report.tier.clone(),
                score: report.score,
                reason: warning_reason(&report),
            });
        }
    }

    warnings.sort_by(|a, b| {
        a.score
            .cmp(&b.score)
            .then_with(|| a.package_name.cmp(&b.package_name))
    });
    official_notices.sort_by(|a, b| {
        a.repo
            .cmp(&b.repo)
            .then_with(|| a.package_name.cmp(&b.package_name))
    });

    Ok(SystemAudit {
        counts,
        official_notices,
        warnings,
    })
}

/// Audit a single installed package.
pub fn audit_package(cache: &Cache, name: &str) -> Result<Option<(PackageInfo, TrustReport)>> {
    let packages = pacman::installed()?;
    let repo_map = pacman::repo_map()?;
    let Some(pkg) = packages.into_iter().find(|pkg| pkg.name == name) else {
        return Ok(None);
    };

    let mut aur_map = HashMap::new();
    if !repo_map.contains_key(&pkg.name) {
        if let Ok(Some(aur_pkg)) = aur::info(&pkg.name) {
            aur_map.insert(aur_pkg.name.clone(), aur_pkg);
        }
    }

    let pkg = hydrate_package(pkg, &repo_map, &aur_map);
    let report = trust::analyze(cache, &pkg);

    Ok(Some((pkg, report)))
}

fn hydrate_package(
    mut pkg: PackageInfo,
    repo_map: &HashMap<String, String>,
    aur_map: &HashMap<String, PackageInfo>,
) -> PackageInfo {
    if let Some(repo) = repo_map.get(&pkg.name) {
        pkg.source = classify_repo(repo);
        return pkg;
    }

    if let Some(aur_pkg) = aur_map.get(&pkg.name) {
        return merge_aur_metadata(pkg, aur_pkg.clone());
    }

    pkg.source = PackageSource::Unknown;
    pkg
}

fn merge_aur_metadata(mut local: PackageInfo, remote: PackageInfo) -> PackageInfo {
    local.source = PackageSource::Aur;
    local.maintainer = remote.maintainer;
    local.votes = remote.votes;
    local.popularity = remote.popularity;
    local.first_submitted = remote.first_submitted;
    local.last_modified = remote.last_modified;
    local.out_of_date = remote.out_of_date;
    local.url = remote.url;
    local.licenses = remote.licenses;
    local.depends = remote.depends;
    local
}

fn increment_counts(counts: &mut AuditCounts, tier: &TrustTier) {
    match tier {
        TrustTier::Official => counts.official += 1,
        TrustTier::ThirdParty => counts.third_party += 1,
        TrustTier::Community => counts.community += 1,
        TrustTier::Unknown => counts.unknown += 1,
    }
}

fn warning_reason(report: &TrustReport) -> String {
    if matches!(report.tier, TrustTier::Unknown) {
        return "Installed outside CPAC".to_string();
    }

    let mut signals = report.signals.iter().collect::<Vec<_>>();
    signals.sort_by(|a, b| a.points.cmp(&b.points).then_with(|| a.name.cmp(&b.name)));

    signals
        .into_iter()
        .take(3)
        .map(|signal| format!("{}: {}", signal.name, signal.detail))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn is_distro_specific_repo(repo: &str) -> bool {
    repo == "endeavouros" || repo.starts_with("cachyos")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trust::{TrustReport, TrustSignal};

    #[test]
    fn unknown_packages_use_the_spec_summary() {
        let report = TrustReport {
            package_name: "mystery".to_string(),
            tier: TrustTier::Unknown,
            score: 0,
            signals: vec![],
            recommendation: "Danger".to_string(),
        };

        assert_eq!(warning_reason(&report), "Installed outside CPAC");
    }

    #[test]
    fn warning_reason_keeps_the_lowest_signals_first() {
        let report = TrustReport {
            package_name: "foo".to_string(),
            tier: TrustTier::Community,
            score: 34,
            signals: vec![
                TrustSignal {
                    name: "Popularity".to_string(),
                    points: 11,
                    max_points: 15,
                    detail: "101 votes".to_string(),
                },
                TrustSignal {
                    name: "Maintainer".to_string(),
                    points: -5,
                    max_points: 15,
                    detail: "Orphaned - no active maintainer".to_string(),
                },
                TrustSignal {
                    name: "Last Updated".to_string(),
                    points: 3,
                    max_points: 15,
                    detail: "2 years ago".to_string(),
                },
            ],
            recommendation: "Warning".to_string(),
        };

        assert_eq!(
            warning_reason(&report),
            "Maintainer: Orphaned - no active maintainer | Last Updated: 2 years ago | Popularity: 101 votes"
        );
    }

    #[test]
    fn official_arch_repos_are_official() {
        assert!(matches!(
            classify_repo("core"),
            PackageSource::Official { .. }
        ));
        assert!(matches!(
            classify_repo("extra"),
            PackageSource::Official { .. }
        ));
        assert!(matches!(
            classify_repo("multilib"),
            PackageSource::Official { .. }
        ));
        assert!(matches!(
            classify_repo("testing"),
            PackageSource::Official { .. }
        ));
        assert!(matches!(
            classify_repo("community"),
            PackageSource::Official { .. }
        ));
    }

    #[test]
    fn distro_specific_repos_are_third_party() {
        assert!(matches!(
            classify_repo("endeavouros"),
            PackageSource::ThirdParty
        ));
        assert!(matches!(
            classify_repo("cachyos"),
            PackageSource::ThirdParty
        ));
        assert!(matches!(
            classify_repo("cachyos-v3"),
            PackageSource::ThirdParty
        ));
        assert!(matches!(classify_repo("garuda"), PackageSource::ThirdParty));
        assert!(matches!(
            classify_repo("manjaro"),
            PackageSource::ThirdParty
        ));
    }

    #[test]
    fn other_third_party_repos_are_third_party() {
        assert!(matches!(
            classify_repo("chaotic-aur"),
            PackageSource::ThirdParty
        ));
        assert!(matches!(
            classify_repo("blackarch"),
            PackageSource::ThirdParty
        ));
    }

    #[test]
    fn distro_specific_repos_are_reported_for_exclusion_notes() {
        assert!(is_distro_specific_repo("endeavouros"));
        assert!(is_distro_specific_repo("cachyos"));
        assert!(is_distro_specific_repo("cachyos-v3"));
        assert!(!is_distro_specific_repo("core"));
        assert!(!is_distro_specific_repo("extra"));
    }
}
