# CPAC v0.7.0 Release Notes

## Overview

v0.7.0 integrates the cpac-trust-db into CPAC via a Cloudflare Worker proxy at `api.thecinderproject.qd.je`, adding real-time advisory warnings, PKGBUILD sanitization, anomaly detection, snapshot submission, and a transparent curl-based installer.

## New Features

### Trust DB Integration via API Proxy

CPAC communicates with the trust DB backend through a Cloudflare Worker proxy at `api.thecinderproject.qd.je`:
- Meta check (staleness detection) on every `cpac install` and `cpac update`
- Auto-sync when data is stale (>24 hours)
- Delta sync for lightweight incremental updates
- Local cache at `~/.cpac/trust-db/` for offline use

**Files**: `src/trust_db.rs`

### Advisory Warnings

When installing or updating packages, CPAC checks the advisory database and displays color-coded warnings:
- **Critical** (red): Known malicious packages — blocks install with DANGER verdict
- **High** (red): Confirmed compromise — blocks install with WARNING verdict
- **Medium** (yellow): Suspicious activity — shows CAUTION
- **Low** (blue): Minor concerns — informational only
- **Suspected** (yellow): Under investigation — shows WARNING

Advisory signal contributes -30 to -5 penalty to trust score depending on severity.

**Files**: `src/trust/mod.rs`, `src/install.rs`, `src/update.rs`

### PKGBUILD Sanitization (Pass 1 + Pass 2)

Before any snapshot is submitted, CPAC runs two sanitization passes:

**Pass 1 — Structural Redaction**: Removes sensitive data while preserving diff structure:
- URLs (replaced with `[URL:REDACTED]`)
- Maintainer info (replaced with `[MAINTAINER:REDACTED]`)
- Comments (removed)
- Local file references (replaced with `[LOCAL_FILE:REDACTED]`)

**Pass 2 — Anomaly Detection**: Identifies 8 categories of suspicious patterns:
- Remote script execution (`curl | bash`, `wget | sh`, `fetch | sh`)
- Obfuscated content (hex escapes, base64, unicode tricks, concatenation)
- `eval` and `exec` usage
- Aggressive file removal (`rm -rf /`, `rm -rf ~`)
- Dynamic `pkgver` (non-deterministic builds)
- Package manager install inside build (`pacman -S`, `apt install`)
- System path modifications (`export PATH=`, modifying `/etc/`)
- Suspicious npm/bun install patterns

**Files**: `src/sanitize.rs`

### SHA-256 Hashing

CPAC computes SHA-256 hashes of sanitized PKGBUILDs for fast consensus checking without transmitting full content.

**Files**: `src/sanitize.rs`

### Pre-flight Intelligence Check

The new `compare` module provides `preflight_check()` — a single call that returns everything CPAC needs before install:
- Verdict: Clean, AdvisoryHit, Divergent, Outdated, Unknown
- Advisory status with severity and message
- Hash match/divergence status
- Outdatedness check against local cache
- Anomaly detection results (if full consent)

**Files**: `src/compare.rs`

### Snapshot Submission Pipeline

CPAC now submits PKGBUILD snapshots to the trust DB:
- Queue locally in `~/.cpac/trust-db/pending_snapshots.json`
- Flush queue on `cpac update` (never blocks install)
- Consent-aware: Hash-only (consent=hash) or full sanitized PKGBUILD (consent=full)
- `should_submit` flag prevents redundant submissions

**Files**: `src/trust_db.rs`, `src/install.rs`, `src/update.rs`

### Anonymous Client Tokens

Each CPAC installation gets a UUID-based anonymous token stored in `~/.cpac/trust-db/token`. Used for rate limiting only — no authentication or identification.

**Files**: `src/trust_db.rs`

### Transparent Install Script

`install.sh` builds CPAC from source via `cargo install`:
- Auto-detects if Rust is already installed
- Installs temporary Rust toolchain if needed
- Builds and installs to `/usr/local/bin`
- Cleans up temporary toolchain on exit (trap-based)
- Handles both success and failure paths

```bash
curl -sSf https://thecinderproject.qd.je/cpac/install.sh | bash
```

**Files**: `install.sh`

### GitHub Actions Release Workflow

Automated binary builds on tag push:
- x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu
- SHA-256 checksums
- GitHub Release with assets
- Uses rustls-tls (no OpenSSL dependency)

**Files**: `.github/workflows/release.yml`

## Bug Fixes

### aarch64 Cross-Compilation

Switched reqwest from native-tls to rustls-tls, eliminating the OpenSSL cross-compilation dependency that caused aarch64 builds to fail.

**Files**: `Cargo.toml`, `Cargo.lock`

## Changes Since v0.5.0

### v0.6.0 (unreleased as tag)

See v0.6.0 section below for config subcommands, auto cache, AUR failure handling, and first-run consent prompt.

### v0.7.0

- Trust DB integration via API proxy
- Advisory warnings with color-coded severity
- PKGBUILD sanitization (2 passes, 8 anomaly categories)
- SHA-256 hashing for fast consensus
- Pre-flight intelligence check with verdicts
- Snapshot submission pipeline with local queue
- Anonymous client tokens
- Delta sync for incremental updates
- Auto-sync during install/update
- Transparent build-from-source installer
- GitHub Actions release workflow (x86_64 + aarch64)
- Switched to rustls-tls (no OpenSSL dependency)

---

# CPAC v0.6.0 Release Notes

## Overview

v0.6 focuses on stability, usability, and removing friction. Key fixes prevent search failures and stale results, AUR is now enabled by default, and the config system was rebuilt with proper subcommands. Auto cache clearing and a first-run consent prompt round out the release.

## Bug Fixes

### AUR failure no longer kills entire search

If the AUR RPC returned an error (timeout, DNS failure, rate limit), the entire search failed — no pacman results were shown. Now AUR failures are caught gracefully with a warning, and official repo results are still returned.

### Cache TTL prevents stale results

Search results were cached forever. Now:
- Search cache expires after 1 hour
- Info cache expires after 24 hours
- Old entries gracefully fall through to live search

### AUR enabled by default

New users no longer get zero AUR results silently. AUR defaults to `on`.

## New Features

### Auto cache clearing

```bash
cpac config set cache daily    # clear daily
cpac config set cache weekly   # clear weekly
cpac config set cache monthly  # clear monthly (default)
```

Runs silently on every invocation. Manual `cpac clear-cache` still available.

### First-run consent prompt

On first launch in interactive terminals, CPAC asks about crowdsourced data sharing:
- `[1]` No submission
- `[2]` Hash/signature only (default)
- `[3]` Full PKGBUILD

Change anytime with `cpac config set consent`.

### Redesigned config command

```bash
cpac config show               # display all settings
cpac config set aur on|off     # toggle AUR
cpac config set consent ...    # set consent level
cpac config set cache ...      # set cache interval
cpac config reset              # reset to defaults
cpac config path               # show config file path
```

All non-interactive, scriptable, self-documenting via `--help`.

## Verification

```bash
$ cpac config show
Current configuration:
  AUR support:           on
  Crowdsourced data:     Hash/signature only
  Auto-clear cache:      monthly
  Config file:           /home/user/.cpac/config.toml

$ cpac config set cache weekly
Auto-clear cache interval set to: weekly
```

## Checks

- `cargo clippy` — zero warnings
- `cargo build --release` — clean
