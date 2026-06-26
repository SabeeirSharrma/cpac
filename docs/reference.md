---
title: Reference
description: CPAC command reference.
order: 4
---

# Command Reference

## `cpac search <query>`

Search official repositories and the AUR (if enabled) for packages matching the query.

```bash
cpac search firefox
```

Results are sorted by relevance (exact match, starts-with, contains, description) then by source (Official > ThirdParty > AUR). By default, the top 25 results are shown.

### Flags

| Flag | Description |
| --- | --- |
| `--all` | Show all results instead of the default 25 |

### Example

```bash
$ cpac search firefox
Package                             Version        Source           Description
firefox                             128.0esr-1     Official         Standalone web browser from mozilla.org
firefox-developer-edition          130.0b9-1      Official         Developer Edition of the Firefox web browser
...

Showing 25 of 87 results. Use --all to view everything.
```

---

## `cpac trust <package>`

Show a full trust report for a named package. Resolves the package from official repos or AUR (if enabled) and runs the complete trust analysis algorithm.

```bash
cpac trust firefox
```

### Example

```
$ cpac trust firefox

Package:          firefox
Version:          128.0esr-1
Repository:       Official
Trust Tier:       Official
Maintainer:       Remote Maintainers
URL:              https://www.mozilla.org/en-US/firefox/
Popularity:       High
Installed Size:   219.5 MB
Licenses:         MPL2
Dependencies:     12 dependencies

══════════════════════════════
Trust Score: 92/100
Recommendation: SAFE
══════════════════════════════

Signal                          Points
Repository Source               +30/30
Package Age                     +15/15
Maintainer                      +13/15
Popularity                      +15/15
Last Updated                    +15/15
```

---

## `cpac install <package>`

Install a package with trust analysis shown before proceeding. Supports both official repositories and AUR packages.

```bash
cpac install firefox
```

### Flags

| Flag | Description |
| --- | --- |
| `--force` | Skip trust analysis and confirmation prompt |
| `--dry-run` | Show what would be installed without actually installing |

### Behavior

1. Resolves the package from official repos or AUR
2. Runs trust analysis and displays the report (unless `--force`)
3. For upgrades: diffs the new PKGBUILD against the cached version and flags suspicious changes
4. Prompts for confirmation (unless `--force`)
5. Installs the package using the appropriate backend
6. Caches the PKGBUILD for future diffing

### Backend Selection

| Source | Backend |
| --- | --- |
| Official / ThirdParty | `pacman` |
| AUR | `paru` (preferred) or `yay` (auto-detected) |

### Example

```bash
$ cpac install firefox

Trust Score: 92/100 (SAFE)
Continue? [Y/n]

[installing...]
$ firefox installed successfully.
```

```bash
$ cpac install google-chrome --dry-run

Trust Score: 65/100 (MODERATE)
[DRY RUN] Would install 'google-chrome' using yay backend
```

---

## `cpac remove <package>`

Remove an installed package. Shows the trust report before removal to inform the user.

```bash
cpac remove firefox
```

### Flags

| Flag | Description |
| --- | --- |
| `--recursive` | Also remove unneeded dependencies (`pacman -Rs`) |
| `--force` | Skip trust analysis and confirmation prompt |

### Behavior

1. Checks that the package is installed (errors if not)
2. Displays the trust report (unless `--force`)
3. Prompts for confirmation (unless `--force`)
4. Removes the package

---

## `cpac update`

Update package databases. Runs `pacman -Sy` for official repositories.

```bash
cpac update
```

### Flags

| Flag | Description |
| --- | --- |
| `--aur` | Also update AUR databases (useful when AUR is disabled but you want a one-time AUR update) |

### Behavior

1. Syncs official package databases (`pacman -Sy`)
2. If AUR is enabled (or `--aur` is passed): syncs AUR databases via `paru -Sy` or `yay -Sy`
3. Clears cached metadata since repository state has changed

---

## `cpac audit [package]`

Run a trust audit on installed packages.

```bash
cpac audit          # audit all installed packages
cpac audit firefox  # audit a single package
```

### System Audit (no arguments)

Scans all installed packages and produces:

- **Summary**: Total installed packages with breakdown by trust tier (Official, ThirdParty, Community, Unknown)
- **Official notices**: Packages from distro-specific repos (EndeavourOS, CachyOS) noted as excluded from warnings
- **Warnings**: Packages with trust score below 60 or Unknown tier, sorted by score

After printing the summary, prompts "View Details? [Y/n]" to show individual trust reports for each warning.

### Single Package Audit

Audits one installed package and prints its trust report. Errors if the package is not installed.

### Example

```
$ cpac audit

Installed Packages: 842

  Official:     721
  Third Party:   86
  Community:     30
  Unknown:        5

12 package(s) from official repositories (EndeavourOS, CachyOS) are excluded from warnings.

Warnings:
  foo-bin       [Trust: 34/100 -- New maintainer, recent PKGBUILD change]
  bar-git       [Trust: Unknown -- Installed outside CPAC]
  baz-nightly   [Trust: 41/100 -- No signed package]

View Details? [Y/n]
```

---

## `cpac diff <package>`

Show the PKGBUILD diff for an AUR package. Compares the cached PKGBUILD (from a previous CPAC install) against the current version from the AUR.

```bash
cpac diff firefox
```

### Behavior

1. Resolves the package (must be an AUR package)
2. Retrieves the cached PKGBUILD and fetches the current one from AUR
3. Displays the diff with color-coded output:
   - **Green** (`+`): Added lines
   - **Red** (`-`): Removed lines
   - **Yellow** (`!`): Suspicious patterns detected

### Suspicious Patterns Detected

| Pattern | Risk |
| --- | --- |
| `curl \| sh`, `wget \| bash` | Remote script execution |
| `eval`, `exec` | Inline script execution |
| `rm -rf` outside pkgdir/srcdir | Aggressive file deletion |
| Base64/hex decoding | Obfuscation |
| Dynamic `pkgver` from network | Untrusted version source |
| `pip install`, `npm install`, `cargo install` | Language package manager in build |
| Modifications to `/etc/`, `/usr/`, `/bin/` | System path modifications |

---

## `cpac config`

View and change CPAC configuration.

```bash
cpac config
```

Displays the current configuration and presents an interactive menu for changing the crowdsourced data submission level.

### Options

| Choice | Label | Description |
| --- | --- | --- |
| `1` | No submission | Don't submit anything |
| `2` | Hash/signature only | Submit only hash/signature data (default) |
| `3` | Full PKGBUILD | Submit the full PKGBUILD text |

See [Configuration](configuration.md) for full details.

---

## `cpac aur <enable|disable>`

Enable or disable AUR support.

```bash
cpac aur enable    # enable AUR
cpac aur disable   # disable AUR
```

When disabled, CPAC only queries official repositories. When enabled, search, trust, and install operations include the AUR.

---

## `cpac clear-cache`

Clear the local metadata cache at `~/.cpac/cache/`.

```bash
cpac clear-cache
```

This removes all cached search results, trust reports, and PKGBUILD snapshots. The cache is rebuilt automatically on next use.

---

## Exit Codes

| Code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | Error (package not found, not installed, network failure, etc.) |

## Configuration File Location

| Path | Contents |
| --- | --- |
| `~/.cpac/config.toml` | User configuration (AUR toggle, consent level) |
| `~/.cpac/cache/` | Metadata cache (packages, trust, PKGBUILDs) |
