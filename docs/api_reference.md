# API Reference: agc-api

## Base URL

`http://127.0.0.1:8080` (default; configurable via `ConsoleConfig.api_bind`)

## Multi-Tenancy

Every trace and audit endpoint requires a `X-Tenant-Id` header. There is
no "default tenant" fallback: a missing or empty header is rejected with
`400`, so tenant isolation can't be silently bypassed by forgetting the
header. Each distinct tenant ID gets its own `TraceStore` and `AuditLog`,
created lazily on that tenant's first request; with `AGC_AUDIT_DB_DIR`
set, each tenant's audit log also gets its own SQLite file
(`{dir}/{tenant_id}.sqlite`).

**Policies are not tenant-scoped.** A policy loaded via `POST
/api/v1/policies` is shared governance that gates every tenant's
ingestion; this is deliberate (see `ROADMAP.md`), not a gap.
ingestion: this is deliberate (see `ROADMAP.md`), not a gap.

### GET /api/v1/tenants

Lists every tenant ID that has made at least one request so far
(sorted). No `X-Tenant-Id` header needed for this one endpoint.

```json
{ "tenants": ["tenant-a", "tenant-b"] }
```

## RBAC (opt-in)

With `AGC_JWT_SECRET` or `AGC_AAD_TENANT_ID` set, every trace/audit/policy
endpoint requires `Authorization: Bearer <token>`:

- **Reads** (`GET` endpoints) need at least the `Viewer` role.
- **Writes** (`POST /api/v1/traces`, `POST /api/v1/policies`) need the `Admin` role.
- Missing or malformed header: `401`. Valid token, insufficient role: `403`.

With neither env var set, RBAC is off and every request is treated as
`Admin`, identical to this API's behavior before RBAC existed.

### Modes

| Mode | Env vars | Algorithm | Role claim |
|------|----------|-----------|------------|
| HS256 (shared secret) | `AGC_JWT_SECRET` | HS256 | `roles` array in the JWT payload, e.g. `{"roles": ["admin"]}` |
| Entra ID (AAD) | `AGC_AAD_TENANT_ID`, `AGC_AAD_AUDIENCE` (default `api://agc`) | RS256, verified against the tenant's JWKS (`https://login.microsoftonline.com/{tenant}/discovery/v2.0/keys`) | `roles` claim on the token (app roles from a client-credentials/app-only token; group-based roles would need a separate Graph call this crate doesn't make) |

Unrecognized role strings never grant `Admin` (fail-safe to `Viewer`).
`exp`, if present, is enforced (an expired token is rejected); it's not
required to be present at all, so short-lived and long-lived tokens both
work.

Responses:

```json
// 401
{ "error": "unauthorized", "reason": "missing or malformed Authorization: Bearer <token> header" }
// 403
{ "error": "forbidden", "reason": "requires at least Admin role" }
```

**What's verified vs. not**: the HS256 path is tested against real signed
tokens. The AAD/JWKS path's fetch-`kid`-lookup-RS256-verify pipeline is
tested against a real mock JWKS server with a real RSA-signed token, but
has not been exercised against a live Entra ID tenant (none was
available while building this), same disclosed limitation as
available while building this), the same disclosed limitation as
`agc_azure::ManagedIdentityCredential`, see `docs/azure_integration.md`.

## Endpoints

### GET /health

Returns service health and version. No tenant header needed.

```json
{ "status": "ok", "version": "1.0.1" }
```

### GET /api/v1/traces/count

Returns the ingested span count for the tenant in `X-Tenant-Id`.

```json
{ "tenant_id": "tenant-a", "span_count": 42 }
```

### POST /api/v1/traces

Ingests a `TraceSpan` (JSON body) into the `X-Tenant-Id` tenant's trace
store. Every in-scope policy rule (global, not tenant-scoped) is
evaluated against the span first: each matched rule writes one
`AuditRecord` to that tenant's audit log, and if any matched rule's
action is `block`, the span is rejected with `403` and never stored.

```json
// Request body
{
  "span_id": "...", "trace_id": "...", "parent_span_id": null,
  "agent_id": "agent-1", "operation": "tool_call", "level": "info",
  "started_at": "2026-07-17T12:00:00Z", "ended_at": null,
  "attributes": { "tokens": 512 }
}
```

Success (`201`):

```json
{ "tenant_id": "tenant-a", "span_id": "...", "trace_id": "...", "policy_events": 1 }
```

Blocked (`403`):

```json
{ "error": "blocked_by_policy", "rule_id": "r1", "reason": "too severe" }
```

Missing tenant header (`400`):

```json
{ "error": "missing_tenant_id", "reason": "the X-Tenant-Id header is required on this endpoint" }
```

### GET /api/v1/traces/{trace_id}

Retrieves every span for a trace ID within the `X-Tenant-Id` tenant's
store. `404` if none are found (including if the trace exists, but under
a different tenant).

```json
{ "tenant_id": "tenant-a", "trace_id": "...", "spans": [ { "span_id": "...", "...": "..." } ] }
```

### GET /api/v1/audit/count

Returns the audit record count for the tenant in `X-Tenant-Id`.

```json
{ "tenant_id": "tenant-a", "record_count": 7 }
```

### GET /api/v1/audit?limit=50&offset=0

Paginated audit log query for the `X-Tenant-Id` tenant, ordered
oldest-first. `limit` is clamped to `[1, 500]` and defaults to `50`;
`offset` defaults to `0`.

```json
{ "tenant_id": "tenant-a", "total": 132, "limit": 50, "offset": 0, "records": [ { "...": "..." } ] }
```

### GET /api/v1/audit/export.ndjson

Streams the `X-Tenant-Id` tenant's full audit log as newline-delimited
JSON (`content-type: application/x-ndjson`), one `AuditRecord` per line.

### GET /api/v1/audit/export.csv

Streams the `X-Tenant-Id` tenant's full audit log as CSV
(`content-type: text/csv`), header row
`id,timestamp,agent_id,action,outcome,policy_id`.

### GET /api/v1/compliance/report

Responsible-AI-aligned compliance report for the `X-Tenant-Id` tenant.
Markdown by default (`content-type: text/markdown`); `?format=json` for
the same data as JSON. See [docs/compliance.md](compliance.md) for what
each section covers and what's explicitly out of scope.

### GET /api/v1/policies/count

Returns total loaded policy count (global, no tenant header needed).

```json
{ "policy_count": 3 }
```

### POST /api/v1/policies

Loads a `GovernancePolicy` (JSON body) into the running engine. Global:
no tenant header needed, and the policy gates every tenant's ingestion.

```json
{
  "policy_id": "p1", "name": "Error gate", "agent_scope": [],
  "rules": [{
    "rule_id": "r1", "description": "Block on error",
    "condition": { "type": "span_level_at_least", "level": "error" },
    "action": { "type": "block", "reason": "too severe" }
  }]
}
```

Response (`201`): `{ "policy_id": "p1", "loaded": true }`

#### Policy condition types

| `type` | Fields | Matches when |
|--------|--------|--------------|
| `span_level_at_least` | `level` (`debug`\|`info`\|`warn`\|`error`) | span's level is at or above the threshold |
| `token_budget_exceeded` | `max_tokens` (u64) | `attributes.tokens` (if present) exceeds `max_tokens` |
| `operation_matches` | `pattern` (string, `*` wildcard) | span's `operation` matches the pattern, e.g. `tool_*` |

#### Policy action types

| `type` | Fields | Effect |
|--------|--------|--------|
| `warn` | `message` | Audit record with outcome `warned`; span still ingested |
| `block` | `reason` | Audit record with outcome `blocked`; span rejected with `403`, never stored |
| `alert` | `channel` | Audit record with outcome `alerted`; span still ingested (external delivery is a v0.3.0 Azure Monitor item, not implemented yet) |

---

## Planned Endpoints (later milestones)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/traces?agent_id=X` | Filter spans by agent |
| GET | `/api/v1/policies` | List loaded policies |
| * | (RBAC-gated versions of the above) | JWT/AAD auth for the REST API itself, see ROADMAP.md |

See [ROADMAP.md](../ROADMAP.md) for the full v1.0.0 Enterprise GA plan (RBAC, Sentinel export, Dashboard UI, Helm chart). Sentinel export itself (`agc-cli sentinel export`, not a REST endpoint) is documented in [docs/sentinel.md](sentinel.md).

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
    pub fn list_paginated(&self, limit: usize, offset: usize) -> (Vec<AuditRecord>, usize)
    pub fn records_for_agent(&self, agent_id: &str) -> Vec<AuditRecord>
    pub fn blocked_records(&self) -> Vec<AuditRecord>
    pub fn record_count(&self) -> usize
}
```

### PolicyEngine

```rust
pub struct PolicyEngine { ... }
impl PolicyEngine {
    pub fn load_policy(&mut self, policy: GovernancePolicy)
    pub fn load_policies_from_dir(&mut self, dir: &Path) -> Result<usize, PolicyError>
    pub fn policy_count(&self) -> usize
    pub fn applicable_rules(&self, agent_id: &str, operation: &str) -> Vec<&PolicyRule>
    /// Real-time gate: rules whose condition actually matches the span.
    pub fn evaluate(&self, span: &TraceSpan) -> Vec<(String, PolicyRule)>
}
```

### TenantStore (`agc-api`)

```rust
pub struct TenantStore {
    pub traces: Mutex<TraceStore>,
    pub audit: Mutex<AuditLog>,
}
```

One instance per tenant, held in `AppState`'s internal
`HashMap<String, Arc<TenantStore>>`, resolved by the `X-Tenant-Id`
header via the `TenantId` extractor.
