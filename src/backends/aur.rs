use anyhow::{Context, Result};
use chrono::DateTime;
use serde::Deserialize;

use super::{PackageInfo, PackageSource};

const AUR_RPC_URL: &str = "https://aur.archlinux.org/rpc/";

/// AUR RPC response envelope.
#[derive(Debug, Deserialize)]
struct AurResponse {
    #[serde(rename = "resultcount")]
    result_count: u32,
    #[serde(default)]
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
    let client = reqwest::blocking::Client::new();
    let response: AurResponse = client
        .get(AUR_RPC_URL)
        .query(&[("v", "5"), ("type", "search"), ("arg", query)])
        .send()
        .context("Failed to connect to AUR. Check your internet connection.")?
        .error_for_status()
        .context("AUR returned an error response")?
        .json()
        .context("Failed to parse AUR response")?;

    let packages = response
        .results
        .into_iter()
        .map(|p| p.into_package_info())
        .collect();

    Ok(packages)
}

/// Get detailed info for multiple AUR packages in one RPC request.
pub fn info_multi(packages: &[&str]) -> Result<Vec<PackageInfo>> {
    if packages.is_empty() {
        return Ok(vec![]);
    }

    let client = reqwest::blocking::Client::new();
    let mut query = vec![("v", "5"), ("type", "info")];
    for package in packages {
        query.push(("arg[]", *package));
    }

    let response: AurResponse = client
        .get(AUR_RPC_URL)
        .query(&query)
        .send()
        .context("Failed to connect to AUR. Check your internet connection.")?
        .error_for_status()
        .context("AUR returned an error response")?
        .json()
        .context("Failed to parse AUR response")?;

    Ok(response
        .results
        .into_iter()
        .map(|p| p.into_package_info())
        .collect())
}

/// Get detailed info for a specific AUR package.
pub fn info(package: &str) -> Result<Option<PackageInfo>> {
    let client = reqwest::blocking::Client::new();
    let response: AurResponse = client
        .get(AUR_RPC_URL)
        .query(&[("v", "5"), ("type", "info"), ("arg", package)])
        .send()
        .context("Failed to connect to AUR. Check your internet connection.")?
        .error_for_status()
        .context("AUR returned an error response")?
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

/// Fetch the PKGBUILD content for an AUR package.
pub fn fetch_pkgbuild(package: &str) -> Result<Option<String>> {
    // AUR git repository URL pattern
    let url = format!(
        "https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={}",
        package
    );

    let response = reqwest::blocking::get(&url).context("Failed to connect to AUR for PKGBUILD")?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let content = response.text().context("Failed to read PKGBUILD content")?;

    if content.contains("404") || content.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(content))
}
