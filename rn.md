# CPAC v0.9.4 Release Notes

## Overview

Hardened trust scoring release. Unknown metadata now penalizes, AUR scored more conservatively, and submission deduplication prevents duplicates.

## Changes

- **Stricter source scoring** — AUR: +5 (was +10), ThirdParty: +10 (was +15), Unknown: -5 (was 0)
- **Unknown metadata penalties** — age: -2, maintainer: -3, popularity: -2, recency: -2
- **Orphaned penalty** — -10 (was -5)
- **Source-aware outdated** — penalty only for AUR/third-party, not official packages
- **Submission dedup** — skip if version in DB, hash matches latest, or hash well-known
- **Official PKGBUILDs** — fetched from Arch GitLab for trust DB submission
- **Self-updater** — installs temporary Rust if not present, cleans up after

---

# CPAC v0.9.2 Release Notes

_(see previous entry)_

# CPAC v0.9.1 Release Notes

_(see previous entry)_

# CPAC v0.9.0 Release Notes

_(see previous entry)_
