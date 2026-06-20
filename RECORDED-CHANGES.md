# CPAC v0.5.0 — Package Installation, Removal & Updates

## Overview

Version 0.5 introduces the core package management commands: `cpac install`, `cpac remove`, `cpac update`, and `cpac diff`. These commands integrate trust analysis with actual package operations, providing a complete workflow from trust evaluation to installation.

---

## Changes

### New Commands

#### `cpac install <package>`

- **Trust analysis first**: Shows full trust report before prompting for installation
- **AUR support**: Uses `paru` (preferred) or `yay` for AUR packages, `pacman` for official repos
- **PKGBUILD diffing on upgrades**: Caches PKGBUILDs after install; on upgrade, diffs new vs cached PKGBUILD and flags suspicious patterns (remote code execution, obfuscation, system path modifications, etc.)
- **Trust score adjustment**: Suspicious patterns add negative trust signals, lowering score
- **Flags**:
  - `--force`: Skip trust analysis and confirmation prompt
  - `--dry-run`: Show what would be installed without actually installing
- **AUR gating**: Respects `cpac aur enable/disable` setting

#### `cpac remove <package>`

- **Trust analysis before removal**: Shows trust report to inform user
- **Recursive removal**: `--recursive` flag removes unneeded dependencies (`pacman -Rs`)
- **Force flag**: `--force` skips confirmation prompt
- **Safety**: Won't remove packages that aren't installed

#### `cpac update`

- **Official databases**: Runs `pacman -Sy` to refresh official repositories
- **AUR databases**: Optional `--aur` flag runs `paru -Sy` or `yay -Sy` (prefers paru)
- **AUR gating**: Only updates AUR if enabled via `cpac aur enable`

#### `cpac diff <package>`

- **Local diffing**: Compares cached PKGBUILD (from previous install) against current PKGBUILD
- **Suspicious pattern detection**: Flags remote script execution, obfuscation, system path modifications, etc.
- **AUR support**: Fetches current PKGBUILD from AUR git repository
- **Upgrade awareness**: Shows what changed since last CPAC install

---

### PKGBUILD Diff Analysis (Local)

New suspicious pattern detection in `src/trust/mod.rs`:

- Remote script execution (`curl | sh`, `wget | bash`)
- Base64/hex decoding (obfuscation)
- Inline `eval`/`exec`/`source`
- Aggressive `rm -rf` outside pkgdir/srcdir
- Dynamic `pkgver` from network
- Language package manager installs (`pip install`, `npm install`, `cargo install` in build)
- System path modifications outside pkgdir

### New Backend Module

- `src/backends/install.rs` — `InstallBackend` enum (Pacman/Paru/Yay), backend selection, install/remove/update operations
- `src/backends/aur.rs` — Added `fetch_pkgbuild()` to retrieve PKGBUILD from AUR git

### Resolver Extensions

- `fetch_pkgbuild()` — Fetch PKGBUILD from appropriate source
- `is_installed()` — Check if package is installed
- PKGBUILD caching integration for diffing

### Trust Extensions

- `analyze_pkgbuild_diff()` — Compare two PKGBUILDs for suspicious changes
- `diff_to_signals()` — Convert diff findings to trust signals (negative points)
- `get_cached_pkgbuild()` / `cache_pkgbuild()` — PKGBUILD cache operations

---

## Verification

### Install with Trust Analysis

```bash
$ cpac install firefox --dry-run
# Shows trust report (80/100 SAFE), then:
[DRY RUN] Would install 'firefox' using pacman backend

$ cpac install google-chrome --dry-run
# Shows trust report (65/100 MODERATE), then:
[DRY RUN] Would install 'google-chrome' using yay backend
```

### AUR Disabled Blocks Install

```bash
$ cpac aur disable
$ cpac install google-chrome --dry-run
Error: Package 'google-chrome' not found in official repositories or AUR
```

### Update Command

```bash
cpac update          # Official only
cpac update --aur    # Official + AUR (if enabled)
```

### Diff Command (after install caches PKGBUILD)

```bash
cpac diff firefox    # Shows diff between cached and current PKGBUILD
```

### Remove with Trust Analysis

```bash
$ cpac remove firefox --force
# Shows trust report, then removes with pacman -R
```

### All Tests Pass

- 6/6 unit tests passing (repo classification, trust scoring, audit logic)
- Cargo check clean (only expected warnings about unused placeholder cache fields)

---

## Files Changed (v0.5)

- `Cargo.toml` — version bump to 0.5.0
- `src/main.rs` — added `install`, `remove`, `update`, `diff` modules
- `src/backends/install.rs` — new install backend module
- `src/backends/aur.rs` — added `fetch_pkgbuild()`, `build_aur_package()`
- `src/backends/mod.rs` — export `InstallBackend`
- `src/resolver/mod.rs` — `fetch_pkgbuild()`, `is_installed()`, PKGBUILD caching
- `src/trust/mod.rs` — PKGBUILD diff analysis, suspicious pattern detection, caching
- `src/install.rs` — new install command implementation
- `src/remove.rs` — new remove command implementation
- `src/update.rs` — new update command implementation
- `src/diff.rs` — new diff command implementation
- `src/cli/mod.rs` — updated command definitions and imports
