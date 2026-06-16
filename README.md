<div align="center">

<img src="RayStudio.png" alt="RayStudio" width="80" />

# Agent Governance Console

**Governance, tracing, policy enforcement and observability for agentic workflows.**

Ingest execution traces, apply governance policies, export audit records — with optional Azure Monitor integration.

[![CI](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions/workflows/ci.yml/badge.svg)](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: Linux / Windows / macOS](https://img.shields.io/badge/Platform-Linux%20%7C%20Windows%20%7C%20macOS-blue)](#)

</div>

---

## Overview

Agent Governance Console (AGC) is an enterprise toolkit for governing, observing and auditing AI agent workflows. It provides a REST API for trace ingestion, a governance policy engine, an append-only audit log, and opt-in Azure Monitor / OTLP telemetry.

## Features

| Feature | Description |
|---------|-------------|
| **Trace Ingestion** | OpenTelemetry-compatible span ingestion and in-memory store |
| **Policy Engine** | Rule-based governance: warn, block or alert on span conditions |
| **Audit Log** | Append-only records with NDJSON and CSV export |
| **Azure Integration** | Hints for Azure Monitor (OTLP), Microsoft Graph, Log Analytics |
| **Opt-in Telemetry** | Disabled by default; configure OTLP endpoint to enable |
| **REST API** | Axum-based HTTP API for trace and audit queries |

## Quickstart

```bash
# Build
cargo build --workspace

# Start API server (default: http://127.0.0.1:8080)
cargo run --bin agc-api

# Health check
curl http://127.0.0.1:8080/health

# Run tests
cargo test --workspace
```

## Documentation

- [Architecture](ARCHITECTURE.md)
- [Azure Integration Guide](docs/azure_integration.md)
- [Trace Schema](docs/trace_schema.md)
- [Policy DSL Reference](docs/policy_dsl.md)
- [API Reference](docs/api_reference.md)
- [Roadmap](ROADMAP.md)
- [Privacy Policy](PRIVACY.md)

## Requirements

- Rust 1.78+
- Docker (optional — for containerised deployment)
- Azure Monitor / OTLP endpoint (optional — for telemetry)

## Security

See [SECURITY.md](SECURITY.md) for vulnerability reporting.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

<div align="center">

**RayStudio · Rafael Yilmaz · MIT License · 2026**

</div>
