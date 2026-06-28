# CPAC v0.9.2 — Patch: Self-Updater + Official PKGBUILD Fetching

## Overview

Patch release adding automatic temporary Rust toolchain installation to the self-updater, and fetching PKGBUILDs for official Arch packages from Arch GitLab for trust DB submission.

---

## Changes

### Self-Updater — Temporary Rust Toolchain

`cpac upgrade` now installs a temporary Rust toolchain if `cargo` is not found:

- **Auto-detect**: checks for `cargo` before building
- **Install**: uses `pacman -S rustup` on Arch, falls back to `rustup.rs` installer
- **Cleanup**: removes temporary Rust toolchain after build (via Drop guard)
- **Error-safe**: cleanup happens on success, failure, or cancellation

### Official Package PKGBUILD Fetching

`fetch_pkgbuild_for_package()` now fetches PKGBUILDs for official Arch packages from `gitlab.archlinux.org`:

- **URL pattern**: `https://gitlab.archlinux.org/archlinux/packaging/packages/<pkg>/-/raw/main/PKGBUILD`
- **10-second timeout** per request
- **Graceful fallback**: returns `None` if fetch fails (no error shown)
- **Enables trust DB submission** for official packages (previously skipped)

### Progress Messages

- `Sanitizing PKGBUILD and queuing snapshot...` shown when PKGBUILD is being prepared
- `Snapshot queued for submission on next 'cpac update'.` shown after queueing

**Files**: `src/upgrade.rs`, `src/resolver/mod.rs`, `src/install.rs`

---

# CPAC v0.9.1 — Patch: Direct Worker URL + Brand Fix

_(see previous entry)_

# CPAC v0.9.0 — Trust Scoring Overhaul & Polish

_(see previous entry)_
