# CPAC v0.9.2 Release Notes

## Overview

Patch release making `cpac upgrade` work for users without Rust installed.

## Changes

- **Self-updater installs temporary Rust** — if `cargo` is not found, `cpac upgrade` installs a temporary toolchain via `pacman` or `rustup.rs`, builds, then removes it
- **Drop guard cleanup** — temporary Rust and build directory are cleaned up automatically on success or failure

---

# CPAC v0.9.1 Release Notes

_(see previous entry)_

# CPAC v0.9.0 Release Notes

_(see previous entry)_
