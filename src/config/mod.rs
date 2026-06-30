use anyhow::Result;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "trust-db")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "kebab-case")]
pub enum ConsentLevel {
    /// Don't submit anything.
    None,
    /// Submit hash/signature only.
    #[default]
    Hash,
    /// Submit full PKGBUILD text.
    Full,
}

#[cfg(feature = "trust-db")]
impl ConsentLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "No submission",
            Self::Hash => "Hash/signature only",
            Self::Full => "Full PKGBUILD",
        }
    }

    #[allow(dead_code)]
    pub fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::None),
            2 => Some(Self::Hash),
            3 => Some(Self::Full),
            _ => None,
        }
    }
}

#[cfg(feature = "trust-db")]
impl fmt::Display for ConsentLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "kebab-case")]
pub enum CacheInterval {
    Daily,
    Weekly,
    #[default]
    Monthly,
}

impl CacheInterval {
    pub fn label(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }

    /// Returns the interval in seconds.
    pub fn as_secs(self) -> u64 {
        match self {
            Self::Daily => 86400,
            Self::Weekly => 604800,
            Self::Monthly => 2592000, // 30 days
        }
    }
}

impl fmt::Display for CacheInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_aur_enabled")]
    pub aur_enabled: bool,
    #[cfg(feature = "trust-db")]
    #[serde(default)]
    pub consent: ConsentLevel,
    #[serde(default)]
    pub cache_interval: CacheInterval,
    #[serde(default)]
    pub last_cache_clear: u64,
    #[serde(default)]
    pub first_run_done: bool,
    #[serde(default)]
    pub last_update_check: u64,
    #[serde(default)]
    pub cached_latest_version: Option<String>,
}

fn default_aur_enabled() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aur_enabled: default_aur_enabled(),
            #[cfg(feature = "trust-db")]
            consent: ConsentLevel::default(),
            cache_interval: CacheInterval::default(),
            last_cache_clear: 0,
            first_run_done: false,
            last_update_check: 0,
            cached_latest_version: None,
        }
    }
}

fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not get home directory"))?;
    Ok(home.join(".cpac/config.toml"))
}

/// Return the path to the config file.
pub fn path() -> Result<PathBuf> {
    config_path()
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn set_aur_enabled(enabled: bool) -> Result<()> {
    let mut config = load().unwrap_or_default();
    config.aur_enabled = enabled;
    save(&config)
}

pub fn is_aur_enabled() -> bool {
    load().map(|c| c.aur_enabled).unwrap_or(false)
}

#[cfg(feature = "trust-db")]
#[allow(dead_code)]
pub fn set_consent(consent: ConsentLevel) -> Result<()> {
    let mut config = load().unwrap_or_default();
    config.consent = consent;
    save(&config)
}

pub fn set_cache_interval(interval: CacheInterval) -> Result<()> {
    let mut config = load().unwrap_or_default();
    config.cache_interval = interval;
    save(&config)
}

/// Check if the cache should be cleared based on the configured interval.
/// Returns true if the cache was cleared.
pub fn maybe_clear_cache() -> Result<bool> {
    let mut config = load().unwrap_or_default();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let interval_secs = config.cache_interval.as_secs();

    if now.saturating_sub(config.last_cache_clear) > interval_secs {
        crate::cache::clear_cache()?;
        config.last_cache_clear = now;
        save(&config)?;
        return Ok(true);
    }

    Ok(false)
}

/// Mark the first-run consent prompt as completed.
#[cfg(feature = "trust-db")]
#[allow(dead_code)]
pub fn mark_first_run_done() -> Result<()> {
    let mut config = load().unwrap_or_default();
    config.first_run_done = true;
    save(&config)
}

/// Returns true if the first-run consent prompt has been shown.
#[cfg(feature = "trust-db")]
#[allow(dead_code)]
pub fn is_first_run_done() -> bool {
    load().map(|c| c.first_run_done).unwrap_or(false)
}

/// Get current time in seconds since UNIX epoch.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Record that we checked for updates and cache the latest version found.
pub fn set_update_check(latest_version: &str) -> Result<()> {
    let mut config = load().unwrap_or_default();
    config.last_update_check = now_secs();
    config.cached_latest_version = Some(latest_version.to_string());
    save(&config)
}
