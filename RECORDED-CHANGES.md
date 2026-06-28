# CPAC v0.8.1 — Source-Based Self-Update System

## Overview

Version 0.8.1 adds a self-update system that builds from source. When a newer version is available on GitHub, CPAC clones the repo at the target tag, builds a release binary, and replaces the running binary — preserving all user config in `~/.cpac/`.

---

## Changes

### Source-Based Self-Update

New `cpac upgrade` command builds CPAC from source and replaces the binary:

- **Version check on every run** — non-blocking, cached 24 hours in `~/.cpac/config.toml`
- **`cpac upgrade`** — clones repo at latest tag, `cargo build --release`, replaces binary
- **`--no-check-updates`** — global flag to skip version check on any command
- **Config preserved** — user config in `~/.cpac/` is never touched during upgrade
- **Sudo handling** — detects if install dir is writable, uses `sudo` for `/usr/local/bin`
- **Safe replacement** — renames current binary to `.old`, copies new, deletes `.old`
- **Prerequisite checks** — verifies `git` and `cargo` are available before attempting upgrade
- **Build cleanup** — temp build directory removed after install (success or failure)
- **Colored notice** — after every command, if newer version exists, shows upgrade notice

### Flow

1. Fetch latest release tag from GitHub API (`/repos/SabeeirSharrma/cpac/releases`)
2. Compare semver versions (strip `v` prefix, compare major.minor.patch)
3. If newer: `git clone --depth 1 --branch <tag>` into temp dir
4. `cargo build --release`
5. Replace current binary (with `sudo` if needed)
6. Clean up temp dir
7. Verify new version with `cpac --version`

### Config Safety

User configuration is stored in `~/.cpac/` and is completely separate from the binary:
- `~/.cpac/config.toml` — settings (aur, consent, cache interval, update check cache)
- `~/.cpac/trust-db/` — trust database cache
- `~/.cpac/cache/` — search/info cache

None of these are modified during an upgrade.

**Files**: `src/upgrade.rs` (new), `src/cli/mod.rs`, `src/config/mod.rs`, `src/main.rs`

### Panel Redesign — Unified Review Workflow

All three panels (volunteer, maintainer, admin) share a new "Review" tab replacing the old "Comparer" tab:

- **Package list** auto-fetched on tab load (packages needing advisories: no advisory or outdated)
- **Automated LCS diff** runs on package select
- **AI analysis** on-demand via NVIDIA NIM (3-hour cache)
- **Layout toggle** — Tabs or Side-by-Side, persisted to `localStorage`
- **Notes system** — floating button, textarea overlay, auto-saves to `localStorage`, cleared on publish
- **"Recompare"** button for manual re-run

### NVIDIA NIM Integration

AI analysis connected to NVIDIA NIM free tier (no credit card required):

- **Reasoning model** (`nvidia/nemotron-3-super-120b-a12b`) for PKGBUILD diff security analysis
- **Nano model** (`nvidia/nemotron-3-nano-30b-a3b`) for weekly report insights
- Worker endpoints: `POST /ai/analyze-diff`, `POST /ai/generate-report`
- Structured JSON response: recommendation, analysis, summary, severity, affected/safe versions, references
- Server-side API key (NVIDIA key never exposed to browser)

### Automated Weekly Email Reports

- Resend integration for transactional emails
- Reports generated daily, sent exactly 7 days after previous report per user (DOW matching)
- Staggered schedule based on account creation date (spreads across week, stays under 100/day quota)
- Zero activity = no email that week
- Reports sent as HTML table in email body, stored→sent→deleted (ephemeral)

### Cloudflare Cron Trigger

- Daily cron at midnight UTC (`0 0 * * *`)
- Worker `scheduled()` handler calls `/reports/generate` then `/reports/send`
- Worker config migrated from `wrangler.toml` to `wrangler.jsonc` (preferred format)

### Account Management

- Admin panel can create volunteer/maintainer accounts (random password, emailed via Resend)
- Account creation via Worker endpoint (`POST /accounts/create`)
- No public signups — accounts created by admin only

### RLS Recursion Fix

- `SECURITY DEFINER` helper functions (`is_admin()`, `is_maintainer()`, `is_volunteer()`) prevent recursive RLS on profiles table
- All panel auth uses `currentSession.access_token` as Bearer token

### Panel Data — Direct Supabase

- Panels call Supabase REST API directly for snapshots, advisories, ai_analysis, pending_advisories, RPC calls
- Worker direct URL (`https://cpac-trust-db-api.sabplay-idk.workers.dev`) for AUR proxy and account creation

---

## Files Changed (v0.8.0)

- `worker/wrangler.jsonc` — new config file (replaces wrangler.toml), adds `NVIDIA_API_KEY`, cron trigger
- `worker/src/index.ts` — `scheduled()` handler, `callNvidiaNim()`, `/ai/analyze-diff`, `/ai/generate-report`, `/accounts/create`, `/reports/generate`, `/reports/send`
- `worker/src/resend.ts` — Resend SDK, email templates, weekly report HTML builder
- `supabase/migrations/20260629000005_fix_rls_recursion.sql` — SECURITY DEFINER helpers

---

# CPAC v0.7.0 — Trust DB Integration & PKGBUILD Sanitization

## Overview

Version 0.7.0 integrates the cpac-trust-db into CPAC via a Cloudflare Worker proxy at `api.thecinderproject.qd.je`. CPAC performs two-pass PKGBUILD sanitization before submission, detects anomalies, submits snapshots with consent-aware privacy controls, and ships a transparent build-from-source installer. The release workflow builds x86_64 and aarch64 binaries with rustls-tls (no OpenSSL dependency).

---

## Changes

### Trust DB Integration via API Proxy

CPAC communicates with the trust DB backend through a Cloudflare Worker proxy at `api.thecinderproject.qd.je/cpac-trust-db/api/*`. The worker forwards requests to Supabase, handling CORS and auth headers. The `trust_db` module handles:
- Meta check (staleness detection) on every `cpac install` and `cpac update`
- Auto-sync when data is stale (>24 hours)
- Delta sync for lightweight incremental updates (`updated_at > last_sync`)
- Local cache at `~/.cpac/trust-db/` for offline use

**Files**: `src/trust_db.rs`

### Advisory Warnings

When installing or updating packages, CPAC checks the advisory database and displays color-coded warnings:
- **Critical** (red): Known malicious — DANGER verdict, blocks install
- **High** (red): Confirmed compromise — WARNING verdict, blocks install
- **Medium** (yellow): Suspicious activity — CAUTION
- **Low** (blue): Minor concerns — informational
- **Suspected** (yellow): Under investigation — WARNING

Advisory signal contributes -30 to -5 penalty to trust score.

**Files**: `src/trust/mod.rs`, `src/install.rs`, `src/update.rs`

### PKGBUILD Sanitization (Pass 1 + Pass 2)

Before any snapshot submission, CPAC runs two sanitization passes:

**Pass 1 — Structural Redaction**:
- URLs → `[URL:REDACTED]`
- Maintainer info → `[MAINTAINER:REDACTED]`
- Comments → removed
- Local file references → `[LOCAL_FILE:REDACTED]`

**Pass 2 — Anomaly Detection** (8 categories):
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

Computes SHA-256 hashes of sanitized PKGBUILDs for fast consensus checking without transmitting full content.

**Files**: `src/sanitize.rs`

### Pre-flight Intelligence Check

`compare::preflight_check()` is a single call returning everything needed before install:
- Verdict: Clean, AdvisoryHit, Divergent, Outdated, Unknown
- Advisory status with severity and message
- Hash match/divergence status
- Outdatedness check against local cache
- Anomaly detection results (if full consent)

**Files**: `src/compare.rs`

### Snapshot Submission Pipeline

- Queue locally in `~/.cpac/trust-db/pending_snapshots.json`
- Flush queue on `cpac update` (never blocks install)
- Consent-aware: Hash-only (consent=hash) or full sanitized PKGBUILD (consent=full)
- `should_submit` flag prevents redundant submissions

**Files**: `src/trust_db.rs`, `src/install.rs`, `src/update.rs`

### Anonymous Client Tokens

UUID-based anonymous token stored in `~/.cpac/trust-db/token`. Used for rate limiting only.

**Files**: `src/trust_db.rs`

### Transparent Install Script

`install.sh` builds from source via `cargo install`:
- Auto-detects Rust, installs temp toolchain if needed
- Builds and installs to `/usr/local/bin`
- Cleans up temp toolchain on exit (trap-based)

```bash
curl -sSf https://thecinderproject.qd.je/cpac/install.sh | bash
```

**Files**: `install.sh`

### GitHub Actions Release Workflow

Automated binary builds on tag push:
- x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu
- SHA-256 checksums, GitHub Release with assets
- Uses rustls-tls (no OpenSSL dependency)

**Files**: `.github/workflows/release.yml`

### Bug Fixes

#### aarch64 Cross-Compilation

Switched reqwest from native-tls to rustls-tls, eliminating the OpenSSL cross-compilation dependency.

**Files**: `Cargo.toml`, `Cargo.lock`

---

## Files Changed (v0.7.0)

- `Cargo.toml` — version bump to 0.7.0, reqwest rustls-tls, new deps (regex, hostname, sha2, uuid)
- `Cargo.lock` — updated dependency tree
- `src/trust_db.rs` — API proxy client (api.thecinderproject.qd.je), meta check, delta sync, snapshot submission, pending queue, anonymous tokens
- `src/compare.rs` — pre-flight intelligence check with verdicts
- `src/sanitize.rs` — Pass 1 structural redaction + Pass 2 anomaly detection (8 categories, 6 tests)
- `src/trust/mod.rs` — advisory signal in trust scoring
- `src/install.rs` — auto-sync, consent-aware queuing, preflight + anomaly display
- `src/update.rs` — delta sync, flush queue, advisory warnings
- `src/config/mod.rs` — ConsentLevel enum
- `install.sh` — transparent build-from-source installer
- `.github/workflows/release.yml` — release workflow (x86_64 + aarch64)
- `docs/installation.md` — updated install docs
- `docs/configuration.md` — updated config docs
- `docs/trust-algorithm.md` — updated trust algorithm docs
- `docs/reference.md` — updated reference docs

---

# CPAC v0.6.0 — Stability, Config & Auto-Cache

## Overview

Version 0.6 focuses on hardening, usability, and removing friction. Key fixes: AUR failures no longer kill searches, cache has TTL to prevent stale results, AUR is enabled by default for new users. The config system was redesigned with proper subcommands, auto cache clearing was added, and the first-run consent prompt ensures users set privacy preferences before using the tool.

---

## Changes

### Bug Fixes

#### AUR failure no longer kills entire search

Previously, if the AUR RPC returned an error (network timeout, DNS failure, rate limiting), the `?` operator propagated the error and the **entire search failed** — no pacman results were shown. Now AUR failures are caught gracefully: a warning is printed and official repo results are still returned.

**Files**: `src/resolver/mod.rs`

#### Cache now has TTL (no more stale results forever)

Search results were previously cached indefinitely. If packages were added/removed from repos since the last search, users would see stale results until manually running `cpac clear-cache`. Now all cache entries have timestamps:
- Search cache expires after **1 hour**
- Info cache expires after **24 hours**
- Expired entries trigger a fresh live search automatically
- Old cache entries (without timestamps) gracefully fall through to live search

**Files**: `src/resolver/mod.rs`

#### AUR now enabled by default

`Config::default()` previously set `aur_enabled: false`, so new users with no config file got zero AUR results silently with no warning. Now AUR defaults to `true` for new installations.

**Files**: `src/config/mod.rs`

### New Features

#### Auto cache clearing

CPAC automatically clears its metadata cache based on a configurable interval:
- **Daily** (`cpac config set cache daily`)
- **Weekly** (`cpac config set cache weekly`)
- **Monthly** (`cpac config set cache monthly`) — default

Auto-clearing runs silently on every CPAC invocation. The interval and last clear timestamp are stored in `~/.cpac/config.toml`.

**Files**: `src/config/mod.rs`, `src/cli/mod.rs`

#### First-run consent prompt

On first launch, CPAC shows an interactive consent prompt for crowdsourced data sharing. This only appears once, only in interactive terminals. Users can change their preference anytime via `cpac config set consent`.

The prompt asks:
- `[1]` No, don't submit anything
- `[2]` Yes, hash/signature only (default)
- `[3]` Yes, full PKGBUILD

**Files**: `src/cli/mod.rs`, `src/config/mod.rs`

#### Redesigned config command

The config command was rebuilt with proper clap subcommands for scriptability and self-documentation:

```bash
cpac config show               # display all current settings
cpac config set aur on|off     # enable/disable AUR search
cpac config set consent ...    # set crowdsourced data sharing level
cpac config set cache ...      # set auto-clear interval
cpac config reset              # reset all settings to defaults
cpac config path               # show config file location
```

All settings are non-interactive and work in scripts. The old `cpac aur enable/disable` command is removed — replaced by `cpac config set aur`.

**Files**: `src/cli/mod.rs`, `src/config/mod.rs`

### Warning Cleanup

- `src/cache/mod.rs` — Added `#[allow(dead_code)]` for `advisories` field and methods (reserved for future advisory integration per spec)
- `src/config/mod.rs` — Replaced manual `Default` impl for `CacheInterval` with `#[derive(Default)]`

---

## Verification

### Config command

```bash
$ cpac config show
Current configuration:
  AUR support:           on
  Crowdsourced data:     Hash/signature only
  Auto-clear cache:      monthly
  Config file:           /home/user/.cpac/config.toml

$ cpac config set consent full
Crowdsourced data set to: Full PKGBUILD

$ cpac config set cache weekly
Auto-clear cache interval set to: weekly

$ cpac config set aur off
AUR support disabled.

$ cpac config reset
Configuration reset to defaults.
```

### All Checks Pass

- `cargo clippy` — clean (zero warnings)
- `cargo build --release` — clean (zero warnings)

---

## Files Changed (v0.6)

- `Cargo.toml` — version bump to 0.6.0
- `src/resolver/mod.rs` — AUR failure graceful fallback; cache TTL with `CachedEntry<T>` wrapper
- `src/config/mod.rs` — `CacheInterval` enum, `Config` struct with TTL/first-run fields, `ValueEnum` derives, `path()` helper
- `src/cache/mod.rs` — annotated `advisories` as reserved for future use
- `src/cli/mod.rs` — redesigned `config` command with clap subcommands; first-run consent prompt; auto cache clearing; removed standalone `Aur` command
