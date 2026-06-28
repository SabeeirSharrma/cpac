COMBINED WITH PATCHES FOR v0.7.0 (v0.7.1 and v0.7.2)
# CPAC v0.8.2 Release Notes

## Overview

v0.8.2 aligns the advisory status system with bidirectional trust attestations. Advisories can now be positive (`safe` = +10) or negative (`suspicious`/`warning`/`malicious`), making the trust score more accurate.

## Changes

### Bidirectional Advisory Statuses

- **`safe`** — positive attestation, package verified clean (+10 trust signal)
- **`suspicious`** — under investigation, proceed with caution (-15)
- **`warning`** — credible concern, not yet confirmed (-20)
- **`malicious`** — confirmed malicious (-30)
- **`resolved`** — was malicious/suspicious, now clean (0, neutral)
- Old `confirmed` → `warning`, old `suspected` → `suspicious`
- `confirmed_malicious` dropped — `malicious` is sufficient (publication implies review)

### Trust Score — Status-First Scoring

- Advisory penalty now determined by **status** (not severity)
- `safe` advisories add a positive signal, not just absence of penalty
- Severity still displayed in reports but status drives the score

---

# CPAC v0.8.1 Release Notes

## Overview

v0.8.1 adds a source-based self-update system. CPAC always builds from source — the updater clones the repo at the latest tag, runs `cargo build --release`, and replaces the binary while preserving all user config.

## Changes

### Source-Based Self-Update

- **`cpac upgrade`** — clones repo at latest tag, builds from source, replaces binary
- **Version check on every run** — cached 24h, shows notice if newer version available
- **`--no-check-updates`** — skip version check on any command
- **Config preserved** — `~/.cpac/` (config, trust-db, cache) is never modified during upgrade
- **Sudo handling** — auto-detects if `/usr/local/bin` needs elevated permissions
- **Prerequisite checks** — verifies `git` and `cargo` are installed before upgrading
- **Safe replacement** — renames current binary, copies new one, cleans up

### Panel Redesign — Unified Review Workflow

All three panels (volunteer, maintainer, admin) now share a single "Review" tab:

- **Package list** — auto-fetched on load, shows packages needing advisories
- **Automated compare** — LCS diff runs on package select, highlights suspicious patterns
- **AI analysis** — on-demand via NVIDIA NIM, structured response with recommendation, summary, severity, affected/safe versions
- **Layout toggle** — Tabs or Side-by-Side, persisted to `localStorage`
- **Notes system** — floating notes button, auto-saved per package, cleared on publish
- **Recompare** — re-run with different versions

### NVIDIA NIM AI Integration

- Worker proxies requests to NVIDIA NIM (API key stays server-side)
- Reasoning model (`nemotron-3-super-120b-a12b`) for security-focused diff analysis
- Nano model (`nemotron-3-nano-30b-a3b`) for weekly report summaries
- 3-hour cache in Supabase `ai_analysis` table

### Weekly Email Reports via Resend

- Reports generated daily, sent exactly 7 days after previous report per user
- Staggered by account creation date (Mon→Mon, Wed→Wed, etc.)
- HTML table in email body with submissions, approval rate, trust tier
- Zero activity = no email that week
- Ephemeral: stored→sent→deleted

### Cloudflare Cron Trigger

- Daily at midnight UTC (`0 0 * * *`)
- Calls `/reports/generate` then `/reports/send`
- Worker config migrated to `wrangler.jsonc`

### Account Management

- Admin panel creates volunteer/maintainer accounts (random password emailed via Resend)
- No public signups — admin-only account creation
- `POST /accounts/create` endpoint on Worker

### RLS & Auth Fixes

- `SECURITY DEFINER` functions prevent recursive RLS on profiles
- All panel auth uses `currentSession.access_token` as Bearer token
- Panels call Supabase REST API directly (Worker proxy URL unreachable due to missing DNS CNAME — now fixed)

---

# CPAC v0.7.2 Release Notes

## Overview

v0.7.2 adds paru preference to help text, fixes donate link trailing slash, and expands suspicious pattern detection for npm/bun pipe-to-shell attacks.

## Changes

- **Paru preference in help** — `cpac --help` now mentions Paru is preferred (yay still supported)
- **Donate link trailing slash** — Fixed to `https://thecinderproject.qd.je/donate/`
- **npm/bun pipe-to-shell detection** — Pass 2 now catches `npm install | sh`, `bun install | sh`, `npx | sh`, `curl | npx`, `wget | npx` patterns
- **Unknown package behavior** — Local scoring still runs when package not in trust DB; missing DB data shown as neutral signals (+0), not penalties

---

# CPAC v0.7.1 Release Notes

## Overview

v0.7.1 adds a donate link to the help output and continues the multi-session site redesign.

## Changes

- **Donate link in help** — `cpac --help` now displays `Donate: https://thecinderproject.qd.je/donate`

---
