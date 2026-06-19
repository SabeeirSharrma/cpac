use anyhow::Result;
use dirs;
use sled::{Db, IVec, open};
use std::fs;
use std::path::{Path, PathBuf};

/// Initialize the cache directory and open sled databases.
pub fn init(cache_dir: Option<&Path>) -> Result<Cache> {
    let dir = if let Some(dir) = cache_dir {
        dir.to_path_buf()
    } else {
        dirs::home_dir()
            .expect("could not get home dir")
            .join(".cpac/cache")
    };
    std::fs::create_dir_all(&dir)?;

    let packages = open(dir.join("packages.db"))?;
    let trust = open(dir.join("trust.db"))?;
    let advisories = open(dir.join("advisories.db"))?;
    let pkgbuilds = open(dir.join("pkgbuilds.db"))?;

    Ok(Cache {
        _dir: dir,
        packages,
        trust,
        advisories,
        pkgbuilds,
    })
}

/// Handle to all cache databases.
pub struct Cache {
    _dir: PathBuf,
    pub packages: Db,
    pub trust: Db,
    pub advisories: Db,
    pub pkgbuilds: Db,
}

impl Cache {
    /// Simple getter/setter wrappers for demonstration.
    pub fn get_packages<K: AsRef<[u8]>>(&self, key: K) -> Option<Vec<u8>> {
        self
            .packages
            .get(key)
            .ok()
            .flatten()
            .map(|ivec| ivec.to_vec())
    }
    pub fn insert_packages<K: AsRef<[u8]>, V: AsRef<[u8]>>(&self, key: K, value: V) -> Result<()> {
        let ivec = IVec::from(value.as_ref());
        self.packages.insert(key, ivec)?;
        Ok(())
    }
    pub fn get_trust<K: AsRef<[u8]>>(&self, key: K) -> Option<Vec<u8>> {
        self
            .trust
            .get(key)
            .ok()
            .flatten()
            .map(|ivec| ivec.to_vec())
    }
    pub fn insert_trust<K: AsRef<[u8]>, V: AsRef<[u8]>>(&self, key: K, value: V) -> Result<()> {
        let ivec = IVec::from(value.as_ref());
        self.trust.insert(key, ivec)?;
        Ok(())
    }
    pub fn get_advisories<K: AsRef<[u8]>>(&self, key: K) -> Option<Vec<u8>> {
        self
            .advisories
            .get(key)
            .ok()
            .flatten()
            .map(|ivec| ivec.to_vec())
    }
    pub fn insert_advisories<K: AsRef<[u8]>, V: AsRef<[u8]>>(&self, key: K, value: V) -> Result<()> {
        let ivec = IVec::from(value.as_ref());
        self.advisories.insert(key, ivec)?;
        Ok(())
    }
    pub fn get_pkgbuilds<K: AsRef<[u8]>>(&self, key: K) -> Option<Vec<u8>> {
        self
            .pkgbuilds
            .get(key)
            .ok()
            .flatten()
            .map(|ivec| ivec.to_vec())
    }
    pub fn insert_pkgbuilds<K: AsRef<[u8]>, V: AsRef<[u8]>>(&self, key: K, value: V) -> Result<()> {
        let ivec = IVec::from(value.as_ref());
        self.pkgbuilds.insert(key, ivec)?;
        Ok(())
    }
}

/// Remove the entire cache directory.
pub fn clear_cache() -> Result<()> {
    let cache_dir = dirs::home_dir()
        .expect("could not get home dir")
        .join(".cpac/cache");
    if cache_dir.exists() {
        fs::remove_dir_all(cache_dir)?;
    }
    Ok(())
}