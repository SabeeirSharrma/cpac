# CPAC v1.0.0 Release Notes

## Overview

Offline-first release. CPAC v1 ships fully functional without any network dependency. Trust scoring runs entirely on local signals — repository source, package age, maintainer history, popularity, update recency, PKGBUILD diffing, and anomaly detection. The cpac-trust-db backend is feature-flagged off (`#[cfg(feature = "trust-db")]`) and can be re-enabled with a single flag flip in a future release.

## What Changed

- **Offline-first architecture** — all trust scoring runs locally, no network calls on install/trust/audit
- **Trust-db feature-flagged off** — `trust_db` and `compare` modules gated behind `#[cfg(feature = "trust-db")]`
- **Consent system removed** — no crowdsourced data sharing in v1, `ConsentLevel` gated behind feature
- **First-run prompt removed** — no consent to ask for, simpler first-run experience
- **"Local signals only" note** — trust report shows community data unavailable notice when feature off
- **Version bumped to 1.0.0** — codename "Cinder"

## What Stays

- Repository source scoring (Official +30, ThirdParty +10, AUR +5, Unknown -5)
- Unknown metadata penalties (age, maintainer, popularity, recency)
- Orphaned package penalty (-10)
- Source-aware outdated penalty (AUR/third-party only)
- PKGBUILD diffing on upgrade (local, no network)
- Pass 2 anomaly detection (curl|sh, base64, eval/exec, rm -rf, npm/bun pipe-to-shell)
- Self-updater (`cpac upgrade`)
- Auto cache clearing

## What's Removed for v1

- Meta check (`GET /api/meta`)
- Delta sync
- Snapshot submission pipeline
- Advisory lookups from trust-db
- Consent prompt and consent config
- Crowdsourced PKGBUILD diffing

## Re-enabling Trust-DB Later

```bash
cargo build --release --features trust-db
```

Or in Cargo.toml:
```toml
[features]
default = ["trust-db"]
```

---

# CPAC v0.9.4 Release Notes

_(see previous entry)_

# CPAC v0.9.2 Release Notes

_(see previous entry)_

# CPAC v0.9.1 Release Notes

_(see previous entry)_

# CPAC v0.9.0 Release Notes

_(see previous entry)_
