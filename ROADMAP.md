# Roadmap : Agent Governance Console

## v0.1.0 : Initial Import ✅

- Rust workspace: `agc-core`, `agc-api` (Axum), `agc-cli`
- Trace ingestion: `TraceSpan`, `TraceStore`, sorted by `started_at`
- Policy engine: `GovernancePolicy`, `PolicyRule`, condition/action stubs
- Audit log: `AuditRecord`, NDJSON + CSV export
- Opt-in telemetry: `TelemetryConfig`, `TelemetrySink`, OTLP-ready
- REST API: `/health`, `/api/v1/traces/count`, `/api/v1/audit/count`
- 6 unit tests covering all core subsystems
- Bilingual README (EN / DE)
- Full documentation skeleton

## v0.2.0 : Full REST API ✅

Shipped as v0.3.0 (not v0.2.0): the version had already advanced past 0.2.0
via unrelated patch releases by the time this milestone's features landed,
so per this portfolio's own SemVer discipline the release became a Minor
bump to the next available version instead of a downgrade. All items below
are complete regardless of the release number.

- [x] POST `/api/v1/traces` (ingest span)
- [x] GET `/api/v1/traces/{trace_id}` (retrieve full trace)
- [x] POST `/api/v1/policies` (load policy JSON)
- [x] GET `/api/v1/audit` (paginated audit log query)
- [x] GET `/api/v1/audit/export.csv` and `/export.ndjson` (streaming export)
- [x] Policy evaluation on every ingested span (real-time gate): `SpanLevelAtLeast`, `TokenBudgetExceeded` (reads the `tokens` span attribute), `OperationMatches` (single-wildcard glob) all have real evaluation logic now, not stubs; a matched `Block` rule rejects the span with `403` and it is never stored
- [x] SQLite persistence for audit log (`rusqlite`): `AuditLog::open(path)` for a real file, `AuditLog::new()` still in-memory by default; wired into the running server via `AGC_AUDIT_DB_PATH` / `AppState::with_audit_db`/`from_config`

## v0.3.0 : Azure Integration ✅

Shipped as v0.4.0 (not v0.3.0): the version number was already at 0.3.0
before this milestone landed (the previous "v0.2.0: Full REST API"
milestone shipped as 0.3.0 for the same reason), so per this portfolio's
own SemVer discipline the release became a Minor bump to the next
available number instead of a collision. All items below are complete
regardless of the release number.

- [x] OTLP exporter to Azure Monitor (via `opentelemetry-otlp`): real OTLP/HTTP export, wired into `agc-api`'s trace ingestion via `AGC_TELEMETRY_ENDPOINT`/`AGC_TELEMETRY_SERVICE_NAME`. Uses the batch span processor (its own background thread) rather than the simple/synchronous one — the synchronous version deadlocked when called from inside axum's already-running Tokio runtime, a real bug found and fixed during development.
- [x] Azure Log Analytics DCR ingest endpoint for audit NDJSON: `agc-cli azure push-audit` reads a local NDJSON export and POSTs it as a JSON array to a DCR via the Logs Ingestion API.
- [x] Microsoft Graph integration (read agent app registrations): `agc-cli azure list-agents` queries app registrations tagged `agc-agent`.
- [x] AAD managed identity authentication (no secrets in config): `ManagedIdentityCredential` (IMDS-based) backs both commands above; never a client secret in AGC's own config.
- [x] `scripts/azure_setup.sh` (create DCR, workspace, app registration): extended to provision the Log Analytics workspace, Application Insights, the custom audit table, a Data Collection Endpoint, a Data Collection Rule, and a demo `agc-agent`-tagged app registration.

**What's verified vs. not:** all four `agc-azure` integration points have real, passing tests against a local mock HTTP server (`wiremock`), including a regression test for the OTLP deadlock and one for a Managed-Identity timeout hang (IMDS is unreachable off Azure and was found to hang indefinitely without a client-side timeout — also fixed). `ManagedIdentityCredential`'s real IMDS endpoint and the extended `azure_setup.sh` have **not** been exercised against a real Azure subscription (none was available while building this): they are correct-by-construction against the documented contracts, not live-verified. See `docs/azure_integration.md`.

## v0.4.0 : Policy DSL ✅

Shipped as v0.5.0 (not v0.4.0): the version was already at 0.4.0 from the
previous milestone, so per this portfolio's own SemVer discipline the
release became a Minor bump to the next available number.

- [x] YAML-based policy DSL (see `docs/policy_dsl.md`): `GovernancePolicy::from_yaml` parses YAML (and, since YAML 1.2 is a JSON superset, plain JSON too — one parser for both). New `agc-cli policy validate <file>` command for offline validation.
- [x] Hot-reload policies from file (watch via `notify`): `AGC_POLICY_DIR` loads every `*.yaml`/`*.yml`/`*.json` file in a directory at startup and reloads on every filesystem change; a parse error keeps the previous policy set instead of wiping it.
- [x] Token budget enforcement (count from span attributes): already shipped in the v0.2.0 milestone (`TokenBudgetExceeded` reads `attributes.tokens`), just never checked off here.
- [x] OPA-compatible policy export (Rego stub): `agc-cli policy to-rego <file>` renders a structural Rego module (one `deny`/`warn`/`alert` rule per policy rule) — explicitly a starting point for hand-porting, not a full semantic translation (see `docs/policy_dsl.md` for exactly what's approximate).

## v1.0.0 : Enterprise GA

This milestone ships incrementally across several Minor releases as each
item lands (unlike the three previous milestones, this one is genuinely
too large for one release); v1.0.0 itself is declared only once every
item below is checked off.

- [x] Multi-tenant mode (tenant isolation in trace/audit stores): shipped in v0.6.0. `X-Tenant-Id` header (required on every trace/audit endpoint, no silent "default tenant" fallback) resolves a per-tenant `TraceStore`+`AuditLog` pair, created lazily on first use; with `AGC_AUDIT_DB_DIR` set, each tenant gets its own `{tenant_id}.sqlite` file — genuine storage-level isolation, verified by inspecting the files on disk, not just a filtered view over one shared store. `GET /api/v1/tenants` lists every tenant seen so far. Deliberately excludes policies, which stay global/shared governance across all tenants, per this item's own wording ("in trace/audit stores").
- [x] Role-based access control for REST API (JWT / AAD tokens): shipped in v0.7.0. `Authorization: Bearer <token>` gates every trace/audit/policy endpoint (`Viewer` for reads, `Admin` for writes); `AGC_JWT_SECRET` for shared-secret HS256, or `AGC_AAD_TENANT_ID`+`AGC_AAD_AUDIENCE` for Entra ID RS256 via JWKS. Opt-in: with neither set, RBAC stays off and every request is `Admin`, identical to this API's behavior before RBAC existed. The AAD/JWKS path is real-HTTP-tested against a mock server (fetch, `kid` lookup, RS256 verify all actually exercised) but not against a live Entra ID tenant (none was available while building this).
- [ ] Microsoft Sentinel analytics rule export (Kusto query templates for AGC audit events)
- [ ] Entra ID managed identity support for all Azure integrations (no client secrets)
- [ ] Compliance report export aligned with Microsoft AI Responsible Use guidelines
- [ ] Dashboard UI (Tauri or WASM frontend)
- [ ] Helm chart for Kubernetes deployment
- [ ] SLA: p99 ingest latency < 10ms for 1K spans/s

## Dual-Licensing Readiness

Assessed 2026-07-11 as a Dual-Licensing candidate (Community MIT + Commercial/Enterprise tier): governance/audit tooling for regulated environments is a plausible enterprise sales category, and AGC already targets that audience explicitly (see the README's "enterprise AI governance teams" framing). Not ready yet; blocked on:

- [ ] No authentication on the REST API at all (AAD JWT/RBAC for the REST API itself is a v1.0.0 item above; v0.3.0 only added outbound Managed Identity auth for AGC calling Azure Monitor/Graph, not inbound gating of AGC's own endpoints): an Enterprise tier needs a real auth story before it can gate anything
- [ ] No multi-tenancy (v1.0.0 item above): a Commercial tier's core value is usually per-tenant isolation and RBAC
- [x] ~~No persistence yet~~ SQLite persistence for the audit log shipped in v0.2.0 (see above); trace store and policy engine are still in-memory only, so there is still no durable data across the whole system to license access to yet

Once v1.0.0's multi-tenant mode and RBAC land, revisit: candidate Enterprise-only features would be multi-tenant isolation, RBAC/SSO, Sentinel analytics export, and compliance report generation, with the core trace/policy/audit engine staying Community/MIT.

## Out of Scope

- Model training or inference (governance of agent workflows only)
- Replacing a full observability platform (Datadog, Grafana) (AGC is a lightweight complement)
- Cloud-proprietary lock-in (Azure integration is opt-in, not a requirement)
