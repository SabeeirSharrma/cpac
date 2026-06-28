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
cpac config set consent ...      # set data sharing level
cpac config set cache ...        # set auto-clear interval
cpac config reset                # reset to defaults
cpac config path                 # show config file location
cpac clear-cache                 # manually clear cache
```

## AUR Helpers

Paru is the preferred AUR helper, but yay is also supported. CPAC works with either — it detects what's installed and uses it for AUR operations.

## First Run

On first launch, CPAC asks about crowdsourced data sharing (opt-in, anonymous). This can be changed anytime via `cpac config set consent`.

## Trust DB Integration

CPAC connects to the cpac-trust-db backend via a Cloudflare Worker proxy at `api.thecinderproject.qd.je` for real-time trust data:
- **Advisory warnings** — color-coded alerts when installing known-malicious packages
- **Snapshot submissions** — anonymized PKGBUILD data shared with the community (consent-aware)
- **Auto-sync** — trust data refreshes automatically when stale

Data is stored locally at `~/.cpac/trust-db/` for offline use.

## Releases

GitHub Releases include pre-built binaries for x86_64 and aarch64 Linux alongside SHA-256 checksums.

> **⚠ These binaries are provided for checksum verification only — do not download or use them directly.**
> CPAC must be built from source to ensure transparency and reproducibility. Use the [install script](https://thecinderproject.qd.je/cpac/install.sh) or build manually.

## Auto Cache Clearing

CPAC automatically clears its metadata cache on a configurable interval (daily/weekly/monthly, default monthly). Set it with `cpac config set cache`.

## Requirements

- Arch-based Linux distribution
- `pacman` available on `PATH`
- Network access for AUR search and trust DB sync

## Trust Scoring

The trust algorithm uses metadata available from official repositories and the AUR, plus real-time trust DB data:

- repository source
- package age
- maintainer status
- votes/popularity
- update recency
- out-of-date and orphan status
- PKGBUILD diff results
- **advisory warnings** (known malicious/compromised packages)
- **snapshot divergence** (hash comparison with crowdsourced data)
- **anomaly detection** (suspicious PKGBUILD patterns including npm/bun pipe-to-shell)

When a package has no data in the trust DB, local scoring still runs normally. Missing DB data is shown as neutral signals (+0), not penalties.

See [docs/trust-algorithm.md](docs/trust-algorithm.md) for details.

## Made By

**Developer/Maintainer: [Sabeeir Sharrma](https://github.com/SabeeirSharrma)**

**Made under [The Cinder Project - Burn all the Blind Spots](https://thecinderproject.qd.je/)**
