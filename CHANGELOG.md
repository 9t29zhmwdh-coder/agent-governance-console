# Changelog: Agent Governance Console

All notable changes to this project will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [1.0.0] - 2026-07-18

**Enterprise GA.** Ships the final item ("SLA: p99 ingest latency < 10ms
for 1K spans/s", item 8 of 8) from ROADMAP.md's "v1.0.0: Enterprise GA"
milestone, completing it. All 8 items are now shipped: multi-tenant mode
(v0.6.0), RBAC (v0.7.0), Microsoft Sentinel export (v0.8.0), Entra ID
Managed Identity for all Azure integrations (v0.9.0), a Responsible-AI
compliance report (v0.10.0), a dashboard UI (v0.11.0), a Helm chart
(v0.12.0), and this SLA verification.

### Added
- `agc-cli bench ingest`: a real HTTP load generator against `POST /api/v1/traces`, evenly-spaced (not a synchronous burst) requests at a configurable rate/duration, reporting p50/p95/p99/max ingest latency and exiting non-zero if p99 doesn't clear the 10ms SLA target.
- New `agc-core::bench` module: `percentile`, `LatencyReport` -- pure, unit-tested percentile math (6 tests) backing the CLI tool above.
- New `docs/performance.md`: full SLA methodology, findings, and measured numbers.

### Fixed
- A real server-side bottleneck found while verifying the SLA: `AppState.tenants` was a `tokio::sync::Mutex<HashMap<...>>`, so **every** trace/audit request -- even for an already-existing tenant -- serialized behind one global exclusive lock just to do a HashMap read. Switched to `RwLock`: warm lookups (the overwhelming majority in practice) now take a shared read lock and never block each other; only a brand-new tenant's very first request takes the exclusive write lock (with a double-check to avoid a duplicate-creation race).
- A benchmark methodology bug in the first draft of `bench ingest` itself: firing all `rate` requests for a second in one synchronous burst measured artificial same-instant lock contention (p99 ~29ms at 1000 req/s), not steady-state latency a real 1000/s arrival rate actually produces. Fixed by spacing requests evenly; p99 dropped to well under 1ms. Both findings, and why the first "SLA not met" reading was real-but-misleading rather than wrong, are documented in full in `docs/performance.md`.

### Verified
- p99 well under 1ms at 1000 req/s (roughly 20x margin under the 10ms target), sustained over 20 seconds, holding at 2000 req/s (2x target), and still only ~1.4ms in the realistic worst case: every span matching a real policy rule, every resulting audit record actually persisted to a real SQLite file on disk (not in-memory) -- verified by inspecting the file, not just trusting the run's exit code.

## [0.12.0] - 2026-07-18

Ships the "Helm chart for Kubernetes deployment" item from ROADMAP.md's
"v1.0.0: Enterprise GA" milestone (item 7 of 8).

### Added
- Root `Dockerfile`: multi-stage build (`rust:1.90-bookworm` -> `debian:bookworm-slim`, non-root user), plus `.dockerignore`.
- `helm/agent-governance-console`: Deployment, Service, and optional Ingress, HorizontalPodAutoscaler, PersistentVolumeClaim (audit log), ConfigMap (governance policies), RBAC env wiring for both HMAC and Entra ID modes, and Azure Workload Identity annotations for AKS. Both probes hit `GET /health`.
- New `AGC_BIND` env var (`agc-api`): overrides the REST API's bind address (defaults to `127.0.0.1:8080`, same as before).
- New `docs/helm.md`.

### Fixed
- A real bug found while building the Docker image: `rust:1.82-bookworm`'s Cargo couldn't resolve a dependency requiring a newer Cargo edition feature -- fixed by bumping the build stage to `rust:1.90-bookworm`.
- A real bug found while running the built container: the server's default bind address (`127.0.0.1`) is unreachable through Docker's port mapping or a Kubernetes Service/probe. Fixed with the new `AGC_BIND` env var, set to `0.0.0.0:8080` by default inside the container image.

### Verification
- `helm lint`, `helm template` with every conditional path exercised at once, and `kubectl apply --dry-run=server` against a real k3s API server all passed. A genuine `helm install` of the Docker-built image into a local k3s cluster reached `1/1 Ready` (both probes passing for real) and served real traffic through the Kubernetes Service. Colima, Docker, Helm, and kubectl were installed specifically to perform this verification (none were present in the environment beforehand); see `docs/helm.md` for exactly what was and wasn't covered.

## [0.11.0] - 2026-07-18

Ships the "Dashboard UI" item from ROADMAP.md's "v1.0.0: Enterprise GA"
milestone (item 6 of 8), scoped as a static HTML/CSS/JS page rather than
the Tauri/WASM frontend the item's original wording named -- see the
"Scoping note" in `docs/dashboard.md` and `ROADMAP.md` for why.

### Added
- `GET /dashboard`: a single self-contained static page (`agc-api/static/dashboard.html`, embedded via `include_str!`, zero new dependencies) covering health, tenant list, policy count, per-tenant span count, a paginated audit table, and the compliance report -- all client-side `fetch()` calls against the existing REST endpoints, with an optional bearer-token field for RBAC-enabled deployments.
- New `docs/dashboard.md`.
- 1 new integration test in `agc-api` confirming the page is real HTML referencing every endpoint it needs; the page's pure rendering functions are also executed for real under Node against realistic API-shaped data (see Known limitation).

### Known limitation
- No headless browser was available in the environment this was built in, so actual browser rendering (layout, CSS, click-through interaction) is unverified -- only the served HTML's structure and the JS rendering functions' output (run under Node, not a browser) were checked for real.

## [0.10.0] - 2026-07-18

Ships the "Compliance report export aligned with Microsoft AI Responsible
Use guidelines" item from ROADMAP.md's "v1.0.0: Enterprise GA" milestone
(item 5 of 8).

### Added
- New `agc-core::compliance` module: `ComplianceReport::generate(tenant_id, &AuditLog, &TraceStore, &PolicyEngine, SecurityPosture)` builds a report against 4 of [Microsoft's 6 Responsible AI principles](https://learn.microsoft.com/azure/machine-learning/concept-responsible-ai) (Accountability, Transparency, Reliability and Safety, Privacy and Security) from a tenant's real audit/trace data; `to_markdown()` renders it. Fairness and Inclusiveness are explicitly reported as out of scope, since they require model-output-level evaluation this governance/audit layer never collects.
- `GET /api/v1/compliance/report` (tenant-scoped, `Viewer` role): Markdown by default, `?format=json` for machine-readable output.
- `agc-core::AuditLog::all_records` made public (was already implemented for `export_ndjson`/`export_csv`, just not exposed).
- 8 new tests: 6 unit tests in `agc-core::compliance` (outcome counting, per-policy grouping, the repeated-block-agent threshold, span error-rate math, the empty-tenant case, and that the rendered Markdown covers every section including the out-of-scope disclosure), 2 integration tests in `agc-api` (a real block-action policy driven through 3 real requests, checked in both Markdown and JSON; the missing-tenant-header rejection).
- New `docs/compliance.md`.

### Known limitation
- Not reviewed by a compliance or legal professional -- a factual summary of AGC's own recorded governance data, not a certification of regulatory compliance.

## [0.9.0] - 2026-07-18

Ships the "Entra ID managed identity support for all Azure integrations
(no client secrets)" item from ROADMAP.md's "v1.0.0: Enterprise GA"
milestone (item 4 of 8).

### Added
- `agc_azure::OtlpExporter::new` now takes an optional bearer token, sent as a static `Authorization: Bearer <token>` header on every OTLP export request -- the shape Azure Monitor's native OTLP endpoint requires.
- `AGC_TELEMETRY_MANAGED_IDENTITY` (system-assigned) or `AGC_TELEMETRY_MANAGED_IDENTITY_CLIENT_ID` (user-assigned) fetches that token via Managed Identity, scoped to `https://monitor.azure.com/.default`, before the exporter is built. No client secret anywhere in this flow.
- New `TelemetryConfig` fields: `use_managed_identity: bool`, `managed_identity_client_id: Option<String>`.
- New `AppState.otlp_authenticated: bool`, distinct from "authentication was requested": only `true` if a token was actually obtained and attached, so the startup log line (and any future health/status endpoint) can't misreport a failed token fetch as a successful one.
- 2 new tests: a unit test in `agc-azure::otlp` proving the header actually reaches the wire (mock server matches on the literal `Authorization` header value), and a real end-to-end integration test in `agc-api` that exercises the *actual* default IMDS endpoint (unreachable off Azure, 2s timeout) and confirms the server still starts, OTLP still works, and `otlp_authenticated` correctly stays `false`.

### Changed (breaking)
- `agc_azure::OtlpExporter::new(endpoint, service_name)` is now `new(endpoint, service_name, bearer_token: Option<&str>)`.
- `agc_api::AppState::from_config` is now `async` (it awaits a real IMDS call when Managed Identity is requested); all callers updated (`agc-api/src/main.rs`, `agc-api/tests/api_integration.rs`).

### Known limitation
- The Managed Identity token is fetched once, at startup, and is not refreshed for the life of the process. A genuinely long-running deployment needs a restart (or a future refresh mechanism, not built yet) before a long-lived token expires. Microsoft Graph and Azure Monitor Logs Ingestion (`agc-cli azure list-agents` / `push-audit`) were already Managed-Identity-authenticated since v0.3.0 and are unaffected by this release; this item closed the one remaining unauthenticated Azure integration path (OTLP).

## [0.8.0] - 2026-07-18

Ships the "Microsoft Sentinel analytics rule export" item from
ROADMAP.md's "v1.0.0: Enterprise GA" milestone (item 3 of 8).

### Added
- `agc-core::sentinel` module: `SentinelRule` (name, description, severity, KQL query), `builtin_rules(table)` returning 4 built-in governance-focused analytics rules (repeated policy blocks by one agent, a new agent's first action being a block, a portfolio-wide warn/alert volume spike, an agent triggering many distinct policies), `to_kql()` (raw query text) and `to_arm_resource()` (a `Microsoft.SecurityInsights/alertRules`, kind `Scheduled`, ARM resource snippet).
- `agc-cli sentinel export --table <name> --format kql|arm --output-dir <dir>`: `kql` writes one `.kql` file per rule (ready to paste into Sentinel's "Analytics rules → Create → Set rule logic" editor); `arm` writes a single deployable ARM template containing all 4 rules as resources (`az deployment group create`). Table name defaults to `AGCAudit_CL` (what `scripts/azure_setup.sh` provisions) and is fully parameterized for a renamed table.
- 5 new unit tests in `agc-core::sentinel` (rule count/shape, custom table name substitution, a schema-column validator that tokenizes every query and rejects any column-like identifier not in `azure_setup.sh`'s actual custom table schema, ARM resource shape, raw KQL passthrough), plus a real end-to-end CLI smoke test (both formats actually written to disk and inspected: real KQL text, valid parseable ARM JSON with all 4 resources well-formed, the `--format` error path, and the custom-table-name path).

### Known limitation
- Correct against the exact column names `azure_setup.sh`'s custom table declares, but not verified against a live Sentinel workspace (none was available while building this) — same disclosed-limitation pattern as the rest of this portfolio's Azure integrations.

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
