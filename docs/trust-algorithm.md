---
title: Trust Algorithm
description: How CPAC scores package trust.
order: 5
---

# Trust Algorithm

CPAC assigns every resolved package a score from 0 to 100. The score is built from metadata available via `pacman -Si` and the AUR RPC API.

The score is explainable: `cpac trust <package>` prints each signal, the points it contributed, and the reason.

## Trust Tiers

Trust tier is separate from trust score. CPAC generates an independent score for every package regardless of source.

| Tier | Description |
| --- | --- |
| Official | Package from an official Arch Linux repository |
| ThirdParty | Package from a third-party (non-official) repository |
| Community | Package from the AUR |
| Unknown | Package source cannot be determined |

## Signals

| Signal | Max | Notes |
| --- | ---: | --- |
| Repository source | 30 | Official packages receive the strongest source score. AUR packages receive a smaller community-source score. |
| Package age | 15 | Older packages receive more points. Official packages receive a conservative default. |
| Maintainer | 15 | Maintained packages score higher. Orphaned AUR packages are penalized. |
| Popularity | 15 | AUR vote counts are used when available. Official packages receive a conservative default. |
| Last updated | 15 | Recently updated packages score higher. Official packages receive a conservative default. |
| Out-of-date flag | -10 | AUR packages flagged out-of-date lose points. |

### Repository Source (max +30)

| Source | Points |
| --- | ---: |
| Official repo | +30 |
| Third-party repo | +15 |
| AUR | +10 |
| Unknown | +0 |

### Package Age (max +15)

Based on the `first_submitted` timestamp:

| Age | Points |
| --- | ---: |
| 0 -- 30 days | +2 |
| 31 -- 180 days | +5 |
| 181 -- 365 days | +8 |
| 1 -- 2 years | +11 |
| 2 -- 5 years | +14 |
| 5+ years | +15 |

If metadata is unavailable, partial credit is awarded by source type: Official +13, ThirdParty +8, AUR/Unknown +5.

### Maintainer (max +15)

| Status | Points |
| --- | ---: |
| Orphaned (no maintainer) | **-5** |
| Maintained, official packager | +13 |
| Maintained, non-official | +10 |

If metadata is unavailable, partial credit is awarded by source type.

### Popularity / Votes (max +15)

| Votes | Points |
| --- | ---: |
| 0 -- 5 | +2 |
| 6 -- 25 | +5 |
| 26 -- 100 | +8 |
| 101 -- 500 | +11 |
| 501 -- 2000 | +13 |
| 2001+ | +15 |

If metadata is unavailable, partial credit is awarded by source type.

### Last Updated Recency (max +15)

| Time since update | Points |
| --- | ---: |
| 0 -- 7 days | +15 |
| 8 -- 30 days | +13 |
| 31 -- 90 days | +11 |
| 91 -- 180 days | +8 |
| 181 -- 365 days | +5 |
| 366 -- 730 days | +3 |
| 730+ days | +1 |

If metadata is unavailable, partial credit is awarded by source type.

### Out-of-Date Penalty (-10)

If the package is flagged as out-of-date in the AUR, 10 points are deducted from the score.

## PKGBUILD Diff Penalties

When upgrading an already-installed package, CPAC diffs the new PKGBUILD against the cached version. Each detected suspicious pattern adds a **-10 point penalty** signal to the trust report.

| Pattern | Example |
| --- | --- |
| Remote script execution | `curl ... \| sh`, `wget ... \| bash` |
| Inline script execution | `eval`, `exec` |
| Aggressive file deletion | `rm -rf` outside pkgdir/srcdir |
| Obfuscation | Base64 or hex decoding |
| Dynamic version from network | `pkgver` generated via remote call |
| Language package manager in build | `pip install`, `npm install`, `cargo install` |
| System path modifications | Writes to `/etc/`, `/usr/`, `/bin/`, `/sbin/` outside pkgdir/install |

## Unknown Metadata Handling

If a package has **zero negative signals** but has unknown metadata (missing age, maintainer, popularity, or recency), the recommendation is floored at **Moderate** regardless of score. This prevents packages with simply unavailable metadata from being labeled as Caution or Warning.

## Recommendations

| Score | Recommendation | Color |
| ---: | --- | --- |
| 80 -- 100 | Safe | Green |
| 60 -- 79 | Moderate | Green |
| 40 -- 59 | Caution | Yellow |
| 20 -- 39 | Warning | Red |
| 0 -- 19 | Danger | Red |

## Scope

This is the Week 1 scoring model. Later phases can add:

- Build history verification
- Package integrity checks
- Security advisory databases
- Local audit history
- Maintainer history from a CPAC trust database
- Crowdsourced PKGBUILD snapshots for fresh-install diffing

See [Configuration](configuration.md) for the crowdsourced data consent options.
