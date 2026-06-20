pub mod aur;
pub mod install;
pub mod pacman;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

pub use install::InstallBackend;

/// The source repository a package comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum PackageSource {
    Official { repo: String },
    Aur,
    ThirdParty,
    Unknown,
}

impl fmt::Display for PackageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageSource::Official { repo } => write!(f, "Official ({})", repo),
            PackageSource::Aur => write!(f, "AUR"),
            PackageSource::ThirdParty => write!(f, "Third Party"),
            PackageSource::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Unified package information from any backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: PackageSource,
    pub maintainer: Option<String>,
    pub votes: Option<u32>,
    pub popularity: Option<f64>,
    pub first_submitted: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub out_of_date: bool,
    pub orphan: bool,
    pub url: Option<String>,
    pub licenses: Vec<String>,
    pub depends: Vec<String>,
    pub install_size: Option<String>,
}

/// Official Arch Linux repository names.
const OFFICIAL_ARCH_REPOS: &[&str] = &[
    "core",
    "extra",
    "multilib",
    "testing",
    "core-testing",
    "extra-testing",
    "multilib-testing",
    "community",
    "community-testing",
];

/// Check if a repository is an official Arch Linux repository.
pub fn is_official_arch_repo(repo: &str) -> bool {
    OFFICIAL_ARCH_REPOS.contains(&repo)
}

/// Classify a repository into the appropriate PackageSource.
pub fn classify_repo(repo: &str) -> PackageSource {
    if is_official_arch_repo(repo) {
        PackageSource::Official { repo: repo.to_string() }
    } else {
        PackageSource::ThirdParty
    }
}
