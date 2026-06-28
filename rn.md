# CPAC v0.9.2 Release Notes

## Overview

Patch release making `cpac upgrade` work without Rust, and enabling official package PKGBUILD submission to the trust DB.

## Changes

- **Self-updater installs temporary Rust** — if `cargo` is not found, `cpac upgrade` installs a temporary toolchain, builds, then removes it
- **Official package PKGBUILDs** — fetches from Arch GitLab (`gitlab.archlinux.org`), enables trust DB submission for official packages
- **Progress messages** — shows "Sanitizing PKGBUILD and queuing snapshot..." during install

---

# CPAC v0.9.1 Release Notes

_(see previous entry)_

# CPAC v0.9.0 Release Notes

_(see previous entry)_
