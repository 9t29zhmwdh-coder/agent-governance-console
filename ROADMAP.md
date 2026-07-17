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

## v0.3.0 : Azure Integration

- [ ] OTLP exporter to Azure Monitor (via `opentelemetry-otlp`)
- [ ] Azure Log Analytics DCR ingest endpoint for audit NDJSON
- [ ] Microsoft Graph integration (read agent app registrations)
- [ ] AAD managed identity authentication (no secrets in config)
- [ ] `scripts/azure_setup.sh` (create DCR, workspace, app registration)

## v0.4.0 : Policy DSL

- [ ] YAML-based policy DSL (see `docs/policy_dsl.md`)
- [ ] Hot-reload policies from file (watch via `notify`)
- [ ] Token budget enforcement (count from span attributes)
- [ ] OPA-compatible policy export (Rego stub)

## v1.0.0 : Enterprise GA

- [ ] Multi-tenant mode (tenant isolation in trace/audit stores)
- [ ] Role-based access control for REST API (JWT / AAD tokens)
- [ ] Microsoft Sentinel analytics rule export (Kusto query templates for AGC audit events)
- [ ] Entra ID managed identity support for all Azure integrations (no client secrets)
- [ ] Compliance report export aligned with Microsoft AI Responsible Use guidelines
- [ ] Dashboard UI (Tauri or WASM frontend)
- [ ] Helm chart for Kubernetes deployment
- [ ] SLA: p99 ingest latency < 10ms for 1K spans/s

## Dual-Licensing Readiness

Assessed 2026-07-11 as a Dual-Licensing candidate (Community MIT + Commercial/Enterprise tier): governance/audit tooling for regulated environments is a plausible enterprise sales category, and AGC already targets that audience explicitly (see the README's "enterprise AI governance teams" framing). Not ready yet; blocked on:

- [ ] No authentication on the REST API at all (v0.1 has none, AAD JWT is a v0.3.0/v1.0.0 item above): an Enterprise tier needs a real auth story before it can gate anything
- [ ] No multi-tenancy (v1.0.0 item above): a Commercial tier's core value is usually per-tenant isolation and RBAC
- [x] ~~No persistence yet~~ SQLite persistence for the audit log shipped in v0.2.0 (see above); trace store and policy engine are still in-memory only, so there is still no durable data across the whole system to license access to yet

Once v1.0.0's multi-tenant mode and RBAC land, revisit: candidate Enterprise-only features would be multi-tenant isolation, RBAC/SSO, Sentinel analytics export, and compliance report generation, with the core trace/policy/audit engine staying Community/MIT.

## Out of Scope

- Model training or inference (governance of agent workflows only)
- Replacing a full observability platform (Datadog, Grafana) (AGC is a lightweight complement)
- Cloud-proprietary lock-in (Azure integration is opt-in, not a requirement)
