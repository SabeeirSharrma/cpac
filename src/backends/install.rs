use anyhow::{bail, Context, Result};
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
        if let Ok(output) = Command::new("paru").arg("--version").output() {
            if output.status.success() {
                return Some(InstallBackend::Paru);
            }
        }
        if let Ok(output) = Command::new("yay").arg("--version").output() {
            if output.status.success() {
                return Some(InstallBackend::Yay);
            }
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
        matches!(
            (self, source),
            (InstallBackend::Pacman, PackageSource::Official { .. })
                | (InstallBackend::Pacman, PackageSource::ThirdParty)
                | (InstallBackend::Paru | InstallBackend::Yay, PackageSource::Aur)
                | (InstallBackend::Paru | InstallBackend::Yay, PackageSource::Official { .. })
                | (InstallBackend::Paru | InstallBackend::Yay, PackageSource::ThirdParty)
        )
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
    let status = run_pacman(["-Sy"])?;

    if !status.success() {
        anyhow::bail!("pacman -Sy failed with exit code: {}", status);
    }
    Ok(())
}

/// Prompt for sudo credentials up front so privileged package operations can reuse them.
pub fn ensure_sudo() -> Result<()> {
    if is_running_as_root() {
        return Ok(());
    }

    let status = Command::new("sudo")
        .arg("-v")
        .status()
        .context("Failed to request sudo credentials")?;

    if !status.success() {
        bail!("sudo credential check failed with exit code: {}", status);
    }

    Ok(())
}

/// Install a package using the appropriate backend.
pub fn install_package(backend: InstallBackend, package: &str) -> Result<()> {
    let args = vec!["-S", "--noconfirm", package];

    let status = match backend {
        InstallBackend::Pacman => run_pacman(args)?,
        InstallBackend::Paru | InstallBackend::Yay => Command::new(backend.cmd())
            .args(&args)
            .status()
            .with_context(|| format!("Failed to run {}", backend.cmd()))?,
    };

    if !status.success() {
        anyhow::bail!(
            "{} install failed with exit code: {}",
            backend.cmd(),
            status
        );
    }
    Ok(())
}

/// Remove a package using pacman.
pub fn remove_package(package: &str, recursive: bool) -> Result<()> {
    let args = if recursive {
        vec!["-Rs", "--noconfirm", package]
    } else {
        vec!["-R", "--noconfirm", package]
    };

    let status = run_pacman(args)?;

    if !status.success() {
        anyhow::bail!(
            "pacman {} failed with exit code: {}",
            if recursive { "-Rs" } else { "-R" },
            status
        );
    }
    Ok(())
}

fn run_pacman<I, S>(args: I) -> Result<std::process::ExitStatus>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let mut command = if is_running_as_root() {
        Command::new("pacman")
    } else {
        let mut sudo = Command::new("sudo");
        sudo.arg("pacman");
        sudo
    };

    command.args(args).status().context("Failed to run pacman")
}

fn is_running_as_root() -> bool {
    Command::new("id")
        .args(["-u"])
        .output()
        .ok()
        .and_then(|output| {
            if !output.status.success() {
                return None;
            }

            std::str::from_utf8(&output.stdout)
                .ok()
                .map(|value| value.trim() == "0")
        })
        .unwrap_or(false)
}
