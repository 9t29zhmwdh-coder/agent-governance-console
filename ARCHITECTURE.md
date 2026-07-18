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
- `GovernancePolicy`: named policy with agent scope and rule list
- `PolicyRule`: condition + action pair
- `PolicyCondition`: `SpanLevelAtLeast`, `TokenBudgetExceeded`, `OperationMatches`
- `PolicyAction`: `Warn`, `Block`, `Alert`
- `PolicyEngine`: load policies, resolve applicable rules per agent/operation

### `audit`
- `AuditRecord`: immutable record: agent, action, outcome, policy reference, details
- `AuditOutcome`: Allowed / Blocked / Warned / Alerted
- `AuditLog`: append-only log; NDJSON export (Azure Log Analytics), CSV export

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
     │  POST /api/v1/traces  (TraceSpan JSON)
     ▼
  agc-api (ingest_trace handler)
     │
     ├── PolicyEngine::evaluate(&span)          — real condition evaluation,
     │       │ matched rules                       not just scope filtering
     │       ▼
     │   AuditLog::append(AuditRecord { outcome: Blocked | Warned | Alerted })
     │       │
     │       ▼
     │   any rule Block? ──▶ yes ──▶ 403, span is NOT stored, stop here
     │       │ no
     │       ▼
     ├── TraceStore::ingest(span)                — 201 Created
     │
     └── AppState.otlp.record_span(operation, duration_ms)
              │ only if AGC_TELEMETRY_ENDPOINT is configured
              ▼
          agc_azure::OtlpExporter ──HTTP──▶ Azure Monitor / OTLP collector
```

## REST API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Service health |
| POST | `/api/v1/traces` | Ingest a span, policy-gated |
| GET | `/api/v1/traces/count` | Total span count |
| GET | `/api/v1/traces/{trace_id}` | Full trace by ID |
| POST | `/api/v1/policies` | Load a governance policy |
| GET | `/api/v1/policies/count` | Total loaded policy count |
| GET | `/api/v1/audit` | Paginated audit query |
| GET | `/api/v1/audit/count` | Total audit record count |
| GET | `/api/v1/audit/export.ndjson` / `.csv` | Streaming audit export |

Full request/response schemas: `docs/api_reference.md`.

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
| `wiremock` (dev) | Mock HTTP server for `agc-azure`/`agc-api` Azure integration tests |
