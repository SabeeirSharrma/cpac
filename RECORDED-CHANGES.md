# CPAC v0.9.2 — Patch: Self-Updater Installs Temporary Rust

## Overview

Patch release adding automatic temporary Rust toolchain installation to the self-updater. Users without Rust installed can now run `cpac upgrade` without manual setup.

---

## Changes

### Self-Updater — Temporary Rust Toolchain

`cpac upgrade` now installs a temporary Rust toolchain if `cargo` is not found:

- **Auto-detect**: checks for `cargo` before building
- **Install**: uses `pacman -S rustup` on Arch, falls back to `rustup.rs` installer
- **Cleanup**: removes temporary Rust toolchain after build (via Drop guard)
- **Error-safe**: cleanup happens on success, failure, or cancellation

Previously, users without Rust saw: `cargo is required for upgrades. Please install Rust and try again.`

Now it just works.

**Files**: `src/upgrade.rs`

---

# CPAC v0.9.1 — Patch: Direct Worker URL + Brand Fix

_(see previous entry)_

# CPAC v0.9.0 — Trust Scoring Overhaul & Polish

_(see previous entry)_
