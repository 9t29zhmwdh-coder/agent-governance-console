<div align="center">

<img src="RayStudio.png" alt="RayStudio" width="120" />

# Agent Governance Console

</div>

[🇩🇪 Deutsche Version](README.de.md)

**Governance, tracing, policy enforcement and observability for agentic workflows.**

A Rust workspace for tracing, policy enforcement and audit logging of AI agent activity, with real Azure Monitor telemetry/audit export, Microsoft Graph integration, and Microsoft Sentinel analytics rule export.

Aligned with [Microsoft's Responsible AI principles](https://learn.microsoft.com/en-us/azure/machine-learning/concept-responsible-ai) and designed for enterprise AI governance teams operating in regulated Microsoft cloud environments.

[![CI](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions/workflows/ci.yml/badge.svg)](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions) ![Azure Ready](https://img.shields.io/badge/Azure-Ready-0078d4?logo=microsoftazure&logoColor=white) ![Platform](https://img.shields.io/badge/Platform-Windows_%7C_Ubuntu-lightgrey) ![Rust](https://img.shields.io/badge/Rust-CE422B?logo=rust&logoColor=white) ![AI | Claude Code](https://img.shields.io/badge/AI-Claude_Code-black?logo=anthropic&logoColor=white) ![AI | Copilot](https://img.shields.io/badge/AI-Copilot-black?logo=github&logoColor=white) [![Release](https://img.shields.io/github/v/release/9t29zhmwdh-coder/agent-governance-console?color=3F8E7E)](https://github.com/9t29zhmwdh-coder/agent-governance-console/releases) [![License](https://img.shields.io/github/license/9t29zhmwdh-coder/agent-governance-console?color=lightgrey)](LICENSE)

> **How it runs:** AGC is not a hosted service and not a desktop app. `agc-api` is a small REST API server you run yourself with `cargo run`, on `127.0.0.1:8080` by default. There is no installer and nothing runs in the background; you start and stop the process yourself.

![Agent Governance Console](docs/screenshot.png)

---

> 🌱 New here? → [Step-by-step guide for beginners](GETTING_STARTED.md)

---

## Overview

Agent Governance Console (AGC) is an early-stage Rust workspace (`agc-core`, `agc-api`, `agc-cli`, `agc-azure`) for governing, observing and auditing AI agent workflows. The core library models trace spans, governance policies and audit records with a tested API; the REST API supports full trace ingestion with a real-time policy gate, policy loading, and paginated/streaming audit queries; and Azure integration (OTLP telemetry export, Managed-Identity-authenticated audit push to Azure Monitor, Microsoft Graph agent lookup) is real and wired in, not just planned (see [ROADMAP.md](ROADMAP.md)).

**In practice:** you can load a governance policy, POST agent trace spans against it, and have matching rules warn, block, or (recorded, not yet externally delivered) alert in real time, with every decision written to a queryable, exportable audit log that can be pushed to Azure Monitor. Trace and audit data are isolated per tenant (`X-Tenant-Id`, each with its own store); policies stay shared governance across every tenant. Optional RBAC (`Authorization: Bearer <JWT>`, HS256 or Entra ID) gates writes to an `Admin` role.

---

## Features

| Feature | Status |
|---------|--------|
| **Trace model** (`TraceSpan`, `TraceStore`) | Available: in-memory store, sorted ingestion, tested |
| **Audit model** (`AuditRecord`, `AuditLog`) | Available: SQLite-backed (in-memory by default, or a real per-tenant file via `AGC_AUDIT_DB_DIR`), NDJSON/CSV export, paginated query, tested and exposed via API |
| **Multi-tenant isolation** | Available: `X-Tenant-Id` header (required, no silent default) resolves an isolated trace+audit store per tenant, created lazily; `GET /api/v1/tenants` lists tenants seen so far. Policies stay global/shared. |
| **Policy model** (`GovernancePolicy`, `PolicyRule`) | Available: real condition evaluation (span level, token budget, operation glob), not just a data model |
| **Trace ingestion via API** | Available: `POST /api/v1/traces`, `GET /api/v1/traces/{trace_id}` |
| **Policy loading & real-time gating via API** | Available: `POST /api/v1/policies`; every ingested span is evaluated against loaded policies, `block` rules reject the span with `403` |
| **Audit query & export via API** | Available: `GET /api/v1/audit?limit=&offset=`, `GET /api/v1/audit/export.ndjson` / `.csv` |
| **REST API** | `/health`, `/api/v1/tenants`, `/api/v1/traces`, `/api/v1/traces/count`, `/api/v1/traces/{trace_id}`, `/api/v1/audit`, `/api/v1/audit/count`, `/api/v1/audit/export.ndjson`, `/api/v1/audit/export.csv`, `/api/v1/policies`, `/api/v1/policies/count` |
| **OTLP telemetry export to Azure Monitor** | Available: `AGC_TELEMETRY_ENDPOINT` wires a real OTLP/HTTP exporter into every ingested span |
| **Audit export to Azure Monitor (DCR)** | Available: `agc-cli azure push-audit`, Managed-Identity-authenticated, no client secret |
| **Microsoft Graph agent lookup** | Available: `agc-cli azure list-agents` (app registrations tagged `agc-agent`) |
| **YAML policy DSL** | Available: `GovernancePolicy::from_yaml` parses YAML or JSON (one parser, YAML is a JSON superset); `agc-cli policy validate` for offline checks |
| **Policy hot-reload** | Available: `AGC_POLICY_DIR` loads and live-reloads every policy file in a directory; a bad edit keeps the previous policy set instead of wiping it |
| **OPA/Rego export** | Available: `agc-cli policy to-rego` renders a structural Rego stub per policy — a hand-porting starting point, not a full semantic translation |
| **RBAC for REST API** | Available: `AGC_JWT_SECRET` (HS256) or `AGC_AAD_TENANT_ID` (Entra ID RS256) gates writes to `Admin`, reads need `Viewer`; opt-in, off by default |
| **Microsoft Sentinel export** | Available: `agc-cli sentinel export --format kql\|arm` generates 4 built-in analytics rule templates from AGC's audit table, as KQL files or a deployable ARM template — see [docs/sentinel.md](docs/sentinel.md) |

Full current vs. planned endpoint list: [docs/api_reference.md](docs/api_reference.md).

---

## Requirements

- Rust 1.78+
- Docker (optional, for containerised deployment)
- Azure subscription (optional, for OTLP telemetry export, audit push to Azure Monitor, and Microsoft Graph agent lookup — see [docs/azure_integration.md](docs/azure_integration.md))

---

## Quickstart

```bash
# Build all crates
cargo build --workspace

# Start API server (default: http://127.0.0.1:8080)
cargo run --bin agc-api

# Same, but persist each tenant's audit log to its own SQLite file
AGC_AUDIT_DB_DIR=./agc-audit cargo run --bin agc-api

# Health check
curl http://127.0.0.1:8080/health

# Counts (trace/audit endpoints require a tenant)
curl http://127.0.0.1:8080/api/v1/traces/count -H "X-Tenant-Id: tenant-a"
curl http://127.0.0.1:8080/api/v1/audit/count -H "X-Tenant-Id: tenant-a"
curl http://127.0.0.1:8080/api/v1/policies/count

# Run tests
cargo test --workspace
```

### Try the policy gate and multi-tenant isolation

```bash
# Load a policy that blocks anything at error level (global, applies to every tenant)
curl -X POST http://127.0.0.1:8080/api/v1/policies -H "content-type: application/json" -d '{
  "policy_id": "p1", "name": "Error gate", "agent_scope": [],
  "rules": [{"rule_id": "r1", "description": "Block on error",
    "condition": {"type": "span_level_at_least", "level": "error"},
    "action": {"type": "block", "reason": "too severe"}}]
}'

# This span is ingested into tenant-a's store, normally (201)
curl -X POST http://127.0.0.1:8080/api/v1/traces -H "content-type: application/json" -H "X-Tenant-Id: tenant-a" -d '{
  "span_id": "3fa85f64-5717-4562-b3fc-2c963f66afa6", "trace_id": "3fa85f64-5717-4562-b3fc-2c963f66afa7",
  "parent_span_id": null, "agent_id": "agent-1", "operation": "tool_call", "level": "info",
  "started_at": "2026-07-17T12:00:00Z", "ended_at": null, "attributes": {}
}'

# This one is rejected by the (global) policy gate (403), and never stored
curl -X POST http://127.0.0.1:8080/api/v1/traces -H "content-type: application/json" -H "X-Tenant-Id: tenant-a" -d '{
  "span_id": "3fa85f64-5717-4562-b3fc-2c963f66afa8", "trace_id": "3fa85f64-5717-4562-b3fc-2c963f66afa7",
  "parent_span_id": null, "agent_id": "agent-1", "operation": "risky_call", "level": "error",
  "started_at": "2026-07-17T12:00:01Z", "ended_at": null, "attributes": {}
}'

# tenant-b's store is untouched: real isolation, not a filtered view
curl http://127.0.0.1:8080/api/v1/traces/count -H "X-Tenant-Id: tenant-b"   # {"span_count":0,...}

# tenant-a's block decision is in tenant-a's audit log
curl http://127.0.0.1:8080/api/v1/audit -H "X-Tenant-Id: tenant-a"
curl http://127.0.0.1:8080/api/v1/audit/export.csv -H "X-Tenant-Id: tenant-a"

# Every tenant that has made at least one request so far
curl http://127.0.0.1:8080/api/v1/tenants
```

Full endpoint and policy schema reference: [docs/api_reference.md](docs/api_reference.md).

### Try the YAML policy DSL and hot-reload

```bash
mkdir -p ./policies
cat > ./policies/block-errors.yaml <<'EOF'
policy_id: p1
name: Error gate
agent_scope: []
rules:
  - rule_id: r1
    description: Block on error
    condition:
      type: span_level_at_least
      level: error
    action:
      type: block
      reason: too severe
EOF

# Validate a policy file offline, without a running server
cargo run --bin agc-cli -- policy validate ./policies/block-errors.yaml

# Start agc-api pointed at the directory: it loads every policy file at
# startup and reloads automatically whenever a file in it changes
AGC_POLICY_DIR=./policies cargo run --bin agc-api

# Render a structural Rego stub for a policy (see docs/policy_dsl.md for
# exactly what's a real translation vs. a hand-porting starting point)
cargo run --bin agc-cli -- policy to-rego ./policies/block-errors.yaml
```

### Try RBAC (optional)

```bash
# Enable HS256 JWT auth with a shared secret
AGC_JWT_SECRET=s3cret cargo run --bin agc-api

# No token: 401
curl -w "\nHTTP %{http_code}\n" http://127.0.0.1:8080/api/v1/traces/count -H "X-Tenant-Id: tenant-a"

# A viewer-role token can read but not write (403 on POST)
# An admin-role token can do both -- generate one with any HS256 JWT
# library using the same secret and a {"roles": ["admin"]} payload.
```

Or point `AGC_AAD_TENANT_ID` (+ optional `AGC_AAD_AUDIENCE`) at a real Entra ID tenant instead of a shared secret — see `docs/api_reference.md` for the full RBAC section.

### Try the Azure integration (optional)

```bash
# Provision the Azure resources once (needs an Azure subscription and az CLI)
AZURE_RG=my-rg AZURE_LOCATION=westeurope ./scripts/azure_setup.sh

# Export spans over OTLP to Azure Monitor as they're ingested
AGC_TELEMETRY_ENDPOINT="https://<region>.otelcollector.azure.com/v1/traces" cargo run --bin agc-api

# List Entra ID app registrations tagged 'agc-agent' (Managed Identity + Microsoft Graph)
cargo run --bin agc-cli -- azure list-agents

# Push a local audit export to an Azure Monitor DCR (Managed Identity, no client secret)
./scripts/export_audit.sh ndjson
cargo run --bin agc-cli -- azure push-audit --file audit-*.ndjson \
  --dce-endpoint "https://<name>.<region>-1.ingest.monitor.azure.com" \
  --dcr-id "dcr-..." --stream "Custom-AGCAudit_CL"
```

Full walkthrough, including what's mock-tested vs. verified against real Azure: [docs/azure_integration.md](docs/azure_integration.md).

---

## Uninstall / Cleanup

By default `agc-api` keeps everything in memory: stopping the process (Ctrl-C) removes all ingested data, there is nothing to clean up on disk. If you started it with `AGC_AUDIT_DB_DIR` set, each tenant's audit log persists in its own `{tenant_id}.sqlite` file in that directory; delete the directory to clear all audit history, or an individual file to clear just one tenant's. Delete the `target/` build directory to reclaim build cache space.

---

## Documentation

- [Architecture](ARCHITECTURE.md)
- [Azure Integration Guide](docs/azure_integration.md)
- [Trace Schema](docs/trace_schema.md)
- [Policy DSL Reference](docs/policy_dsl.md)
- [API Reference](docs/api_reference.md)
- [Privacy & Telemetry](PRIVACY.md)
- [Roadmap](ROADMAP.md)

---

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting. All policy decisions are logged immutably; audit records cannot be modified or deleted via the API.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

**Author:** [Rafael Yilmaz](https://github.com/9t29zhmwdh-coder) · **Status:** Early Release · ![version](https://img.shields.io/github/v/release/9t29zhmwdh-coder/agent-governance-console?color=6b7280&style=flat-square) · **License:** MIT
