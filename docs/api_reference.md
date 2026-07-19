# API Reference: agc-api

## Base URL

`http://127.0.0.1:8080` (default; configurable via `ConsoleConfig.api_bind`)

## Endpoints (v0.1.0)

### GET /health

Returns service health and version.

```json
{ "status": "ok", "version": "0.1.0" }
```

### GET /api/v1/traces/count

Returns total ingested span count.

```json
{ "span_count": 42 }
```

### GET /api/v1/audit/count

Returns total audit record count.

```json
{ "record_count": 7 }
```

---

## Planned Endpoints (v0.2.0+)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/traces` | Ingest a `TraceSpan` |
| GET | `/api/v1/traces/{trace_id}` | Retrieve spans for a trace |
| GET | `/api/v1/traces?agent_id=X` | Filter spans by agent |
| POST | `/api/v1/policies` | Load a `GovernancePolicy` |
| GET | `/api/v1/policies` | List loaded policies |
| GET | `/api/v1/audit?page=0&size=100` | Paginated audit log |
| GET | `/api/v1/audit/export.ndjson` | Stream full audit as NDJSON |
| GET | `/api/v1/audit/export.csv` | Stream full audit as CSV |

## Rust Types

### TraceStore

```rust
pub struct TraceStore { ... }
impl TraceStore {
    pub fn ingest(&mut self, span: TraceSpan)
    pub fn spans_for_trace(&self, trace_id: &Uuid) -> Vec<&TraceSpan>
    pub fn error_spans(&self) -> Vec<&TraceSpan>
    pub fn span_count(&self) -> usize
}
```

### AuditLog

```rust
pub struct AuditLog { ... }
impl AuditLog {
    pub fn append(&mut self, record: AuditRecord)
    pub fn export_ndjson(&self) -> String
    pub fn export_csv(&self) -> String
    pub fn records_for_agent(&self, agent_id: &str) -> Vec<&AuditRecord>
    pub fn blocked_records(&self) -> Vec<&AuditRecord>
    pub fn record_count(&self) -> usize
}
```

### PolicyEngine

```rust
pub struct PolicyEngine { ... }
impl PolicyEngine {
    pub fn load_policy(&mut self, policy: GovernancePolicy)
    pub fn policy_count(&self) -> usize
    pub fn applicable_rules(&self, agent_id: &str, operation: &str) -> Vec<&PolicyRule>
}
```
