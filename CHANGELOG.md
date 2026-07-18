# Changelog: Agent Governance Console

All notable changes to this project will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.7.0] - 2026-07-18

Ships the "Role-based access control for REST API (JWT / AAD tokens)"
item from ROADMAP.md's "v1.0.0: Enterprise GA" milestone (item 2 of 8).

### Added
- `agc-api::auth` module: `AuthConfig::Hmac` (HS256, shared secret) and `AuthConfig::Aad` (RS256, Entra ID JWKS) bearer-token validation, plus `AuthConfig::Disabled` (the default) which treats every request as `Admin`, identical to this API's behavior before RBAC existed.
- Every trace/audit/policy endpoint now checks the caller's role: `Viewer` for GET endpoints, `Admin` for `POST /api/v1/traces` and `POST /api/v1/policies`. Missing/malformed token: `401`. Insufficient role: `403`.
- `AGC_JWT_SECRET` env var enables HS256 mode; `AGC_AAD_TENANT_ID` (+ optional `AGC_AAD_AUDIENCE`, default `api://agc`) enables Entra ID mode. Neither set: RBAC stays off.
- 12 new tests: 10 unit tests in `agc-api::auth` (including a real RSA keypair signing a real RS256 JWT, verified against a real JWKS document served by a mock HTTP server -- not just that the code compiles) and 4 new end-to-end integration tests (no-token rejection, viewer-vs-admin write gating, RBAC-disabled-by-default).

### Fixed
- A real validation bug found while writing the first tests: `jsonwebtoken`'s default `Validation` requires an `exp` claim to even be *present*, which rejected every valid, correctly-signed test token that didn't happen to carry one. Cleared `required_spec_claims` so `exp` is optional, while still enforcing it (rejecting an actually-expired token) whenever it *is* present -- verified by a dedicated test with a real expired-in-1970 token.

### Known limitation
- The Entra ID (AAD) mode's real JWKS endpoint and token issuance have not been exercised against a live Entra ID tenant (none was available while building this) -- the fetch/`kid`-lookup/RS256-verify path is proven against a real mock HTTP server instead, same disclosed-limitation pattern as `agc_azure::ManagedIdentityCredential`.

## [0.6.0] - 2026-07-18

Ships the "Multi-tenant mode" item from ROADMAP.md's "v1.0.0: Enterprise
GA" milestone. This is a Minor release, not v1.0.0: the Enterprise GA
milestone has 8 items total, this ships the first one; v1.0.0 itself is
only declared once all of them land.

### Added
- `X-Tenant-Id` header (required on every trace/audit endpoint, `400` if missing or empty — no silent "default tenant" fallback that would pool everyone's data together) resolves an isolated `TraceStore`+`AuditLog` pair per tenant, created lazily on that tenant's first request.
- `AGC_AUDIT_DB_DIR` (replaces `AGC_AUDIT_DB_PATH`): with it set, each tenant's audit log persists to its own `{tenant_id}.sqlite` file — genuine storage-level isolation, verified by inspecting the files on disk in a real end-to-end test, not just a filtered view over one shared store.
- `GET /api/v1/tenants`: lists every tenant ID seen so far (sorted).
- 5 new integration tests covering tenant isolation (a different tenant's data stays at zero), the missing-header rejection, the tenant list endpoint, and that policies correctly stay global across tenants (not tenant-scoped, per this item's own "in trace/audit stores" wording).

### Changed (breaking)
- `ConsoleConfig::audit_db_path: Option<PathBuf>` renamed to `audit_db_dir: Option<PathBuf>` — it's now a directory (one SQLite file per tenant inside it), not a single file.
- `AppState::from_config` is now infallible (`-> Self`, not `-> rusqlite::Result<Self>`): tenant stores (and their SQLite files) are opened lazily per-request now, not eagerly at startup, so there's nothing left that can fail synchronously at construction time.
- Every response from a tenant-scoped endpoint now includes a `"tenant_id"` field.

Policies are deliberately **not** tenant-scoped: a policy loaded via `POST /api/v1/policies` is shared governance that gates every tenant's ingestion, matching this roadmap item's literal wording ("tenant isolation in trace/audit stores").

## [0.5.0] - 2026-07-18

Ships the full "v0.4.0: Policy DSL" roadmap milestone (released as
0.5.0, a Minor bump, since the previous milestone had already advanced
the version to 0.4.0).

### Added
- `GovernancePolicy::from_yaml`/`to_yaml` (`agc-core`, via the `serde_norway` crate): parses YAML policy documents; since YAML 1.2 is a JSON superset, the same parser also accepts the existing JSON format unchanged.
- `PolicyEngine::load_policies_from_dir`: loads every `*.yaml`/`*.yml`/`*.json` file in a directory (non-recursive, sorted), replacing the full policy set atomically. A parse error in any file aborts that reload and leaves the previous policy set untouched, so one bad edit can't silently wipe a working configuration.
- `AGC_POLICY_DIR` env var (`agc-api`): loads policies from a directory at startup and hot-reloads on every filesystem change, via a new `agc_api::spawn_policy_hot_reload` using the `notify` crate.
- `GovernancePolicy::to_rego_stub`: renders a structural Open Policy Agent (Rego) module — one `deny`/`warn`/`alert` partial rule per policy rule. Explicitly a hand-porting starting point, not a full semantic translation of AGC's condition model (see `docs/policy_dsl.md` for exactly what's approximate, e.g. `span_level_at_least` becomes a string equality check, not a real severity-order comparison).
- `agc-cli policy validate <file>`: parses a policy file and reports whether it's valid, without needing a running server.
- `agc-cli policy to-rego <file>`: prints the Rego stub for a policy file.
- 18 new tests (14 in `agc-core` covering YAML parsing/round-tripping/directory loading/Rego generation, 1 real end-to-end `agc-api` integration test that writes an actual file to a real directory and confirms the real filesystem watcher picks it up and the loaded policy actually gates a real request), all passing; clippy clean on all targets.

### Changed
- ROADMAP.md: "Token budget enforcement" checked off — it actually shipped in the v0.2.0 milestone (`TokenBudgetExceeded` reads `attributes.tokens`) but was never marked done there.

## [0.4.0] - 2026-07-17

Ships the full "v0.3.0: Azure Integration" roadmap milestone (released as
0.4.0, a Minor bump, since the previous milestone had already advanced
the version to 0.3.0). New `agc-azure` crate; no breaking changes to
`agc-core`/`agc-api`'s existing public API beyond `AppState` gaining an
`otlp` field.

### Added
- `agc-azure` crate: `ManagedIdentityCredential` (AAD tokens via IMDS, system- or user-assigned), `MonitorIngestClient` (push audit records to an Azure Monitor DCR via the Logs Ingestion API), `GraphClient` (list Entra ID app registrations tagged `agc-agent`), `OtlpExporter` (real OTLP/HTTP span export).
- `agc-api`: `AGC_TELEMETRY_ENDPOINT`/`AGC_TELEMETRY_SERVICE_NAME` env vars wire a real `OtlpExporter` into `AppState`; every successfully ingested trace span is exported. A misconfigured endpoint logs a warning and leaves telemetry disabled rather than failing startup.
- `agc-cli azure list-agents`: lists `agc-agent`-tagged app registrations via Managed Identity + Microsoft Graph.
- `agc-cli azure push-audit`: reads a local NDJSON audit export and pushes it to an Azure Monitor DCR via Managed Identity.
- `scripts/azure_setup.sh` extended to provision the Data Collection Endpoint, Data Collection Rule, custom `AGCAudit_CL` table, and a demo `agc-agent`-tagged app registration (previously only created the Log Analytics workspace and Application Insights).
- 23 new tests (12 in `agc-azure`, 1 new integration test in `agc-api`), all against real local mock HTTP servers (`wiremock`), not just construction-only unit tests.

### Fixed
- A real deadlock: the OTLP exporter originally used a synchronous span processor that ran its HTTP export inline on the calling thread, hanging forever when called from inside axum's already-running Tokio runtime (i.e. any real request handler). Switched to the batch processor, which runs exports on its own dedicated thread.
- A real hang: `ManagedIdentityCredential` had no HTTP client timeout, so a request to an unreachable/black-holed endpoint (exactly what IMDS's `169.254.169.254` looks like off Azure) could hang indefinitely instead of failing fast. Added a 2-second timeout; `GraphClient`/`MonitorIngestClient` got a 30-second timeout for the same reason.

### Known limitations
- `ManagedIdentityCredential`'s real IMDS endpoint and the extended `scripts/azure_setup.sh` have not been verified against a live Azure subscription (none was available while building this) — both are correct-by-construction against the documented contracts, tested only via mock HTTP servers. See `docs/azure_integration.md`.
- REST API inbound authentication (JWT/AAD-gating AGC's own endpoints) is unrelated to this release and remains a v1.0.0 item; this release only added outbound Managed Identity auth for AGC calling Azure Monitor/Graph.

## [0.3.0] - 2026-07-17

Ships the full "v0.2.0: Full REST API" roadmap milestone (released as
0.3.0, a Minor bump, since patch releases had already advanced the
version past 0.2.0 before this work landed).

### Added
- `POST /api/v1/traces`: ingest a `TraceSpan`, gated by real-time policy evaluation.
- `GET /api/v1/traces/{trace_id}`: retrieve every span for a trace, 404 if none exist.
- `POST /api/v1/policies`: load a `GovernancePolicy` into the running engine.
- `GET /api/v1/audit?limit=&offset=`: paginated audit log query with total count.
- `GET /api/v1/audit/export.ndjson` and `/export.csv`: streaming audit export with correct content types.
- `PolicyCondition::matches` and `PolicyEngine::evaluate`: real condition evaluation (span level threshold, token budget read from the `tokens` span attribute, single-wildcard operation glob), replacing the previous scope-only stub. A matched `Block` rule now rejects the span with `403` and it is never stored; `Warn`/`Alert` rules record an audit entry and let the span through.
- `AuditLog::list_paginated`: SQLite-backed paginated query, ordered oldest-first.
- 15 new API integration tests and 7 new core unit tests covering the policy gate, pagination, and export content types.

### Changed
- README (EN/DE) and `docs/api_reference.md` updated to describe the now-implemented endpoints and policy condition/action schema, with a worked curl example for the policy gate.

## [0.2.1] - 2026-07-17

### Changed
- CI: added an explicit `permissions: contents: read` block to the workflow(s) that were missing one (CodeQL `actions/missing-workflow-permissions`), narrowing the default GITHUB_TOKEN scope.

## [0.2.0] - 2026-07-13

### Added

- SQLite-backed audit log (`rusqlite`, bundled): `AuditLog::new()` still defaults to an in-memory database (previous behavior), `AuditLog::open(path)` persists to a real file. Wired into the running server: set `AGC_AUDIT_DB_PATH` (or call `AppState::with_audit_db`/`from_config` directly) to persist audit records across restarts instead of losing them on every shutdown.
- Closes the persistence blocker in this repo's Dual-Licensing Readiness assessment (ROADMAP.md); no-auth-on-the-API and no-multi-tenancy remain open.

### Changed

- `AuditLog::records_for_agent`/`blocked_records` now return owned `Vec<AuditRecord>` instead of `Vec<&AuditRecord>`, since a SQLite-backed store can't hand out references into its own rows.

## [0.1.8] - 2026-07-13

### Added

- README.de.md was missing whole "Security"/"Sicherheit" and "Contributing"/"Mitwirken" sections that README.md has; added both.
- README.md's "Documentation" list was missing the "Trace Schema" doc link that README.de.md already had; added it for parity in both languages.

### Fixed

- README.de.md section order now matches README.md (Voraussetzungen/Requirements moved from the end to before Schnellstart/Quickstart).

## [0.1.7] - 2026-07-12

### Fixed

- Removed em-dashes/en-dashes across 17 files (docs, source comments, scripts, Cargo.toml descriptions), Swiss German orthography rule. Deleted stale scaffold bookkeeping files SKELETON.md and TEMPLATE_NOTES.md.
- Corrected a version drift: Cargo.toml's workspace version was still at 0.1.5 while the latest release tag was already v0.1.6.
## [0.1.6] - 2026-07-11

### Added

- Documented Dual-Licensing readiness assessment in ROADMAP.md: candidate for Community/Commercial split (governance/audit tooling for regulated environments), but blocked on auth, multi-tenancy and persistence, all already planned for later milestones.

## [0.1.5] - 2026-07-11

### Fixed

- Updated actions/checkout and codecov/codecov-action to their latest major versions in CI. GitHub is deprecating the Node.js 20 runtime, and actions still targeting it (like the previous actions/checkout@v4) were being forced onto Node 24 and crashing during their post-run cleanup step.

## [0.1.4] - 2026-07-11

### Fixed

- Corrected README hero section: only the title image and title stay centered, tagline, description and badges are now left aligned like the rest of the document

## [0.1.3] - 2026-07-10

### Fixed

- Removed em-dashes from CHANGELOG.md, replaced with colons/plain hyphens

## [0.1.2] - 2026-07-10

### Changed

- Moved the "New here? -> beginners guide" callout in README.md to the top of the file (previously only appeared near Requirements)

### Added

- Added the "New here?" beginner guide callout to README.de.md (was missing)

## [0.1.0] - <EARLIEST_COMMIT_DATE>

### Added

- Rust workspace with `agc-core`, `agc-api` (Axum) and `agc-cli` crates
- `TraceStore`: sorted in-memory span store; query by trace ID; filter error spans
- `TraceSpan`: OpenTelemetry-compatible span model with structured attributes
- `PolicyEngine`: load `GovernancePolicy` objects; resolve applicable rules per agent/operation
- `PolicyRule`: `Warn`, `Block`, `Alert` actions on span-level, token-budget and operation conditions
- `AuditLog`: append-only record store; NDJSON (Azure Log Analytics) and CSV export
- `TelemetryConfig` / `TelemetrySink`: opt-in OTLP telemetry; disabled by default
- `ConsoleConfig`: typed configuration with `default_local()` (binds to 127.0.0.1:8080)
- Axum REST API: `/health`, `/api/v1/traces/count`, `/api/v1/audit/count`
- 6 unit tests covering trace ingestion, audit export, policy resolution, telemetry opt-in
- Bilingual README (EN / DE)
- Full documentation skeleton: ARCHITECTURE, PRIVACY, ROADMAP, CONTRIBUTING, SECURITY
- Azure integration guide, trace schema, policy DSL reference, API reference
- Examples: `trace_ingestion.rs`, `policy_enforcement.rs`
- Scripts: `azure_setup.sh`, `export_audit.sh`
