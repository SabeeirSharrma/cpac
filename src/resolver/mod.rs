use std::collections::HashMap;

use anyhow::Result;

use crate::backends::{self, PackageInfo, PackageSource};

/// Search official repositories and AUR, ranked by relevance then source.
pub fn search(query: &str) -> Result<Vec<PackageInfo>> {
    let mut by_name: HashMap<String, PackageInfo> = HashMap::new();

    // AUR first, then pacman — pacman overwrites on name collision (preferred)
    for pkg in backends::aur::search(query)? {
        by_name.insert(pkg.name.clone(), pkg);
    }

    for pkg in backends::pacman::search(query)? {
        by_name.insert(pkg.name.clone(), pkg);
    }

    let query_lower = query.to_lowercase();
    let mut packages: Vec<PackageInfo> = by_name.into_values().collect();

    packages.sort_by(|a, b| {
        let rank_a = relevance_rank(&a.name, &a.description, &query_lower);
        let rank_b = relevance_rank(&b.name, &b.description, &query_lower);

        rank_a
            .cmp(&rank_b)
            .then_with(|| source_rank(&a.source).cmp(&source_rank(&b.source)))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(packages)
}

/// Resolve one package, using official repositories before falling back to AUR.
pub fn resolve(package: &str) -> Result<Option<PackageInfo>> {
    if let Some(pkg) = backends::pacman::info(package)? {
        return Ok(Some(pkg));
    }

    backends::aur::info(package)
}

/// Relevance ranking: lower = more relevant.
///   0 = exact name match
///   1 = name starts with query
///   2 = name contains query
///   3 = description-only match
fn relevance_rank(name: &str, description: &str, query: &str) -> u8 {
    let name_lower = name.to_lowercase();
    if name_lower == *query {
        0
    } else if name_lower.starts_with(query) {
        1
    } else if name_lower.contains(query) {
        2
    } else if description.to_lowercase().contains(query) {
        3
    } else {
        4
    }
}

fn source_rank(source: &PackageSource) -> u8 {
    match source {
        PackageSource::Official { .. } => 0,
        PackageSource::ThirdParty => 1,
        PackageSource::Aur => 2,
        PackageSource::Unknown => 3,
    }
}
