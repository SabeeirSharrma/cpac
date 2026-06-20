use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub aur_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aur_enabled: true,
        }
    }
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
    let mut config = load()?;
    config.aur_enabled = enabled;
    save(&config)
}

pub fn is_aur_enabled() -> bool {
    load().map(|c| c.aur_enabled).unwrap_or(true)
}