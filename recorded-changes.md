# CPAC v0.4.0 — AUR/Install Support & Repo Hardening

## Overview

Version 0.4 introduces AUR enable/disable configuration, comprehensive repo classification for official Arch vs third-party/distro repositories, and fail-safe defaults for AUR access.

---

## Changes

### AUR Enable/Disable Configuration

- **New config file**: `~/.cpac/config.toml` with `aur_enabled` boolean
- **Commands**: `cpac aur enable` / `cpac aur disable`
- **Fail-closed default**: AUR defaults to **disabled** when no config exists or config is malformed
- **Recovery**: Malformed config files are gracefully handled — CLI recovers by writing a valid config
- **Cache enforcement**: AUR gate runs **before** cache lookups in both `search` and `resolve`, preventing cached AUR packages from leaking when AUR is disabled

### Official Repo Classification (Hardening)

- **Official Arch repos** (core, extra, multilib, testing variants, community) → `PackageSource::Official` / `TrustTier::Official` (+30 points)
- **Distro-specific repos** (EndeavourOS, CachyOS, Garuda, Manjaro, CinderOS) → `PackageSource::ThirdParty` / `TrustTier::ThirdParty` (+15 points)
- **Other third-party** (chaotic-aur, blackarch, etc.) → `PackageSource::ThirdParty`
- **AUR** → `PackageSource::Aur` / `TrustTier::Community` (+10 points)
- Centralized in `src/backends/mod.rs`: `is_official_arch_repo()` and `classify_repo()`
- Updated `pacman.rs` search/info and `audit.rs` hydrate to use shared classification

### Search Improvements (from v0.1.x polish)

- **Ranking**: exact match → prefix → contains → description-only
- **Default limit**: 25 results with "Showing X of Y results. Use --all to view everything."
- **`--all` flag** to show all results

### Trust Display Improvements

- **Prominent score box** with colored borders (green ≥70, yellow 40-69, red <40)
- **Unknown data** shows explicit "Unknown (Reason: Metadata unavailable)" instead of vague strings
- **Recommendation labels**: SAFE / MODERATE / CAUTION / WARNING / DANGER

### Version & Help

- `cpac --version` outputs `cpac 0.4.0 — A package trust layer for Arch-based Linux`
- `cpac --help` shows all commands with tagline

### Cache (v0.3 carried forward)

- Sled-based cache at `~/.cpac/cache/` with `packages.db`, `trust.db`, `advisories.db`, `pkgbuilds.db`
- `cpac clear-cache` command to wipe cache

---

## Verification

### AUR Disabled (default, no config)

```bash
$ cpac trust google-chrome
Error: Package 'google-chrome' was not found in official repositories or the AUR
$ cpac search google-chrome
No packages found.
```

### AUR Enabled

```bash
$ cpac aur enable
AUR support enabled.
$ cpac trust google-chrome
Repository: AUR
Trust Tier: Community
Trust Score: 65/100
Recommendation: MODERATE
```

### Official Arch Package (firefox from extra)

```bash
$ cpac trust firefox
Repository: Official (extra)
Trust Tier: Official
Trust Score: 80/100
Recommendation: SAFE
```

### Third-Party Distro Package (yay from EndeavourOS)

```bash
$ cpac trust yay
Repository: Third Party
Trust Tier: Third Party
Trust Score: 28/100
Recommendation: WARNING
```

### System Audit

```bash
$ cpac audit
Installed Packages: 1039
Official: 1020
Third Party: 14      # Includes EndeavourOS packages
Community: 17
Unknown: 0
```

### All Tests Pass

- 6/6 unit tests passing (repo classification, trust scoring, audit logic)
- Cargo check clean (only expected warnings about unused placeholder cache fields)

---

## Files Changed

- `Cargo.toml` — version bump to 0.4.0, added `toml` dependency
- `src/main.rs` — added `config` module
- `src/config/mod.rs` — new config module with fail-closed AUR default
- `src/backends/mod.rs` — `is_official_arch_repo()`, `classify_repo()`
- `src/backends/pacman.rs` — uses `classify_repo()` for search/info
- `src/resolver/mod.rs` — AUR gate before cache lookups in search/resolve
- `src/audit/mod.rs` — uses `classify_repo()` in hydrate
- `src/cli/mod.rs` — imports config, implements `aur` subcommand
