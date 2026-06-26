---
title: Introduction
description: Overview of the CPAC package trust layer.
order: 1
---

# CPAC

**Community Package Analysis Client** -- a package trust layer for Arch-based Linux distributions.

While traditional package managers answer *"Can I install this?"*, CPAC answers ***"Should I trust this?"***

## What is CPAC?

CPAC is a standalone package trust and advisory tool for Arch-based Linux. It works on Arch, EndeavourOS, Garuda, CachyOS, Manjaro, and any other Arch-based distribution.

It is **not** a package manager replacement. CPAC is a **package advisor** -- it sits in front of `pacman` (and optionally AUR helpers) and provides trust scoring, PKGBUILD analysis, and system auditing before you install or upgrade anything.

## Philosophy

- CPAC is a **package advisor**, not a gatekeeper
- The user always has the final say
- Trust scores are transparent and explainable
- CPAC never blocks standard tools like `yay` or `paru`
- Designed to remain useful regardless of the future state of AUR

## Features

### Trust Scoring

Every package receives a score from 0 to 100 based on multiple signals: repository source, package age, maintainer status, popularity, update recency, and out-of-date flags. Scores are always explainable -- `cpac trust <package>` shows each signal, its points, and the reason.

### PKGBUILD Diffing

CPAC detects suspicious modifications to PKGBUILD build scripts before install or upgrade. On upgrades, it diffs the new PKGBUILD against the cached version and flags dangerous patterns like remote code execution, obfuscation, and system path modifications.

### System Audit

`cpac audit` scans all installed packages, classifies them by trust tier (Official, ThirdParty, Community, Unknown), and surfaces warnings for low-scoring or unverified packages.

### AUR Support (Opt-in)

AUR support is disabled by default. Enable it with `cpac aur enable` to extend search, trust, and install across the Arch User Repository.

### Metadata Cache

CPAC maintains a local cache at `~/.cpac/cache/` using sled for fast, offline-capable lookups. Search results, trust reports, and PKGBUILD snapshots are cached automatically.

## Quick Start

```bash
# Search for a package
cpac search firefox

# Check a package's trust score
cpac trust firefox

# Install with trust analysis
cpac install firefox

# Audit all installed packages
cpac audit
```

## Requirements

- Arch-based Linux distribution
- `pacman` available on `PATH`
- Network access for AUR search and trust metadata

## Related Projects

| Project | Role |
| --- | --- |
| [cinderos](https://github.com/SabeeirSharrma/cinderos) | Reference implementation -- ships CPAC by default |
| [cpac-trust-db](https://github.com/SabeeirSharrma/cpac-trust-db) | Community-maintained advisory data + opt-in crowdsourced PKGBUILD snapshots |
| [website](https://github.com/SabeeirSharrma/website) | Project site |

## Made By

**Developer/Maintainer:** [Sabeeir Sharrma](https://github.com/SabeeirSharrma)

**Made under [The Cinder Project](https://thecinderproject.qd.je/)** -- *Making a safer internet*
