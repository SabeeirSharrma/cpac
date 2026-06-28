# CPAC v0.9.1 — Patch: Direct Worker URL + Brand Fix

## Overview

Patch release fixing the trust-db connection (Cloudflare proxy SSL broken) and updating the AI report prompt branding.

---

## Changes

### Trust-DB Connection Fix

The custom domain proxy (`api.thecinderproject.qd.je`) has an SSL certificate provisioning issue. CPAC now connects directly to the Cloudflare Worker at `cpac-trust-db-api.sabplay-idk.workers.dev`.

**Files**: `src/trust_db.rs`

### AI Report Prompt Brand Update

Weekly report insights now say "The Cinder Project, under the CPAC Trust DB division" instead of "The CPAC Trust DB project".

**Files**: Worker `src/index.ts`

---

## Files Changed (v0.9.1)

- `Cargo.toml` — version bump to 0.9.1
- `src/trust_db.rs` — direct worker URL

---

# CPAC v0.9.0 — Trust Scoring Overhaul & Polish

_(see previous entry)_
