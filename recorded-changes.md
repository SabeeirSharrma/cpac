# CPAC v0.3 – Metadata Cache & Clear Cache

## Overview
Version 0.3 introduces a local metadata cache to speed up package searches and trust analysis, and adds a manual cache‑clear command.

## Changes

### Metadata Cache
- CPAC now stores fetched package metadata in `~/.cpac/cache/` using SQLite‑like sled databases:
  - `packages.db` – source, maintainer, last update, popularity
  - `trust.db` – trust scores and audit history
  - `advisories.db` – known security advisories (placeholder for future use)
  - `pkgbuilds.db` – locally installed PKGBUILD snapshots (placeholder for future use)
- On each `cpac search` or `cpac trust`, CPAC first checks the cache; if a cached entry exists for the key, it is returned instantly without network requests. There is no TTL or freshness validation — cached data is reused unconditionally until explicitly cleared.
- Cache invalidation and update mechanisms (e.g., TTL, background refresh, `cpac update`) are not yet implemented.
- This makes repeated operations faster and enables basic offline functionality for previously cached queries.

### Clear Cache Command
- New subcommand: `cpac clear-cache`
- Deletes the entire `~/.cpac/cache/` directory, freeing disk space.
- The command appears automatically in `cpac --help`.
- If the cache does not exist, the command reports success; any errors are shown to the user.

### Code Adjustments
- Added `src/cache/mod.rs` with cache initialization, get/set helpers, and `clear_cache()` function.
- Updated `Cargo.toml` with dependencies: `sled`, `dirs`, `once_cell`.
- Modified `src/resolver/mod.rs` to cache search results and package info.
- Modified `src/trust/mod.rs` to cache trust reports.
- Updated `src/cli/mod.rs`:
  - Initialized a global cache (`CACHE`) using `once_cell`.
  - Passed the cache to resolver and trust functions.
  - Added handling for the `ClearCache` subcommand.
  - Updated audit module to accept the cache.
- Added `Serialize`/`Deserialize` derives to `PackageInfo` and `PackageSource` in `src/backends/mod.rs` for JSON caching.
- Fixed type mismatches between sled’s `IVec` and `Vec<u8>`.

## Impact
- **Speed:** Searches and trust reports are noticeably faster due to cached lookups.
- **Reliability:** Works better under slow or intermittent network conditions.
- **User Control:** Users can manually reclaim space used by the cache.
- **Foundation:** Sets the stage for future offline‑first features and more advanced trust calculations.
