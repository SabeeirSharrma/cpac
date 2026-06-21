use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentLevel {
    /// Don't submit anything.
    None,
    /// Submit hash/signature only.
    #[default]
    Hash,
    /// Submit full PKGBUILD text.
    Full,
}

impl ConsentLevel {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "No submission",
            Self::Hash => "Hash/signature only",
            Self::Full => "Full PKGBUILD",
        }
    }

    pub fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::None),
            2 => Some(Self::Hash),
            3 => Some(Self::Full),
            _ => None,
        }
    }
}

impl fmt::Display for ConsentLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct Config {
    pub aur_enabled: bool,
    #[serde(default)]
    pub consent: ConsentLevel,
}


fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not get home directory"))?;
    Ok(home.join(".cpac/config.toml"))
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
