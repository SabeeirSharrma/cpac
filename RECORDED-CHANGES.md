# CPAC v0.9.4 — Hardened Trust Scoring

## Overview

Major scoring overhaul making the trust system stricter and more security-focused. Unknown metadata now penalizes scores, AUR packages scored more conservatively, outdated penalty is source-aware, and submission logic prevents duplicates.

---

## Changes

### Stricter Source Scoring

| Source | Before | After |
|--------|--------|-------|
| Official | +30 | +30 (unchanged) |
| ThirdParty | +15 | +10 |
| AUR | +10 | +5 |
| Unknown | 0 | -5 |

AUR packages are community-maintained and carry more risk. Unknown sources are actively penalized.

### Unknown Metadata Now Penalizes

Previously, missing metadata scored 0 (neutral). Now it penalizes:

| Signal | Penalty | Detail |
|--------|---------|--------|
| Age unknown | -2 | Unknown provenance |
| Maintainer unknown | -3 | Unknown custodian |
| Popularity unknown | -2 | Unknown adoption |
| Last Updated unknown | -2 | Unknown maintenance status |

### Orphaned Package Penalty Strengthened

-5 → **-10**. An orphaned package has no active maintainer to respond to security issues.

### Outdated Penalty is Source-Aware

The -5 outdated penalty now only applies to AUR/third-party packages. Official packages are not penalized for having newer community versions in the trust DB (they use upstream versioning).

### Submission Deduplication

Snapshot submission now checks three conditions before queueing:
1. Version already in DB → skip
2. PKGBUILD hash matches latest → skip
3. Hash already well-known (10+ submissions) → skip

### Official Package PKGBUILD Fetching

CPAC now fetches PKGBUILDs for official packages from `gitlab.archlinux.org`, enabling trust DB submission for all packages.

### Self-Updater Temp Rust

`cpac upgrade` installs temporary Rust if not present, cleans up after build.

---

## Files Changed (v0.9.4)

- `src/trust/mod.rs` — stricter source scores, unknown metadata penalties, orphaned -10
- `src/compare.rs` — source-aware outdated penalty, version dedup check, source param
- `src/install.rs` — pass source to preflight_check, decouple snapshot from PKGBUILD fetch
- `src/resolver/mod.rs` — official PKGBUILD fetching from Arch GitLab
- `src/upgrade.rs` — temporary Rust toolchain, Drop guard cleanup
- `Cargo.toml` — version bump to 0.9.4

---

# CPAC v0.9.2 — Patch: Self-Updater + Official PKGBUILD Fetching

_(see previous entry)_

# CPAC v0.9.1 — Patch: Direct Worker URL + Brand Fix

_(see previous entry)_

# CPAC v0.9.0 — Trust Scoring Overhaul & Polish

_(see previous entry)_
