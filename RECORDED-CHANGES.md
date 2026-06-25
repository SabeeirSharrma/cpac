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

### Search with AUR failure

```bash
# If AUR is down, search still returns official results:
$ cpac search firefox
Warning: AUR search failed (...). Showing official results only.
Package                          Version        Source           Description
firefox                          152.0.2-1      official/extra   Fast, Private & Safe Web Browser
...
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
