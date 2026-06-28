use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::cache::Cache;
use crate::config;
use anyhow::Result;

use crate::backends::{self, PackageInfo, PackageSource};

/// How long cached search results stay valid.
const SEARCH_CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour

/// How long cached info results stay valid.
const INFO_CACHE_TTL: Duration = Duration::from_secs(86400); // 24 hours

/// Wrapper to attach a timestamp to cached data.
#[derive(Serialize, Deserialize)]
struct CachedEntry<T> {
    timestamp_secs: u64,
    data: T,
}

impl<T> CachedEntry<T> {
    fn new(data: T) -> Self {
        let timestamp_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            timestamp_secs,
            data,
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.timestamp_secs) > ttl.as_secs()
    }
}

/// Search official repositories and AUR, ranked by relevance then source.
pub fn search(cache: &Cache, query: &str) -> Result<Vec<PackageInfo>> {
    // Check if AUR is enabled BEFORE any cache lookups
    let aur_enabled = config::is_aur_enabled();

    let cache_key = format!("search:{query}");
    if let Some(cached) = cache.get_packages(&cache_key)? {
        if let Ok(entry) = serde_json::from_slice::<CachedEntry<Vec<PackageInfo>>>(&cached) {
            if !entry.is_expired(SEARCH_CACHE_TTL) {
                // Filter out AUR packages if AUR is disabled
                if !aur_enabled {
                    let filtered: Vec<PackageInfo> = entry
                        .data
                        .into_iter()
                        .filter(|p| !matches!(p.source, PackageSource::Aur))
                        .collect();
                    return Ok(filtered);
                }
                return Ok(entry.data);
            }
            // Cache expired — fall through to live search
        }
    }

    let mut by_name: HashMap<String, PackageInfo> = HashMap::new();

    if aur_enabled {
        // AUR first, then pacman — pacman overwrites on name collision (preferred)
        // Gracefully handle AUR failures so pacman results are still shown
        match backends::aur::search(query) {
            Ok(aur_pkgs) => {
                for pkg in aur_pkgs {
                    by_name.insert(pkg.name.clone(), pkg);
                }
            }
            Err(e) => {
                eprintln!("Warning: AUR search failed ({}). Showing official results only.", e);
            }
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

    // Cache the results with a timestamp
    if let Ok(serialized) = serde_json::to_vec(&CachedEntry::new(&packages)) {
        if let Err(e) = cache.insert_packages(&cache_key, serialized) {
            eprintln!("Warning: Cache write failed (search): {}", e);
        }
    }

    Ok(packages)
}

/// Resolve one package, using official repositories before falling back to AUR.
pub fn resolve(cache: &Cache, package: &str) -> Result<Option<PackageInfo>> {
    // Check if AUR is enabled BEFORE any cache lookups
    let aur_enabled = config::is_aur_enabled();

    let cache_key = format!("info:{package}");
    if let Some(cached) = cache.get_packages(&cache_key)? {
        if let Ok(entry) = serde_json::from_slice::<CachedEntry<PackageInfo>>(&cached) {
            if !entry.is_expired(INFO_CACHE_TTL) {
                // Skip cached AUR packages if AUR is disabled
                if !aur_enabled && matches!(entry.data.source, PackageSource::Aur) {
                    // Fall through to re-resolve from official sources only
                } else {
                    return Ok(Some(entry.data));
                }
            }
            // Cache expired — fall through to live resolve
        }
    }

    if let Some(pkg) = backends::pacman::info(package)? {
        if let Ok(serialized) = serde_json::to_vec(&CachedEntry::new(&pkg)) {
            if let Err(e) = cache.insert_packages(&cache_key, serialized) {
                eprintln!("Warning: Cache write failed (info): {}", e);
            }
        }
        return Ok(Some(pkg));
    }

    if aur_enabled {
        match backends::aur::info(package) {
            Ok(Some(pkg)) => {
                if let Ok(serialized) = serde_json::to_vec(&CachedEntry::new(&pkg)) {
                    if let Err(e) = cache.insert_packages(&cache_key, serialized) {
                        eprintln!("Warning: Cache write failed (info): {}", e);
                    }
                }
                return Ok(Some(pkg));
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("Warning: AUR lookup failed ({}).", e);
            }
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
    crate::backends::pacman::is_package_installed(package)
}

/// Fetch PKGBUILD for a package based on its source.
/// Currently only supports AUR packages; returns None for official/third-party.
pub fn fetch_pkgbuild_for_package(pkg: &PackageInfo) -> Result<Option<String>> {
    match &pkg.source {
        PackageSource::Aur => crate::backends::aur::fetch_pkgbuild(&pkg.name),
        PackageSource::Official { .. } | PackageSource::ThirdParty => {
            // For official packages, we could fetch from ABS but it's not available by default
            Ok(None)
        }
        PackageSource::Unknown => Ok(None),
    }
}
