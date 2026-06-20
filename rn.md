# CPAC v0.5.0 Release Notes

## Overview
Version 0.5 introduces core package management commands: `cpac install`, `cpac remove`, `cpac update`, and `cpac diff`. These integrate trust analysis with actual package operations, providing a complete workflow from trust evaluation to installation.

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
- AUR databases: Optional `--aur` flag runs `paru -Sy` or `yay -Sy` (prefers paru)
- Sudo preflight: Prompts for sudo credentials before syncing official package databases
- AUR gating: Only updates AUR if enabled via `cpac aur enable`

### `cpac diff <package>`
- Local diffing: Compares cached PKGBUILD (from previous install) against current PKGBUILD
- Suspicious pattern detection: Flags remote script execution, obfuscation, system path modifications, etc.
- AUR support: Fetches current PKGBUILD from AUR git repository
- Upgrade awareness: Shows what changed since last CPAC install

## PKGBUILD Diff Analysis (Local)
New suspicious pattern detection:
- Remote script execution (`curl | sh`, `wget | bash`)
- Base64/hex decoding (obfuscation)
- Inline `eval`/`exec`/`source`
- Aggressive `rm -rf` outside pkgdir/srcdir
- Dynamic `pkgver` from network
- Language package manager installs (`pip install`, `npm install`, `cargo install` in build)
- System path modifications outside pkgdir

## Backend & Resolver Extensions
- New `src/backends/install.rs` — `InstallBackend` enum (Pacman/Paru/Yay)
- `src/backends/aur.rs` — Added `fetch_pkgbuild()` to retrieve PKGBUILD from AUR git
- Resolver extensions: `fetch_pkgbuild()`, `is_installed()`, PKGBUILD caching
- Trust extensions: `analyze_pkgbuild_diff()`, `diff_to_signals()`, PKGBUILD cache operations

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
cpac update          # Official only
cpac update --aur    # Official + AUR (if enabled)

# Diff command (after install caches PKGBUILD)
cpac diff firefox    # Shows diff between cached and current PKGBUILD

# Remove with trust analysis
$ cpac remove firefox --force
# Shows trust report, then removes with pacman -R
```

## Testing
- All unit tests pass
- `cargo check` clean (expected warnings only about unused placeholder cache fields)

## Files Changed
- Version bump to 0.5.0 in Cargo.toml
- Added install, remove, update, diff modules in src/main.rs
- New backend modules: src/backends/install.rs, src/backends/aur.rs
- Updated src/backends/mod.rs, src/resolver/mod.rs, src/trust/mod.rs
- New command implementations: src/install.rs, src/remove.rs, src/update.rs, src/diff.rs
- CLI updates in src/cli/mod.rs
