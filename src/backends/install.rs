use anyhow::{Context, Result};
use std::process::Command;

use crate::backends::PackageSource;

/// Available backends for package installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallBackend {
    Pacman,
    Paru,
    Yay,
}

impl InstallBackend {
    /// Detect the best available AUR helper, preferring paru over yay.
    pub fn detect_aur() -> Option<Self> {
        if Command::new("paru").arg("--version").output().is_ok() {
            return Some(InstallBackend::Paru);
        }
        if Command::new("yay").arg("--version").output().is_ok() {
            return Some(InstallBackend::Yay);
        }
        None
    }

    /// Get the command name for this backend.
    pub fn cmd(&self) -> &'static str {
        match self {
            InstallBackend::Pacman => "pacman",
            InstallBackend::Paru => "paru",
            InstallBackend::Yay => "yay",
        }
    }

    /// Check if this backend can install from the given source.
    /// Currently unused but kept for potential future use in backend selection logic.
    #[allow(dead_code)]
    pub fn can_install(&self, source: &PackageSource) -> bool {
        match (self, source) {
            (InstallBackend::Pacman, PackageSource::Official { .. }) => true,
            (InstallBackend::Pacman, PackageSource::ThirdParty) => true,
            (InstallBackend::Paru | InstallBackend::Yay, PackageSource::Aur) => true,
            (InstallBackend::Paru | InstallBackend::Yay, PackageSource::Official { .. }) => true,
            (InstallBackend::Paru | InstallBackend::Yay, PackageSource::ThirdParty) => true,
            _ => false,
        }
    }
}

/// Select the appropriate backend for a package source.
pub fn select_backend(source: &PackageSource) -> Option<InstallBackend> {
    match source {
        PackageSource::Official { .. } | PackageSource::ThirdParty => Some(InstallBackend::Pacman),
        PackageSource::Aur => InstallBackend::detect_aur(),
        PackageSource::Unknown => None,
    }
}

/// Update package databases (pacman -Sy).
pub fn update_databases() -> Result<()> {
    let status = Command::new("pacman")
        .args(["-Sy"])
        .status()
        .context("Failed to run pacman -Sy")?;

    if !status.success() {
        anyhow::bail!("pacman -Sy failed with exit code: {}", status);
    }
    Ok(())
}

/// Install a package using the appropriate backend.
pub fn install_package(backend: InstallBackend, package: &str) -> Result<()> {
    let args = match backend {
        InstallBackend::Pacman => vec!["-S", "--noconfirm", package],
        InstallBackend::Paru => vec!["-S", "--noconfirm", package],
        InstallBackend::Yay => vec!["-S", "--noconfirm", package],
    };

    let status = Command::new(backend.cmd())
        .args(&args)
        .status()
        .with_context(|| format!("Failed to run {}", backend.cmd()))?;

    if !status.success() {
        anyhow::bail!("{} install failed with exit code: {}", backend.cmd(), status);
    }
    Ok(())
}

/// Remove a package using pacman.
pub fn remove_package(package: &str) -> Result<()> {
    let status = Command::new("pacman")
        .args(["-R", "--noconfirm", package])
        .status()
        .context("Failed to run pacman -R")?;

    if !status.success() {
        anyhow::bail!("pacman -R failed with exit code: {}", status);
    }
    Ok(())
}
