# CPAC v1.0.0 — Offline-First Release

## Overview

CPAC v1 ships fully functional without any network dependency. The trust-db backend is feature-flagged off — re-enabled with a single flag flip. This is not a downgrade; local scoring is the actual foundation of CPAC's trust model.

---

## Changes

### Offline-First Architecture

All trust scoring runs entirely on local signals. No network calls on `cpac install`, `cpac trust`, or `cpac audit`. The tool works fully offline.

### Trust-DB Feature-Flagged Off

The `trust_db` and `compare` modules are gated behind `#[cfg(feature = "trust-db")]`. Building without the feature produces a smaller binary with zero network dependencies for trust scoring.

| Component | Status |
|-----------|--------|
| `src/trust_db.rs` | Gated behind feature |
| `src/compare.rs` | Gated behind feature |
| `src/install.rs` | Trust-db calls gated |
| `src/update.rs` | Trust-db sync/flush gated |
| `src/trust/mod.rs` | Advisory lookup gated |
| `src/cli/mod.rs` | Consent config removed, trust-db calls gated |
| `src/config/mod.rs` | `ConsentLevel` gated, consent functions gated |
| `src/display/mod.rs` | "Local signals only" note added |
| `src/prompt.rs` | `prompt_contribute_package` gated |
| `src/sanitize.rs` | `sha256_hash` gated |

### Consent System Removed

- `ConsentLevel` enum gated behind feature
- `set_consent`, `mark_first_run_done`, `is_first_run_done` gated
- `SetCommand::Consent` variant removed from CLI
- `first_run_prompt` function removed entirely
- Config struct `consent` field gated

### First-Run Experience Simplified

No consent prompt on first launch. The first-run flow is removed entirely — nothing to ask about in v1.

### "Local Signals Only" Note

Trust report output includes:
```
Note: This trust score is based on local signals only.
Community trust data (cpac-trust-db) is not yet available in this release.
```

### Version Bumped to 1.0.0

Codename: "Cinder" (reserved per VERSIONING.md for v1.0).

---

## What Stays (Unchanged from v0.9.4)

- Repository source scoring (Official +30, ThirdParty +10, AUR +5, Unknown -5)
- Unknown metadata penalties (age -2, maintainer -3, popularity -2, recency -2)
- Orphaned penalty (-10)
- Source-aware outdated penalty (AUR/third-party only, not official)
- PKGBUILD diffing on upgrade (local, LCS-based)
- Pass 2 anomaly detection (curl|sh, base64, eval/exec, rm -rf, npm/bun pipe-to-shell)
- Self-updater with temporary Rust toolchain
- Auto cache clearing (daily/weekly/monthly)

---

## Files Changed (v1.0.0)

- `Cargo.toml` — version 1.0.0, `[features]` section, optional uuid
- `src/main.rs` — gated `mod trust_db` and `mod compare`
- `src/cli/mod.rs` — removed first_run_prompt, consent config, gated trust-db calls
- `src/config/mod.rs` — gated ConsentLevel, consent functions, first-run functions
- `src/install.rs` — gated all trust-db calls (meta check, snapshot, consent, pre-flight)
- `src/update.rs` — gated trust-db sync, flush, advisory warnings
- `src/trust/mod.rs` — gated Signal 7 (advisory lookup), advisory floor
- `src/display/mod.rs` — "local signals only" note when feature off
- `src/prompt.rs` — gated prompt_contribute_package
- `src/sanitize.rs` — gated sha256_hash

---

## Re-enabling Trust-DB

In a future release, trust-db can be re-enabled:

```bash
# Build with trust-db
cargo build --release --features trust-db

# Or enable by default in Cargo.toml
[features]
default = ["trust-db"]
```

---

# CPAC v0.9.4 — Hardened Trust Scoring

_(see previous entry)_

# CPAC v0.9.2 — Patch: Self-Updater + Official PKGBUILD Fetching

_(see previous entry)_

# CPAC v0.9.1 — Patch: Direct Worker URL + Brand Fix

_(see previous entry)_

# CPAC v0.9.0 — Trust Scoring Overhaul & Polish

_(see previous entry)_
