---
title: Configuration
description: Configuring CPAC.
order: 3
---

# Configuration

CPAC stores its configuration at `~/.cpac/config.toml` using standard TOML format. The file is created on first save.

## Using the Config Command

The easiest way to view and change settings is:

```bash
cpac config
```

This displays the current configuration and presents an interactive menu for changing the crowdsourced data submission level.

## Configuration File

The raw configuration file looks like this:

```toml
aur_enabled = false
consent = "hash"
```

### Options

| Key | Type | Default | Values |
| --- | --- | --- | --- |
| `aur_enabled` | boolean | `false` | `true` / `false` |
| `consent` | string | `"hash"` | `"none"` / `"hash"` / `"full"` |

### `aur_enabled`

Controls whether CPAC includes the [Arch User Repository (AUR)](https://aur.archlinux.org/) in search results, trust lookups, and install operations.

- **`false`** (default) -- AUR is excluded. `cpac search` only queries official repos. AUR packages cannot be installed via CPAC.
- **`true`** -- AUR is included. Search, trust, and install operations will query both official repos and the AUR.

Toggle via CLI:

```bash
cpac aur enable    # enable AUR support
cpac aur disable   # disable AUR support
```

### `consent`

Controls the level of data CPAC submits to the community trust database for crowdsourced PKGBUILD diffing. This is an opt-in system -- see [Trust Algorithm](trust-algorithm.md) for details.

| Value | Label | Description |
| --- | --- | --- |
| `"none"` | No submission | Don't submit anything to the community database |
| `"hash"` | Hash/signature only | Submit only hash/signature data (default) |
| `"full"` | Full PKGBUILD | Submit the full PKGBUILD text for better diff accuracy |

Change via the interactive config menu:

```bash
cpac config
```

## Cache Location

CPAC stores cached data at `~/.cpac/cache/`:

| File | Contents |
| --- | --- |
| `packages.db` | Package search results and info lookups |
| `trust.db` | Trust analysis reports |
| `advisories.db` | Security advisories (reserved for future use) |
| `pkgbuilds.db` | Cached PKGBUILD snapshots for upgrade diffing |

Clear the cache manually:

```bash
cpac clear-cache
```
