# CPAC

Community Package Analysis Client: a package trust layer for Arch-based Linux distributions.

CPAC is an advisor, not a package manager replacement. It helps answer whether a package looks trustworthy before you install it.

## Install

```bash
curl -sSf https://thecinderproject.qd.je/cpac/install.sh | bash
```

This builds cpac from source and installs it to `/usr/local/bin`. If Rust isn't already installed, it's added temporarily and removed after — no leftover dependencies.

See [docs/installation.md](docs/installation.md) for other install methods (AUR, manual build).

## Usage

Search official repositories and the AUR:

```bash
cpac search firefox
```

Show a trust report:

```bash
cpac trust firefox
```

All commands:

```bash
cpac search <query>              # search across all sources
cpac trust <package>             # full trust report
cpac audit [package]             # system-wide or per-package audit
cpac install <package>           # trust-gated install with PKGBUILD diffing
cpac remove <package>            # trust-gated removal
cpac update [--aur]              # refresh official + AUR databases
cpac diff <package>              # local PKGBUILD diff
cpac config show                 # show current configuration
cpac config set aur on|off       # enable/disable AUR
cpac config set cache ...        # set auto-clear interval
cpac config reset                # reset to defaults
cpac config path                 # show config file location
cpac clear-cache                 # manually clear cache
cpac upgrade                     # self-update from GitHub
```

## AUR Helpers

Paru is the preferred AUR helper, but yay is also supported. CPAC works with either — it detects what's installed and uses it for AUR operations.

## Releases

GitHub Releases include pre-built binaries for x86_64 and aarch64 Linux alongside SHA-256 checksums.

> **These binaries are provided for checksum verification only — do not download or use them directly.**
> CPAC must be built from source to ensure transparency and reproducibility. Use the [install script](https://thecinderproject.qd.je/cpac/install.sh) or build manually.

## Auto Cache Clearing

CPAC automatically clears its metadata cache on a configurable interval (daily/weekly/monthly, default monthly). Set it with `cpac config set cache`.

## Requirements

- Arch-based Linux distribution
- `pacman` available on `PATH`
- Network access for AUR search (optional, can be disabled)

## Trust Scoring

CPAC v1 uses **local signals only** — no network dependency for trust analysis:

- **Repository source** — Official (+30), ThirdParty (+10), AUR (+5), Unknown (-5)
- **Package age** — older packages score higher
- **Maintainer status** — active maintainer (+10-13), orphaned (-10), unknown (-3)
- **Popularity** — vote count mapped to trust score
- **Update recency** — recently updated packages score higher
- **Out-of-date flag** — flagged packages penalized (-10)
- **PKGBUILD diffing** — detects suspicious changes on upgrade (local, no network)
- **Anomaly detection** — catches curl|sh, base64 decode, eval/exec, rm -rf, npm/bun pipe-to-shell

> Note: Community trust data (cpac-trust-db) is not yet available in this release. When available, it will be an opt-in feature.

See [docs/trust-algorithm.md](docs/trust-algorithm.md) for details.

## Made By

**Developer/Maintainer: [Sabeeir Sharrma](https://github.com/SabeeirSharrma)**

**Made under [The Cinder Project - Burn all the Blind Spots](https://thecinderproject.qd.je/)**
