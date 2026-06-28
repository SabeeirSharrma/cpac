# CPAC v0.9.0 Release Notes

## Overview

v0.9.0 is a major polish release addressing 31 check failures across trust scoring, advisory handling, CLI UX, and error messages. Advisory statuses now enforce floor recommendations, missing metadata scores 0, PKGBUILD anomalies penalize trust, legacy commands show migration hints, and `--no-color` is supported.

## Changes

### Trust Scoring Overhaul

- **Missing metadata = 0 points** — no partial credit when popularity/age/last-updated are unavailable
- **Advisory floor enforcement** — `suspicious`/`warning` floor at "Warning", `malicious` floors at "Danger"
- **PKGBUILD anomaly penalty** — curl-pipe-sh, obfuscated content, eval, etc. lower install-time score
- **`advisory_floor()`** called in `analyze()` — only raises recommendation, never lowers

### CLI Improvements

- **`--no-color` global flag** — disables colored output on any command
- **`NO_COLOR` env var** — standard https://no-color.org/ support
- **Legacy command migration** — `cpac aur enable` → "Use: cpac config set aur on"
- **Version codename** — `cpac 0.9.0 (Sentinel)`

### Self-Updater Hardening

- **Atomic binary replacement** — `rename()` on Linux ext4/xfs (crash-safe)
- **Concurrent process detection** — warns if another cpac is running
- **Better error messages** — actionable suggestions on all failure paths

### Unknown Package Handling

- Clearer message: "Trust score based on local signals only. We do not have data on this package in the CPAC Trust DB."
- Contribution prompt only when consent is None or Hash (Full already auto-queues)

### Error Message Actionability

All error messages now include suggestions:
- Package not found → `Try 'cpac search <name>'`
- cargo build failed → `Check Rust toolchain: rustup update`
- No AUR helper → installation URL
- Network errors → `Check your network connection`

### Other Fixes

- Trust DB fallback for `cpac trust` when package not in repos
- Audit output truncated to 50 warnings (shows count)
- Cache write failure warnings (non-blocking)
- First-run token generation if missing

---

# CPAC v0.8.2 Release Notes

_(see previous entry)_
