# CPAC v0.9.0 — Trust Scoring Overhaul & Polish

## Overview

Version 0.9.0 addresses 31 check failures across trust scoring, advisory handling, CLI UX, and error messages. Advisory statuses now enforce floor recommendations, missing metadata scores 0 (no partial credit), PKGBUILD anomalies penalize install-time trust, legacy commands show migration hints, and `--no-color` support is added.

---

## Changes

### Trust Scoring Fixes (31 check failures addressed)

#### Group A — Missing Metadata = 0 Points

Previously, packages with unavailable metadata (popularity, last updated, package age) received partial credit based on source. Now they score **0** for those fields. This prevents a package from appearing "safe" just because it's from an official repo while having no metadata.

**Impact**: `firefox` and similar packages with missing metadata now show lower trust scores. Recommendation floors at "Moderate" when only metadata is missing.

**Files**: `src/trust/mod.rs`

#### Group B — Advisory Floor Enforcement

New `advisory_floor()` function ensures advisory statuses raise the recommendation floor. Called in `analyze()` — only raises, never lowers.

```rust
"safe"        → no floor (neutral)
"suspicious"  → floor at "Warning"
"warning"     → floor at "Warning"  
"malicious"   → floor at "Danger"
"resolved"    → no floor (neutral)
```

**Files**: `src/trust/mod.rs`

#### Group C — PKGBUILD Anomaly Penalty

PKGBUILD anomalies (curl-pipe-sh, obfuscated content, eval usage, etc.) now lower the install-time trust score via `anomaly_penalty()`. Each anomaly category has a configurable point deduction.

**Files**: `src/install.rs`, `src/trust/mod.rs`

#### Group D — `cpac update --aur` Forces AUR Update

When AUR is disabled, `cpac update --aur` now forces a one-time AUR sync and prints a note. Previously it silently skipped AUR.

**Files**: `src/update.rs`

#### Group E — Cache Write Failure Warnings

Cache write failures now print a non-blocking warning instead of being silently swallowed. Users see `Warning: Cache write failed (search): <reason>` but the operation continues.

**Files**: `src/resolver/mod.rs`, `src/trust/mod.rs`

#### Group F — Meta Check on Trust/Audit Commands

`cpac trust` and `cpac audit` now trigger a meta check for staleness detection. If the trust DB cache is >24 hours old, users are prompted to update.

**Files**: `src/cli/mod.rs`

#### Group G — Offline Stale Cache Prompt

When the trust DB cache is stale and the server is unreachable, the CLI shows an interactive prompt: `Would you like to attempt an update? [y/N]`. Non-interactive contexts (piped input) show `Run 'cpac update' when online to refresh.`

**Files**: `src/trust_db.rs`

#### Group H — Unknown Package Handling

Unknown packages (not in repos or trust DB) now show:
```
Trust score based on local signals only.
We do not have data on this package in the CPAC Trust DB.
```

Contribution prompt only shown when consent is `None` or `Hash` (Full already auto-queues). Default is `y` for opt-in consent levels.

**Files**: `src/install.rs`

#### Group I — Self-Updater Improvements

- Atomic binary replacement via `rename()` (Linux ext4/xfs atomic)
- Concurrent process detection: warns if another cpac instance is running
- Better error messages with actionable suggestions
- Build cleanup on failure

**Files**: `src/upgrade.rs`

#### Group J — First-Run Token Generation

Anonymous client token is now generated on first run if missing, instead of failing silently.

**Files**: `src/trust_db.rs`

#### Group K — Version Codename "Sentinel"

Version output now includes codename: `cpac 0.9.0 (Sentinel) — A package trust layer for Arch-based Linux`

**Files**: `src/cli/mod.rs`

### Legacy Command Migration

Old subcommands now show helpful migration messages instead of generic clap errors:

| Legacy Command | Message |
|---|---|
| `cpac aur enable` | `The 'aur' subcommand has been moved. Use: cpac config set aur on instead.` |
| `cpac aur disable` | `The 'aur' subcommand has been moved. Use: cpac config set aur off instead.` |
| `cpac aur <other>` | `The 'aur' subcommand has been removed. Use 'cpac config set aur on|off' to configure AUR.` |
| `cpac update --self` | `The 'update' command has been renamed. Use: cpac upgrade instead.` |

Legacy check runs before clap parsing so it catches the args first.

**Files**: `src/cli/mod.rs`

### `--no-color` Flag & `NO_COLOR` Env Var

- `--no-color` global flag disables colored output on any command
- `NO_COLOR` env var also respected (standard: https://no-color.org/)
- Uses `colored::control::set_override(false)` to disable all coloring
- `--no-color` is marked `global = true` so it works with subcommands

**Files**: `src/cli/mod.rs`, `src/display/mod.rs`

### Actionable Error Messages

All error messages now include actionable suggestions:

| Error | Suggestion Added |
|---|---|
| Package not found (diff/remove) | `Try 'cpac search <name>'` |
| cargo build failed | `Check Rust toolchain: rustup update` |
| Binary not found after build | `Check Cargo.toml for correct binary name` |
| sudo cp failed | `Check that you have sudo access` |
| AUR client error | `Try 'cpac search <name>' to verify the name` |
| paru/yay sync failed | `Check your network connection and try again` |
| No AUR helper found | Installation URL for paru |

**Files**: `src/diff.rs`, `src/remove.rs`, `src/upgrade.rs`, `src/backends/aur.rs`, `src/update.rs`

### Trust DB Fallback

`cpac trust` now falls back to the trust DB when a package is not found in any synced repo (e.g. unsynced CachyOS repos). Shows the trust report with a note about the package not being in synced repos.

**Files**: `src/cli/mod.rs`

### Audit Output Truncation

`cpac audit` now truncates output to 50 warnings with a `(showing 50 of 127)` message, preventing wall-of-text output on large audits.

**Files**: `src/display/mod.rs`

---

## Files Changed (v0.9.0)

- `src/trust/mod.rs` — `advisory_floor()`, `anomaly_penalty()`, missing metadata = 0
- `src/trust_db.rs` — stale cache interactive prompt, first-run token generation, staleness_days()
- `src/cli/mod.rs` — `--no-color` global flag, legacy command interception, trust DB fallback, version codename, audit truncation, meta check on trust/audit
- `src/install.rs` — anomaly penalty applied to score, unknown package handling with conditional prompt
- `src/update.rs` — `--aur` forces AUR update, actionable error messages
- `src/display/mod.rs` — audit truncation (50 warnings)
- `src/diff.rs` — actionable "not found" message
- `src/remove.rs` — actionable "not found" message
- `src/upgrade.rs` — atomic rename, concurrent process detection, actionable errors
- `src/prompt.rs` — `prompt_contribute_package()` for unknown package contribution
- `src/resolver/mod.rs` — cache write failure warnings
- `src/backends/aur.rs` — actionable AUR error messages
- `Cargo.toml` — version bump to 0.9.0

---

# CPAC v0.8.2 — Bidirectional Advisory Statuses

_(see previous entry)_
