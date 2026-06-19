use anyhow::{Context, Result};
use chrono::DateTime;
use serde::Deserialize;

use super::{PackageInfo, PackageSource};

const AUR_RPC_URL: &str = "https://aur.archlinux.org/rpc/v5";

/// AUR RPC response envelope.
#[derive(Debug, Deserialize)]
struct AurResponse {
    #[serde(rename = "resultcount")]
    result_count: u32,
    results: Vec<AurPackage>,
}

/// Individual package from the AUR RPC.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AurPackage {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Version")]
    version: String,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "Maintainer")]
    maintainer: Option<String>,
    #[serde(rename = "NumVotes")]
    num_votes: Option<u32>,
    #[serde(rename = "Popularity")]
    popularity: Option<f64>,
    #[serde(rename = "FirstSubmitted")]
    first_submitted: Option<i64>,
    #[serde(rename = "LastModified")]
    last_modified: Option<i64>,
    #[serde(rename = "OutOfDate")]
    out_of_date: Option<i64>,
    #[serde(rename = "URL")]
    url: Option<String>,
    #[serde(rename = "License")]
    license: Option<Vec<String>>,
    #[serde(rename = "Depends")]
    depends: Option<Vec<String>>,
}

impl AurPackage {
    fn into_package_info(self) -> PackageInfo {
        let first_submitted = self
            .first_submitted
            .and_then(|ts| DateTime::from_timestamp(ts, 0));
        let last_modified = self
            .last_modified
            .and_then(|ts| DateTime::from_timestamp(ts, 0));
        let orphan = self.maintainer.is_none();

        PackageInfo {
            name: self.name,
            version: self.version,
            description: self.description.unwrap_or_default(),
            source: PackageSource::Aur,
            maintainer: self.maintainer,
            votes: self.num_votes,
            popularity: self.popularity,
            first_submitted,
            last_modified,
            out_of_date: self.out_of_date.is_some(),
            orphan,
            url: self.url,
            licenses: self.license.unwrap_or_default(),
            depends: self.depends.unwrap_or_default(),
            install_size: None,
        }
    }
}

/// Search AUR packages by keyword.
pub fn search(query: &str) -> Result<Vec<PackageInfo>> {
    let url = format!("{}/search/{}", AUR_RPC_URL, query);

    let response: AurResponse = reqwest::blocking::get(&url)
        .context("Failed to connect to AUR. Check your internet connection.")?
        .json()
        .context("Failed to parse AUR response")?;

    let packages = response
        .results
        .into_iter()
        .map(|p| p.into_package_info())
        .collect();

    Ok(packages)
}

/// Get detailed info for a specific AUR package.
pub fn info(package: &str) -> Result<Option<PackageInfo>> {
    let url = format!("{}/info/{}", AUR_RPC_URL, package);

    let response: AurResponse = reqwest::blocking::get(&url)
        .context("Failed to connect to AUR. Check your internet connection.")?
        .json()
        .context("Failed to parse AUR response")?;

    if response.result_count == 0 {
        return Ok(None);
    }

    Ok(response
        .results
        .into_iter()
        .next()
        .map(|p| p.into_package_info()))
}
