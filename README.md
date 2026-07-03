<div align="center">

<img src="RayStudio.png" alt="RayStudio" width="80" />

# Agent Governance Console

**Governance, tracing, policy enforcement and observability for agentic workflows.**

Ingest execution traces, enforce governance policies and export audit records. First-class Azure Monitor and Microsoft Sentinel integration included.

Aligned with [Microsoft's Responsible AI principles](https://learn.microsoft.com/en-us/azure/machine-learning/concept-responsible-ai) and designed for enterprise AI governance teams operating in regulated Microsoft cloud environments.

[![CI](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions/workflows/ci.yml/badge.svg)](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions) ![Azure Ready](https://img.shields.io/badge/Azure-Ready-0078d4?logo=microsoftazure&logoColor=white) ![Platform](https://img.shields.io/badge/Platform-Windows_%7C_Ubuntu-lightgrey) ![Rust](https://img.shields.io/badge/Rust-CE422B?logo=rust&logoColor=white) ![AI | Claude Code](https://img.shields.io/badge/AI-Claude_Code-black?logo=anthropic&logoColor=white) ![AI | Copilot](https://img.shields.io/badge/AI-Copilot-black?logo=github&logoColor=white) [![Release](https://img.shields.io/github/v/release/9t29zhmwdh-coder/agent-governance-console?color=3F8E7E)](https://github.com/9t29zhmwdh-coder/agent-governance-console/releases) [![License](https://img.shields.io/github/license/9t29zhmwdh-coder/agent-governance-console?color=lightgrey)](LICENSE)

</div>

---

## Overview

Agent Governance Console (AGC) is an enterprise toolkit for governing, observing, and auditing AI agent workflows. It exposes a REST API for OpenTelemetry-compatible trace ingestion, evaluates governance policies against each span, and writes immutable audit records; all with opt-in export to Azure Monitor, Log Analytics, and Microsoft Sentinel.

---

## Features

| Feature | Description |
|---------|-------------|
| **Trace Ingestion** | OTLP-compatible span ingestion with in-memory and persistent storage |
| **Policy Engine** | Rule-based governance: warn, block, or alert on span conditions |
| **Audit Log** | Append-only records: NDJSON and CSV export for compliance |
| **Azure Monitor** | OTLP export to Azure Monitor Application Insights |
| **Microsoft Sentinel** | CEF / NDJSON audit export for Sentinel SIEM ingestion |
| **Entra ID Auth** | Bearer token validation via Microsoft Identity platform (configurable) |
| **REST API** | Axum-based HTTP API: `/ingest`, `/audit`, `/policies`, `/health` |
| **Opt-in Telemetry** | Disabled by default; activated via `OTLP_ENDPOINT` config |

---

## Quickstart

```bash
# Build all crates
cargo build --workspace

# Start API server (default: http://127.0.0.1:8080)
cargo run --bin agc-api

# Health check
curl http://127.0.0.1:8080/health

# Ingest a trace span
curl -X POST http://127.0.0.1:8080/ingest \
  -H "Content-Type: application/json" \
  -d '{"trace_id":"abc123","span_id":"s1","operation":"llm.invoke","duration_ms":312}'

# Run tests
cargo test --workspace
```

---

## Azure Integration

### Azure Monitor / Application Insights

Set the OTLP endpoint to stream all trace spans to Azure Monitor:

```bash
export OTLP_ENDPOINT="https://ingest.monitor.azure.com/v1/traces"
export AZURE_MONITOR_CONNECTION_STRING="InstrumentationKey=...;IngestionEndpoint=..."
cargo run --bin agc-api
```

Spans appear in Application Insights under **Custom Events → agc.span**.

### Microsoft Sentinel (SIEM)

Export the audit log in CEF format for Sentinel ingestion:

```bash
# Export as NDJSON — ingest via Log Analytics Data Collector API
curl http://127.0.0.1:8080/audit/export?format=ndjson > audit.ndjson

# Or stream via Azure Monitor Agent / Syslog forwarder
```

Query in Sentinel (KQL):
```kql
AgcAuditLog_CL
| where TimeGenerated > ago(24h)
| where policy_verdict_s == "block"
| summarize blocked = count() by agent_id_s
| order by blocked desc
```

### Entra ID (Azure AD) Authentication

Enable bearer token validation for production deployments:

```toml
# agc.toml
[auth]
provider = "entra"
tenant_id = "${AZURE_TENANT_ID}"
client_id = "${AGC_CLIENT_ID}"
```

Tokens are validated against the Microsoft Identity platform JWKS endpoint without any external auth library.

---

## Deployment

### Docker

```bash
docker build -t agc-api .
docker run -p 8080:8080 \
  -e OTLP_ENDPOINT="..." \
  -e AGC_POLICY_PATH="/etc/agc/policies.toml" \
  agc-api
```

### Azure Container Apps

```bash
az containerapp create \
  --name agc-api \
  --resource-group rg-governance \
  --image ghcr.io/9t29zhmwdh-coder/agc-api:latest \
  --ingress external --target-port 8080 \
  --env-vars OTLP_ENDPOINT=secretref:otlp-endpoint
```

---

## Documentation

- [Architecture](ARCHITECTURE.md)
- [Azure Integration Guide](docs/azure_integration.md)
- [Policy DSL Reference](docs/policy_dsl.md)
- [API Reference](docs/api_reference.md)
- [Privacy & Telemetry](PRIVACY.md)
- [Roadmap](ROADMAP.md)

---

## Requirements

- Rust 1.78+
- Docker (optional, for containerised deployment)
- Azure subscription (optional, for Monitor / Sentinel integration)

---

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting. All policy decisions are logged immutably; audit records cannot be modified or deleted via the API.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

**Author:** [Rafael Yilmaz](https://github.com/9t29zhmwdh-coder) · **Status:** Active · v0.1.0 · **License:** MIT
