# Architecture — Agent Governance Console

## System Overview

AGC is a Rust workspace with three crates. The `agc-core` library contains all domain logic. `agc-api` wraps it in an Axum REST server. `agc-cli` is a headless binary for scripting and debugging.

```
┌────────────────────────────────────────────────────────────────────┐
│                          agc-core                                    │
│                                                                       │
│  ┌───────────┐   ┌───────────┐   ┌───────────┐   ┌─────────────┐  │
│  │   trace   │──▶│  policy   │──▶│   audit   │   │  telemetry  │  │
│  │ TraceStore│   │PolicyEngine│   │ AuditLog  │   │TelemetrySink│  │
│  └───────────┘   └───────────┘   └───────────┘   └─────────────┘  │
└────────────────────────────────────────────────────────────────────┘
         │ in-process                     │ opt-in OTLP export
         ▼                                ▼
┌──────────────────┐          ┌──────────────────────────┐
│    agc-api       │          │  Azure Monitor / OTLP    │
│  Axum REST API   │          │  (docs/azure_integration) │
│  :8080           │          └──────────────────────────┘
└──────────────────┘
         │ HTTP
         ▼
┌──────────────────────────────────────────────────────────┐
│  Agentic Workflow Runtime (external)                      │
│  SwiftAgent / LangChain / Semantic Kernel / custom agent  │
└──────────────────────────────────────────────────────────┘
```

## Module Responsibilities

### `trace`
- `TraceLevel` — Debug / Info / Warn / Error (ordered)
- `TraceSpan` — single execution span: agent ID, operation, timestamps, structured attributes
- `TraceStore` — sorted in-memory store; query by trace ID, filter errors

### `policy`
- `GovernancePolicy` — named policy with agent scope and rule list
- `PolicyRule` — condition + action pair
- `PolicyCondition` — `SpanLevelAtLeast`, `TokenBudgetExceeded`, `OperationMatches`
- `PolicyAction` — `Warn`, `Block`, `Alert`
- `PolicyEngine` — load policies, resolve applicable rules per agent/operation

### `audit`
- `AuditRecord` — immutable record: agent, action, outcome, policy reference, details
- `AuditOutcome` — Allowed / Blocked / Warned / Alerted
- `AuditLog` — append-only log; NDJSON export (Azure Log Analytics), CSV export

### `telemetry`
- `TelemetryConfig` — opt-in flag, OTLP endpoint, service name, agent-ID inclusion flag
- `TelemetrySink` — routes to real OTLP exporter or `NoopTelemetry`

## Data Flow — Trace Ingestion + Policy Evaluation

```
Agent Runtime
     │
     │  POST /api/v1/traces  (TraceSpan JSON)
     ▼
  agc-api
     │
     ├── TraceStore::ingest(span)
     │
     ├── PolicyEngine::applicable_rules(agent_id, operation)
     │       │ rules found
     │       ▼
     │   evaluate conditions against span
     │       │ rule fires
     │       ▼
     │   AuditLog::append(AuditRecord { outcome: Blocked | Warned | Alerted })
     │
     └── TelemetrySink::record_span(operation, duration_ms)
              │ if enabled
              ▼
          Azure Monitor OTLP endpoint
```

## REST API Endpoints (v0.1 stub)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Service health |
| GET | `/api/v1/traces/count` | Total span count |
| GET | `/api/v1/audit/count` | Total audit record count |

Full CRUD endpoints planned for v0.2.0 (see ROADMAP.md).

## Azure Integration

See `docs/azure_integration.md` for OTLP, Microsoft Graph and Log Analytics setup.

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
