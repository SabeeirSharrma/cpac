# CPAC v0.5.0 Release Notes

## Overview
Version 0.5 introduces core package management commands: `cpac install`, `cpac remove`, `cpac update`, `cpac diff`, and `cpac config`. These integrate trust analysis with actual package operations, providing a complete workflow from trust evaluation to installation.

## New Commands

### `cpac install <package>`
- Trust analysis first: Shows full trust report before prompting for installation
- AUR support: Uses `paru` (preferred) or `yay` for AUR packages, `pacman` for official repos
- Sudo preflight: Prompts for sudo credentials before privileged install steps
- AUR behavior: Leaves `paru`/`yay` unwrapped, but primes sudo first so helpers can still complete package installation when they invoke `pacman`
- PKGBUILD diffing on upgrades: Caches PKGBUILDs after install; on upgrade, diffs new vs cached PKGBUILD and flags suspicious patterns
- Trust score adjustment: Suspicious patterns add negative trust signals, lowering score
- Flags:
  - `--force`: Skip trust analysis and confirmation prompt
  - `--dry-run`: Show what would be installed without actually installing
- AUR gating: Respects `cpac aur enable/disable` setting

### `cpac remove <package>`
- Trust analysis before removal: Shows trust report to inform user
- Recursive removal: `--recursive` flag removes unneeded dependencies (`pacman -Rs`)
- Sudo preflight: Prompts for sudo credentials before removal, including recursive removal
- Force flag: `--force` skips confirmation prompt
- Safety: Won't remove packages that aren't installed

### `cpac update`
- Official databases: Runs `pacman -Sy` to refresh official repositories
- AUR databases: Updates AUR automatically if enabled; use `--aur` to force AUR update when disabled
- Sudo preflight: Prompts for sudo credentials before syncing official package databases
- AUR gating: Updates AUR by default when enabled via `cpac aur enable`; `--aur` flag forces AUR update when disabled

### `cpac diff <package>`
- Local diffing: Compares cached PKGBUILD (from previous install) against current PKGBUILD
- Suspicious pattern detection: Flags remote script execution, obfuscation, system path modifications, etc.
- AUR support: Fetches current PKGBUILD from AUR git repository
- Upgrade awareness: Shows what changed since last CPAC install

### `cpac config`
- View current settings: Shows AUR support status and crowdsourced data submission level
- Interactive consent management: Choose submission level for crowdsourced PKGBUILD data
  - `[1]` No submission — don't send anything
  - `[2]` Hash/signature only (default)
  - `[3]` Full PKGBUILD — helps with better diff accuracy
- Stored locally: Consent choice persists in `~/.cpac/config.toml`
- Safe default: Pressing Enter with no input preserves the current setting

## PKGBUILD Diff Analysis (Local)
New suspicious pattern detection:
- Remote script execution (`curl | sh`, `wget | bash`)
- Base64/hex decoding (obfuscation)
- Inline `eval`/`exec` (removed overly broad `source` check)
- Aggressive `rm -rf` outside pkgdir/srcdir
- Dynamic `pkgver` from network
- Language package manager installs (`pip install`, `npm install`, `cargo install` in build)
- System path modifications outside pkgdir

## Backend & Resolver Extensions
- New `src/backends/install.rs` — `InstallBackend` enum (Pacman/Paru/Yay) with simplified match arms and exit status check for AUR detection
- `src/backends/aur.rs` — Added `fetch_pkgbuild()` with 30s timeout and proper HTTP error handling (404 returns None, 5xx returns error)
- `src/backends/pacman.rs` — Added `is_package_installed()` for efficient single-package check
- `src/prompt.rs` — New shared confirmation prompt utility with EOF handling
- Resolver extensions: `fetch_pkgbuild()`, `fetch_pkgbuild_for_package()`, `is_installed()` (optimized)
- Trust extensions: `analyze_pkgbuild_diff()` (LCS-based ordered diff), `diff_to_signals()` (fixed double-counting), PKGBUILD cache operations

## Verification Examples
```bash
# Install with trust analysis (dry run)
$ cpac install firefox --dry-run
# Shows trust report (80/100 SAFE), then:
[DRY RUN] Would install 'firefox' using pacman backend

$ cpac install google-chrome --dry-run
# Shows trust report (65/100 MODERATE), then:
[DRY RUN] Would install 'google-chrome' using yay backend

# AUR disabled blocks install
$ cpac aur disable
$ cpac install google-chrome --dry-run
Error: Package 'google-chrome' not found in official repositories or AUR

# Update command
cpac update          # Updates official + AUR (if enabled)
cpac update --aur    # Forces AUR update when disabled

# Diff command (after install caches PKGBUILD)
cpac diff firefox    # Shows diff between cached and current PKGBUILD

# Remove with trust analysis
$ cpac remove firefox --force
# Shows trust report, then removes with pacman -R

# Config command
$ cpac config
Current configuration:
  AUR support:        enabled
  Crowdsourced data:  Hash/signature only

Crowdsourced data submission
  [1] No, don't submit anything
  [2] Yes, hash/signature only  (default)
  [3] Yes, full PKGBUILD

Choice (Default: 2):
```

## Testing
- 9/9 unit tests passing (repo classification, trust scoring, audit logic)
- `cargo clippy` clean (only expected warnings about unused placeholder cache fields)

## Files Changed
- Version bump to 0.5.0 in Cargo.toml
- Added install, remove, update, diff, prompt modules in src/main.rs
- New backend modules: src/backends/install.rs, src/backends/aur.rs, src/prompt.rs
- Updated src/backends/mod.rs, src/backends/pacman.rs, src/resolver/mod.rs, src/trust/mod.rs
- New command implementations: src/install.rs, src/remove.rs, src/update.rs, src/diff.rs
- Added ConsentLevel enum and consent field in src/config/mod.rs
- CLI updates in src/cli/mod.rs
