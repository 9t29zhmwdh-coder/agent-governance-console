# Architecture: Agent Governance Console

## System Overview

AGC is a Rust workspace with four crates. The `agc-core` library contains all domain logic and has no Azure/HTTP dependencies. `agc-api` wraps it in an Axum REST server. `agc-cli` is a headless binary for scripting, debugging, and Azure operations. `agc-azure` holds all Azure integration (Managed Identity, Monitor Logs Ingestion, Microsoft Graph, OTLP export) behind its own crate boundary, so `agc-core` stays lean and Azure concerns don't leak into the domain model.

```
┌────────────────────────────────────────────────────────────────────┐
│                          agc-core                                    │
│                                                                       │
│  ┌───────────┐   ┌───────────┐   ┌───────────┐   ┌─────────────┐  │
│  │   trace   │   │  policy   │   │   audit   │   │  telemetry  │  │
│  │ TraceStore│   │PolicyEngine│   │ AuditLog  │   │TelemetryConfig│ │
│  └───────────┘   └───────────┘   └───────────┘   └─────────────┘  │
└────────────────────────────────────────────────────────────────────┘
         │ used by
         ▼
┌──────────────────┐        ┌──────────────────┐
│    agc-api       │───────▶│    agc-azure      │
│  Axum REST API   │  uses  │  ManagedIdentity   │
│  :8080           │        │  MonitorIngest     │
└──────────────────┘        │  Graph / Otlp      │
         ▲                  └─────────┬──────────┘
         │ HTTP                       │ HTTPS
         │                            ▼
┌──────────────────────┐   ┌───────────────────────────┐
│  agc-cli              │   │  Azure Monitor / Graph /  │
│  (also uses agc-azure)│   │  IMDS (docs/azure_...)    │
└──────────────────────┘   └───────────────────────────┘
         ▲
         │ HTTP
┌──────────────────────────────────────────────────────────┐
│  Agentic Workflow Runtime (external)                      │
│  SwiftAgent / LangChain / Semantic Kernel / custom agent  │
└──────────────────────────────────────────────────────────┘
```

## Module Responsibilities

### `trace`
- `TraceLevel`: Debug / Info / Warn / Error (ordered)
- `TraceSpan`: single execution span: agent ID, operation, timestamps, structured attributes
- `TraceStore`: sorted in-memory store; query by trace ID, filter errors

### `policy`
- `GovernancePolicy`: named policy with agent scope and rule list; `from_yaml`/`to_yaml` (YAML or JSON, one parser — YAML 1.2 is a JSON superset) and `to_rego_stub` (structural OPA export, see `docs/policy_dsl.md`)
- `PolicyRule`: condition + action pair
- `PolicyCondition`: `SpanLevelAtLeast`, `TokenBudgetExceeded`, `OperationMatches`
- `PolicyAction`: `Warn`, `Block`, `Alert`
- `PolicyEngine`: load policies (single, or `load_policies_from_dir` for a whole directory — a failed parse leaves the previous policy set untouched), resolve applicable rules per agent/operation

### `audit`
- `AuditRecord`: immutable record: agent, action, outcome, policy reference, details
- `AuditOutcome`: Allowed / Blocked / Warned / Alerted
- `AuditLog`: append-only log; NDJSON export (Azure Log Analytics), CSV export

### `sentinel`
- `SentinelRule`: name, description, severity, KQL query; `to_kql()`, `to_arm_resource()` (`Microsoft.SecurityInsights/alertRules`, kind `Scheduled`)
- `builtin_rules(table)`: 4 built-in governance-focused analytics rules, parameterized by the Log Analytics custom table name; exposed via `agc-cli sentinel export --format kql|arm`, see [docs/sentinel.md](docs/sentinel.md)

### `telemetry`
- `TelemetryConfig`: opt-in flag, OTLP endpoint, service name, agent-ID inclusion flag
- `TelemetrySink`: sync facade used by `agc-core` itself (debug-logs when enabled); `agc-api` additionally wires a real `agc_azure::OtlpExporter` into `AppState` when telemetry is configured (see below), since a real async HTTP exporter doesn't belong in a dependency-light sync core library

### `agc-azure` (separate crate)
- `ManagedIdentityCredential`: AAD tokens via IMDS (system- or user-assigned), 2s client timeout
- `MonitorIngestClient`: pushes `AuditRecord`s to an Azure Monitor DCR (Logs Ingestion API)
- `GraphClient`: lists Entra ID app registrations tagged `agc-agent`
- `OtlpExporter`: real OTLP/HTTP span export, batch processor (own background thread, never blocks the caller)

## Data Flow: Trace Ingestion + Policy Evaluation

```
Agent Runtime
     │
     │  POST /api/v1/traces  (TraceSpan JSON, X-Tenant-Id header)
     ▼
  agc-api (ingest_trace handler)
     │
     ├── TenantId extractor            — 400 if X-Tenant-Id is missing/empty,
     │       │                            no silent "default tenant"
     │       ▼
     ├── AppState::tenant_store(id)    — lazily creates this tenant's
     │       │                            TraceStore + AuditLog (own SQLite
     │       │                            file too, if AGC_AUDIT_DB_DIR set)
     │       ▼
     ├── PolicyEngine::evaluate(&span) — global, shared across all tenants;
     │       │       matched rules       real condition evaluation, not
     │       │                           just scope filtering
     │       ▼
     │   AuditLog::append(...)         — into THIS TENANT's audit log
     │       │
     │       ▼
     │   any rule Block? ──▶ yes ──▶ 403, span is NOT stored, stop here
     │       │ no
     │       ▼
     ├── TraceStore::ingest(span)      — into THIS TENANT's trace store, 201 Created
     │
     └── AppState.otlp.record_span(operation, duration_ms)
              │ only if AGC_TELEMETRY_ENDPOINT is configured (not tenant-scoped)
              ▼
          agc_azure::OtlpExporter ──HTTP──▶ Azure Monitor / OTLP collector
```

## Multi-Tenancy

`AppState` holds tenant stores behind its own lock, separate from the
global `PolicyEngine` lock, so one tenant's traffic never serializes
behind another's:

```rust
pub struct AppState {
    tenants: Arc<Mutex<HashMap<String, Arc<TenantStore>>>>, // per-tenant
    pub policy: Arc<Mutex<PolicyEngine>>,                    // global
    pub otlp: Option<Arc<agc_azure::OtlpExporter>>,          // global
    audit_db_dir: Option<PathBuf>,
    pub auth: AuthConfig, // global, see "RBAC" below
}

pub struct TenantStore {
    pub traces: Mutex<TraceStore>,
    pub audit: Mutex<AuditLog>,
}
```

Every trace/audit endpoint requires `X-Tenant-Id`; there is deliberately
no default-tenant fallback (see `TenantId`'s `FromRequestParts` impl),
so isolation can't be silently bypassed by a client forgetting the
header. `GET /api/v1/tenants` lists every tenant ID seen so far.

## RBAC (`agc-api::auth`)

Each tenant/policy route closure checks `auth::authorize(&state.auth,
&headers, min_role)` before calling its handler, returning the
`401`/`403` response early if it fails. `AuthConfig` has three variants:
`Disabled` (default, every request is `Admin`), `Hmac` (HS256, shared
secret), `Aad` (RS256, Entra ID JWKS, `kid`-based key selection, JWKS
cached in-process after first fetch). See `docs/api_reference.md` for the
full contract and what's mock-tested vs. live-Entra-ID-tested.

## REST API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Service health (no tenant) |
| GET | `/api/v1/tenants` | List tenants seen so far (no tenant header needed) |
| POST | `/api/v1/traces` | Ingest a span, policy-gated (tenant-scoped) |
| GET | `/api/v1/traces/count` | Total span count (tenant-scoped) |
| GET | `/api/v1/traces/{trace_id}` | Full trace by ID (tenant-scoped) |
| POST | `/api/v1/policies` | Load a governance policy (JSON, global) |
| GET | `/api/v1/policies/count` | Total loaded policy count (global) |
| GET | `/api/v1/audit` | Paginated audit query (tenant-scoped) |
| GET | `/api/v1/audit/count` | Total audit record count (tenant-scoped) |
| GET | `/api/v1/audit/export.ndjson` / `.csv` | Streaming audit export (tenant-scoped) |

Full request/response schemas: `docs/api_reference.md`.

## Policy Loading Paths

Three independent ways to get a policy into a running `PolicyEngine`,
all converging on the same `GovernancePolicy::from_yaml` parser (YAML or
JSON, one parser):

```
POST /api/v1/policies (JSON)  ──┐
AGC_POLICY_DIR file (YAML/JSON) ─┼──▶ PolicyEngine
agc-cli policy validate (offline, doesn't touch a running engine) ──┘
```

`AGC_POLICY_DIR` additionally spawns a `notify`-based filesystem
watcher (`agc_api::spawn_policy_hot_reload`): its callback runs on its
own OS thread outside the Tokio runtime, so it only sends a signal over
a channel to a dedicated async task that does the actual (async-locked)
reload — the same "sync callback, async reload" split as the OTLP batch
processor above avoids the same class of deadlock. A parse error during
a directory reload aborts that reload and keeps the previous policy set,
so a bad edit to one file can't silently wipe a working configuration.

## Azure Integration

See `docs/azure_integration.md` for OTLP, Microsoft Graph and Log Analytics DCR setup, and what's mock-tested vs. verified against a real Azure subscription.

## External Dependencies

| Crate | Purpose |
|-------|---------|
| `axum` | REST API framework |
| `tokio` | Async runtime |
| `serde` / `serde_json` | Serialisation |
| `chrono` | Timestamp handling |
| `uuid` | Span and audit record IDs |
| `tracing` / `tracing-subscriber` | Structured logging |
| `tower-http` | CORS, tracing middleware |
| `reqwest` | HTTP client (`agc-azure`: IMDS, Monitor, Graph) |
| `opentelemetry` / `opentelemetry_sdk` / `opentelemetry-otlp` | Real OTLP span export (`agc-azure`) |
| `clap` | CLI argument parsing (`agc-cli`) |
| `serde_norway` | YAML (de)serialization for the policy DSL (`agc-core`) |
| `notify` | Filesystem watching for policy hot-reload (`agc-api`) |
| `wiremock` (dev) | Mock HTTP server for `agc-azure`/`agc-api` Azure integration tests |
