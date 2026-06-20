use std::collections::HashMap;

use anyhow::Result;
use crate::cache::Cache;
use crate::config;

use crate::backends::{self, PackageInfo, PackageSource};

/// Search official repositories and AUR, ranked by relevance then source.
pub fn search(cache: &Cache, query: &str) -> Result<Vec<PackageInfo>> {
    // Check if AUR is enabled BEFORE any cache lookups
    let aur_enabled = config::is_aur_enabled();

    let cache_key = format!("search:{query}");
    if let Some(cached) = cache.get_packages(&cache_key)? {
        if let Ok(pkgs) = serde_json::from_slice::<Vec<PackageInfo>>(&cached) {
            // Filter out AUR packages if AUR is disabled
            if !aur_enabled {
                let filtered: Vec<PackageInfo> = pkgs
                    .into_iter()
                    .filter(|p| !matches!(p.source, PackageSource::Aur))
                    .collect();
                return Ok(filtered);
            }
            return Ok(pkgs);
        }
    }

    let mut by_name: HashMap<String, PackageInfo> = HashMap::new();

    if aur_enabled {
        // AUR first, then pacman — pacman overwrites on name collision (preferred)
        for pkg in backends::aur::search(query)? {
            by_name.insert(pkg.name.clone(), pkg);
        }
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

    // Cache the results
    if let Ok(serialized) = serde_json::to_vec(&packages) {
        let _ = cache.insert_packages(&cache_key, serialized);
    }

    Ok(packages)
}

/// Resolve one package, using official repositories before falling back to AUR.
pub fn resolve(cache: &Cache, package: &str) -> Result<Option<PackageInfo>> {
    // Check if AUR is enabled BEFORE any cache lookups
    let aur_enabled = config::is_aur_enabled();

    let cache_key = format!("info:{package}");
    if let Some(cached) = cache.get_packages(&cache_key)? {
        if let Ok(pkg) = serde_json::from_slice::<PackageInfo>(&cached) {
            // Skip cached AUR packages if AUR is disabled
            if !aur_enabled && matches!(pkg.source, PackageSource::Aur) {
                // Fall through to re-resolve from official sources only
            } else {
                return Ok(Some(pkg));
            }
        }
    }

    if let Some(pkg) = backends::pacman::info(package)? {
        if let Ok(serialized) = serde_json::to_vec(&pkg) {
            let _ = cache.insert_packages(&cache_key, serialized);
        }
        return Ok(Some(pkg));
    }

    if aur_enabled {
        if let Some(pkg) = backends::aur::info(package)? {
            if let Ok(serialized) = serde_json::to_vec(&pkg) {
                let _ = cache.insert_packages(&cache_key, serialized);
            }
            return Ok(Some(pkg));
        }
    }

    Ok(None)
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

/// Check if a package is already installed.
pub fn is_installed(package: &str) -> Result<bool> {
    let installed = crate::backends::pacman::installed()?;
    Ok(installed.iter().any(|p| p.name == package))
}
