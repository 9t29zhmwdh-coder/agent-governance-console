# Roadmap — Agent Governance Console

## v0.1.0 — Initial Import ✅

- Rust workspace: `agc-core`, `agc-api` (Axum), `agc-cli`
- Trace ingestion: `TraceSpan`, `TraceStore`, sorted by `started_at`
- Policy engine: `GovernancePolicy`, `PolicyRule`, condition/action stubs
- Audit log: `AuditRecord`, NDJSON + CSV export
- Opt-in telemetry: `TelemetryConfig`, `TelemetrySink`, OTLP-ready
- REST API: `/health`, `/api/v1/traces/count`, `/api/v1/audit/count`
- 6 unit tests covering all core subsystems
- Bilingual README (EN / DE)
- Full documentation skeleton

## v0.2.0 — Full REST API

- [ ] POST `/api/v1/traces` — ingest span
- [ ] GET `/api/v1/traces/{trace_id}` — retrieve full trace
- [ ] POST `/api/v1/policies` — load policy JSON
- [ ] GET `/api/v1/audit` — paginated audit log query
- [ ] GET `/api/v1/audit/export.csv` and `/export.ndjson` — streaming export
- [ ] Policy evaluation on every ingested span (real-time gate)
- [ ] SQLite persistence for audit log (`rusqlite`)

## v0.3.0 — Azure Integration

- [ ] OTLP exporter to Azure Monitor (via `opentelemetry-otlp`)
- [ ] Azure Log Analytics DCR ingest endpoint for audit NDJSON
- [ ] Microsoft Graph integration (read agent app registrations)
- [ ] AAD managed identity authentication (no secrets in config)
- [ ] `scripts/azure_setup.sh` — create DCR, workspace, app registration

## v0.4.0 — Policy DSL

- [ ] YAML-based policy DSL (see `docs/policy_dsl.md`)
- [ ] Hot-reload policies from file (watch via `notify`)
- [ ] Token budget enforcement (count from span attributes)
- [ ] OPA-compatible policy export (Rego stub)

## v1.0.0 — Enterprise GA

- [ ] Multi-tenant mode (tenant isolation in trace/audit stores)
- [ ] Role-based access control for REST API (JWT / AAD tokens)
- [ ] Dashboard UI (Tauri or WASM frontend)
- [ ] Helm chart for Kubernetes deployment
- [ ] SLA: p99 ingest latency < 10ms for 1K spans/s

## Out of Scope

- Model training or inference — governance of agent workflows only
- Replacing a full observability platform (Datadog, Grafana) — AGC is a lightweight complement
- Cloud-proprietary lock-in — Azure integration is opt-in, not a requirement
