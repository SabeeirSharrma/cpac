# CPAC v0.6.0 Release Notes

## Overview

v0.6 focuses on stability, usability, and removing friction. Key fixes prevent search failures and stale results, AUR is now enabled by default, and the config system was rebuilt with proper subcommands. Auto cache clearing and a first-run consent prompt round out the release.

## Bug Fixes

### AUR failure no longer kills entire search

If the AUR RPC returned an error (timeout, DNS failure, rate limit), the entire search failed — no pacman results were shown. Now AUR failures are caught gracefully with a warning, and official repo results are still returned.

### Cache TTL prevents stale results

Search results were cached forever. Now:
- Search cache expires after 1 hour
- Info cache expires after 24 hours
- Old entries gracefully fall through to live search

### AUR enabled by default

New users no longer get zero AUR results silently. AUR defaults to `on`.

## New Features

### Auto cache clearing

```bash
cpac config set cache daily    # clear daily
cpac config set cache weekly   # clear weekly
cpac config set cache monthly  # clear monthly (default)
```

Runs silently on every invocation. Manual `cpac clear-cache` still available.

### First-run consent prompt

On first launch in interactive terminals, CPAC asks about crowdsourced data sharing:
- `[1]` No submission
- `[2]` Hash/signature only (default)
- `[3]` Full PKGBUILD

Change anytime with `cpac config set consent`.

### Redesigned config command

```bash
cpac config show               # display all settings
cpac config set aur on|off     # toggle AUR
cpac config set consent ...    # set consent level
cpac config set cache ...      # set cache interval
cpac config reset              # reset to defaults
cpac config path               # show config file path
```

All non-interactive, scriptable, self-documenting via `--help`.

## Verification

```bash
$ cpac config show
Current configuration:
  AUR support:           on
  Crowdsourced data:     Hash/signature only
  Auto-clear cache:      monthly
  Config file:           /home/user/.cpac/config.toml

$ cpac config set cache weekly
Auto-clear cache interval set to: weekly
```

## Checks

- `cargo clippy` — zero warnings
- `cargo build --release` — clean
