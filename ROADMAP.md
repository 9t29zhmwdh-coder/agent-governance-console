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

- [x] OTLP exporter to Azure Monitor (via `opentelemetry-otlp`): real OTLP/HTTP export, wired into `agc-api`'s trace ingestion via `AGC_TELEMETRY_ENDPOINT`/`AGC_TELEMETRY_SERVICE_NAME`. Uses the batch span processor (its own background thread) rather than the simple/synchronous one: the synchronous version deadlocked when called from inside axum's already-running Tokio runtime, a real bug found and fixed during development.
- [x] Azure Log Analytics DCR ingest endpoint for audit NDJSON: `agc-cli azure push-audit` reads a local NDJSON export and POSTs it as a JSON array to a DCR via the Logs Ingestion API.
- [x] Microsoft Graph integration (read agent app registrations): `agc-cli azure list-agents` queries app registrations tagged `agc-agent`.
- [x] AAD managed identity authentication (no secrets in config): `ManagedIdentityCredential` (IMDS-based) backs both commands above; never a client secret in AGC's own config.
- [x] `scripts/azure_setup.sh` (create DCR, workspace, app registration): extended to provision the Log Analytics workspace, Application Insights, the custom audit table, a Data Collection Endpoint, a Data Collection Rule, and a demo `agc-agent`-tagged app registration.

**What's verified vs. not:** all four `agc-azure` integration points have real, passing tests against a local mock HTTP server (`wiremock`), including a regression test for the OTLP deadlock and one for a Managed-Identity timeout hang (IMDS is unreachable off Azure and was found to hang indefinitely without a client-side timeout; also fixed). `ManagedIdentityCredential`'s real IMDS endpoint and the extended `azure_setup.sh` have **not** been exercised against a real Azure subscription (none was available while building this): they are correct-by-construction against the documented contracts, not live-verified. See `docs/azure_integration.md`.
**What's verified vs. not:** all four `agc-azure` integration points have real, passing tests against a local mock HTTP server (`wiremock`), including a regression test for the OTLP deadlock and one for a Managed-Identity timeout hang (IMDS is unreachable off Azure and was found to hang indefinitely without a client-side timeout, also fixed). `ManagedIdentityCredential`'s real IMDS endpoint and the extended `azure_setup.sh` have **not** been exercised against a real Azure subscription (none was available while building this): they are correct-by-construction against the documented contracts, not live-verified. See `docs/azure_integration.md`.

## v0.4.0 : Policy DSL ✅

Shipped as v0.5.0 (not v0.4.0): the version was already at 0.4.0 from the
previous milestone, so per this portfolio's own SemVer discipline the
release became a Minor bump to the next available number.

- [x] YAML-based policy DSL (see `docs/policy_dsl.md`): `GovernancePolicy::from_yaml` parses YAML (and, since YAML 1.2 is a JSON superset, plain JSON too, one parser for both). New `agc-cli policy validate <file>` command for offline validation.
- [x] YAML-based policy DSL (see `docs/policy_dsl.md`): `GovernancePolicy::from_yaml` parses YAML (and, since YAML 1.2 is a JSON superset, plain JSON too, using one parser for both). New `agc-cli policy validate <file>` command for offline validation.
- [x] Hot-reload policies from file (watch via `notify`): `AGC_POLICY_DIR` loads every `*.yaml`/`*.yml`/`*.json` file in a directory at startup and reloads on every filesystem change; a parse error keeps the previous policy set instead of wiping it.
- [x] Token budget enforcement (count from span attributes): already shipped in the v0.2.0 milestone (`TokenBudgetExceeded` reads `attributes.tokens`), just never checked off here.
- [x] OPA-compatible policy export (Rego stub): `agc-cli policy to-rego <file>` renders a structural Rego module (one `deny`/`warn`/`alert` rule per policy rule), explicitly a starting point for hand-porting, not a full semantic translation (see `docs/policy_dsl.md` for exactly what's approximate).

## v1.0.0 : Enterprise GA ✅

This milestone shipped incrementally across 6 Minor releases (v0.6.0
through v0.12.0) as each of the 8 items below landed -- unlike the three
previous milestones, this one was genuinely too large for one release.
All 8 items are complete; v1.0.0 is declared as of this release.

- [x] Multi-tenant mode (tenant isolation in trace/audit stores): shipped in v0.6.0. `X-Tenant-Id` header (required on every trace/audit endpoint, no silent "default tenant" fallback) resolves a per-tenant `TraceStore`+`AuditLog` pair, created lazily on first use; with `AGC_AUDIT_DB_DIR` set, each tenant gets its own `{tenant_id}.sqlite` file: genuine storage-level isolation, verified by inspecting the files on disk, not just a filtered view over one shared store. `GET /api/v1/tenants` lists every tenant seen so far. Deliberately excludes policies, which stay global/shared governance across all tenants, per this item's own wording ("in trace/audit stores").
- [x] Role-based access control for REST API (JWT / AAD tokens): shipped in v0.7.0. `Authorization: Bearer <token>` gates every trace/audit/policy endpoint (`Viewer` for reads, `Admin` for writes); `AGC_JWT_SECRET` for shared-secret HS256, or `AGC_AAD_TENANT_ID`+`AGC_AAD_AUDIENCE` for Entra ID RS256 via JWKS. Opt-in: with neither set, RBAC stays off and every request is `Admin`, identical to this API's behavior before RBAC existed. The AAD/JWKS path is real-HTTP-tested against a mock server (fetch, `kid` lookup, RS256 verify all actually exercised) but not against a live Entra ID tenant (none was available while building this).
- [x] Microsoft Sentinel analytics rule export (Kusto query templates for AGC audit events): shipped in v0.8.0. `agc-cli sentinel export --table <name> --format kql|arm --output-dir <dir>` writes 4 built-in analytics rule templates (repeated policy blocks by one agent, a new agent's first action being a block, a portfolio-wide warn/alert volume spike, an agent triggering many distinct policies) either as one `.kql` file per rule or as a single ARM template (`Microsoft.SecurityInsights/alertRules`, kind `Scheduled`) deployable via `az deployment group create`. Table name defaults to `AGCAudit_CL` (what `scripts/azure_setup.sh` provisions) but is fully parameterized for a renamed table. Correct against the exact columns `azure_setup.sh`'s custom table declares, not verified against a live Sentinel workspace (none was available while building this), same disclosed-limitation pattern as the rest of this portfolio's Azure integrations.
- [x] Microsoft Sentinel analytics rule export (Kusto query templates for AGC audit events): shipped in v0.8.0. `agc-cli sentinel export --table <name> --format kql|arm --output-dir <dir>` writes 4 built-in analytics rule templates (repeated policy blocks by one agent, a new agent's first action being a block, a portfolio-wide warn/alert volume spike, an agent triggering many distinct policies) either as one `.kql` file per rule or as a single ARM template (`Microsoft.SecurityInsights/alertRules`, kind `Scheduled`) deployable via `az deployment group create`. Table name defaults to `AGCAudit_CL` (what `scripts/azure_setup.sh` provisions) but is fully parameterized for a renamed table. Correct against the exact columns `azure_setup.sh`'s custom table declares, not verified against a live Sentinel workspace (none was available while building this), the same disclosed-limitation pattern as the rest of this portfolio's Azure integrations.
- [x] Entra ID managed identity support for all Azure integrations (no client secrets): shipped in v0.9.0. Microsoft Graph and Azure Monitor Logs Ingestion (audit push) were already Managed-Identity-authenticated since v0.3.0; this closed the one remaining gap, the OTLP span exporter, which previously sent no authentication at all. `AGC_TELEMETRY_MANAGED_IDENTITY` (or `AGC_TELEMETRY_MANAGED_IDENTITY_CLIENT_ID` for a user-assigned identity) fetches a Managed Identity token scoped to `https://monitor.azure.com/.default` (the scope Azure Monitor's native OTLP endpoint requires) and attaches it as the export's `Authorization: Bearer` header. No client secret anywhere. A token fetch failure logs a warning and lets telemetry proceed unauthenticated rather than failing startup -- verified with a real end-to-end test against the real (off-Azure, intentionally unreachable) IMDS endpoint. Known limitation: the token is fetched once at startup and not refreshed for the life of the process -- a genuinely long-running deployment should plan for a restart (or a future refresh mechanism) before it expires.
- [x] Compliance report export aligned with Microsoft AI Responsible Use guidelines: shipped in v0.10.0. `GET /api/v1/compliance/report` (Markdown by default, `?format=json` for machine-readable) reports against 4 of [Microsoft's 6 Responsible AI principles](https://learn.microsoft.com/azure/machine-learning/concept-responsible-ai) using AGC's own tenant data: Accountability (policies enforced, decisions by policy), Transparency (every decision traceable, % matching an explicit rule), Reliability and Safety (span error rate, agents with 3+ repeated policy blocks), Privacy and Security (tenant isolation, RBAC status, OTLP Managed Identity status). Fairness and Inclusiveness are explicitly reported as out of scope in the report itself: they require observing an AI model's actual output content, which a governance/audit layer never collects -- see `docs/compliance.md`.
- [x] Dashboard UI: shipped in v0.11.0 as a self-contained static HTML/CSS/JS page (`GET /dashboard`), not the Tauri or WASM frontend this item's own wording named. Scoping decision: AGC is a REST API server the operator runs themselves (see README's "How it runs"), not a desktop or installed app, so a Tauri shell would add an entire second packaging/distribution story for no functional gain; a Rust/WASM frontend (Yew/Dioxus) would add a `wasm32-unknown-unknown` build toolchain and a compiled-asset pipeline to ship the same thing a 9KB vanilla-JS page already does against this REST API. The page itself needs no build step (embedded via `include_str!`, zero new dependencies) and covers health, tenant list, policy count, per-tenant span count, a paginated audit table, and the compliance report -- all client-side `fetch` calls against the existing REST endpoints, with an optional bearer-token field for RBAC-enabled deployments. See `docs/dashboard.md`.
- [x] Helm chart for Kubernetes deployment: shipped in v0.12.0. `helm/agent-governance-console` (Deployment, Service, optional Ingress/HPA/PVC/policy-ConfigMap, RBAC env wiring for both HMAC and Entra ID modes, Azure Workload Identity annotations) plus a new multi-stage `Dockerfile`. A real bug was found and fixed while building this: the server's default bind address (`127.0.0.1`) is unreachable through Docker's port mapping or a Kubernetes Service/probe -- fixed with a new `AGC_BIND` env var, defaulted to `0.0.0.0:8080` in the container image. The chart was verified for real: `helm lint`, `helm template` with every conditional path exercised (ingress, autoscaling, persistence, policy ConfigMap, both RBAC modes, Azure Workload Identity, extra env), `kubectl apply --dry-run=server` against a real k3s API server, and a genuine `helm install` into a local k3s cluster with the Docker-built image -- the pod reached `Ready`, its liveness/readiness probes passed against real `/health` checks, and the Kubernetes Service correctly routed traffic to it. See `docs/helm.md`.
- [x] SLA: p99 ingest latency < 10ms for 1K spans/s: verified in v1.0.0. New `agc-cli bench ingest` load-generation tool (evenly-spaced real HTTP requests, not a synchronous burst) plus `agc-core::bench` percentile math (6 unit tests). Measured p99 well under 1ms at 1000 req/s (roughly 20x margin), sustained over 20s, holding at 2x the target rate, and still only ~1.4ms in the realistic worst case (every span matching a real policy rule, every audit record actually persisted to a real SQLite file on disk). Two real things found and fixed while verifying this: a genuine server-side bottleneck (`AppState.tenants` was a `Mutex`, serializing every tenant lookup behind one exclusive lock even for already-existing tenants -- switched to `RwLock`), and a benchmark methodology bug (firing all 1000 req/s in one synchronous per-second burst measured artificial lock-contention queueing delay, not steady-state latency -- fixed by spacing requests evenly). Full investigation and exact numbers: `docs/performance.md`.

## Dual-Licensing Readiness

Assessed 2026-07-11 as a Dual-Licensing candidate (Community MIT + Commercial/Enterprise tier): governance/audit tooling for regulated environments is a plausible enterprise sales category, and AGC already targets that audience explicitly (see the README's "enterprise AI governance teams" framing). Not ready yet; blocked on:

- [x] ~~No authentication on the REST API at all~~ RBAC (HS256 or Entra ID) shipped in v0.7.0
- [x] ~~No multi-tenancy~~ shipped in v0.6.0
- [x] ~~No persistence yet~~ SQLite persistence for the audit log shipped in v0.2.0; trace store and policy engine are still in-memory only, so there is still no durable data for those two subsystems specifically

All three originally-listed technical blockers are now resolved as of
v1.0.0. This section is a factual technical-readiness record, not a
decision to pursue Dual-Licensing for this repo -- that remains a
separate business call, and the portfolio's standing decision (2026-07-13)
is to not expand Dual-Licensing to further Community repos.

## Out of Scope

- Model training or inference (governance of agent workflows only)
- Replacing a full observability platform (Datadog, Grafana) (AGC is a lightweight complement)
- Cloud-proprietary lock-in (Azure integration is opt-in, not a requirement)
