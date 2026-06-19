pub mod aur;
pub mod pacman;

use chrono::{DateTime, Utc};
use std::fmt;

/// The source repository a package comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
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
