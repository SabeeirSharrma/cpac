use anyhow::Result;
use dirs;
use sled::{open, Db, IVec};
use std::fs;
use std::path::{Path, PathBuf};

/// Initialize the cache directory and open sled databases.
pub fn init(cache_dir: Option<&Path>) -> Result<Cache> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not get home directory"))?;

    let dir = if let Some(dir) = cache_dir {
        dir.to_path_buf()
    } else {
        home_dir.join(".cpac/cache")
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
    pub fn get_packages<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>> {
        Ok(self.packages.get(key)?.map(|ivec| ivec.to_vec()))
    }
    pub fn insert_packages<K: AsRef<[u8]>, V: AsRef<[u8]>>(&self, key: K, value: V) -> Result<()> {
        let ivec = IVec::from(value.as_ref());
        self.packages.insert(key, ivec)?;
        Ok(())
    }
    pub fn get_trust<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>> {
        Ok(self.trust.get(key)?.map(|ivec| ivec.to_vec()))
    }
    pub fn insert_trust<K: AsRef<[u8]>, V: AsRef<[u8]>>(&self, key: K, value: V) -> Result<()> {
        let ivec = IVec::from(value.as_ref());
        self.trust.insert(key, ivec)?;
        Ok(())
    }
    pub fn get_advisories<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>> {
        Ok(self.advisories.get(key)?.map(|ivec| ivec.to_vec()))
    }
    pub fn insert_advisories<K: AsRef<[u8]>, V: AsRef<[u8]>>(
        &self,
        key: K,
        value: V,
    ) -> Result<()> {
        let ivec = IVec::from(value.as_ref());
        self.advisories.insert(key, ivec)?;
        Ok(())
    }
    pub fn get_pkgbuilds<K: AsRef<[u8]>>(&self, key: K) -> Result<Option<Vec<u8>>> {
        Ok(self.pkgbuilds.get(key)?.map(|ivec| ivec.to_vec()))
    }
    pub fn insert_pkgbuilds<K: AsRef<[u8]>, V: AsRef<[u8]>>(&self, key: K, value: V) -> Result<()> {
        let ivec = IVec::from(value.as_ref());
        self.pkgbuilds.insert(key, ivec)?;
        Ok(())
    }

    /// Clear package metadata that depends on repository state.
    pub fn clear_metadata(&self) -> Result<()> {
        self.packages.clear()?;
        self.trust.clear()?;
        self.pkgbuilds.clear()?;
        Ok(())
    }
}

/// Remove the entire cache directory.
pub fn clear_cache() -> Result<()> {
    let cache_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not get home directory"))?
        .join(".cpac/cache");
    // Remove directory and ignore "not found" errors (race condition safe)
    let _ = fs::remove_dir_all(&cache_dir);
    Ok(())
}
