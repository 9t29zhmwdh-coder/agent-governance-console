# Dashboard Guide: Agent Governance Console

## Overview

`GET /dashboard` serves a single self-contained HTML page (no build
step, no framework, no external CDN -- everything inline in one file,
`agc-api/static/dashboard.html`, embedded into the binary at compile
time via `include_str!`) that gives a lightweight browser UI over the
REST API `agc-api` already exposes.

**Scoping note:** `ROADMAP.md`'s original wording named a "Tauri or WASM
frontend" for this item. AGC is a REST API server the operator runs
themselves (see the README's "How it runs"), not a desktop or installed
app, so a Tauri shell would add an entire second packaging/distribution
story for no functional gain; a Rust/WASM frontend (Yew/Dioxus) would
add a `wasm32-unknown-unknown` build toolchain and a compiled-asset
pipeline to serve the same thing a ~9KB vanilla-JS page already does
against this REST API with zero new dependencies. This is a deliberate,
disclosed scoping decision, not an oversight.

---

## Usage

```bash
cargo run --bin agc-api
# then open http://127.0.0.1:8080/dashboard in a browser
```

Enter a tenant ID in the header bar (persisted to `localStorage`) to see
that tenant's span count, audit log, and compliance report. If RBAC is
enabled (`AGC_JWT_SECRET` or `AGC_AAD_TENANT_ID`, see `docs/api_reference.md`),
also enter a bearer token -- it's sent as the `Authorization` header on
every request the page makes, exactly like a `curl` client would.

### What it shows

- **Health**: `GET /health` status and version.
- **Policies**: `GET /api/v1/policies/count` (global).
- **Traces**: `GET /api/v1/traces/count` for the entered tenant.
- **Tenants Seen**: `GET /api/v1/tenants` (no tenant header needed).
- **Audit Log**: `GET /api/v1/audit?limit=&offset=`, paginated (20 records per page), with Prev/Next.
- **Compliance Report**: `GET /api/v1/compliance/report`, rendered as-is (the same Markdown the API returns, see `docs/compliance.md`).

The page makes no server-side calls of its own beyond serving this one
static file -- every data request is a same-origin `fetch()` from the
browser straight to the REST endpoints above, so it's exactly as
authenticated, tenant-isolated, and rate-limited as using `curl` against
the same endpoints would be.

---

## What's verified vs. not

Real, tested: an integration test in `agc-api` confirms `GET /dashboard`
returns real HTML (`text/html; charset=utf-8`, the real title, no tenant
header required) that references every REST endpoint it depends on. The
page's pure rendering functions (`renderAuditTable`, `renderTenants`,
`outcomeBadge`) are executed for real under Node against
realistically-shaped API response data and checked for correct output,
not just that the JavaScript parses.

Not verified: actual browser rendering (layout, CSS, click-through
interaction) -- no headless browser was available in the environment
this was built in. The underlying REST endpoints the page calls are
independently covered by `agc-api`'s own integration test suite (see
`docs/api_reference.md`), so the data contract is solid; only the visual
presentation itself is unverified.
