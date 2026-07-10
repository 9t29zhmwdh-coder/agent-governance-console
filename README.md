<div align="center">

<img src="RayStudio.png" alt="RayStudio" width="120" />

# Agent Governance Console

[🇩🇪 Deutsche Version](README.de.md)

**Governance, tracing, policy enforcement and observability for agentic workflows.**

A Rust workspace laying the groundwork for tracing, policy enforcement and audit logging of AI agent activity, with Azure Monitor and Microsoft Sentinel integration on the roadmap.

Aligned with [Microsoft's Responsible AI principles](https://learn.microsoft.com/en-us/azure/machine-learning/concept-responsible-ai) and designed for enterprise AI governance teams operating in regulated Microsoft cloud environments.

[![CI](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions/workflows/ci.yml/badge.svg)](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions) ![Azure Ready](https://img.shields.io/badge/Azure-Ready-0078d4?logo=microsoftazure&logoColor=white) ![Platform](https://img.shields.io/badge/Platform-Windows_%7C_Ubuntu-lightgrey) ![Rust](https://img.shields.io/badge/Rust-CE422B?logo=rust&logoColor=white) ![AI | Claude Code](https://img.shields.io/badge/AI-Claude_Code-black?logo=anthropic&logoColor=white) ![AI | Copilot](https://img.shields.io/badge/AI-Copilot-black?logo=github&logoColor=white) [![Release](https://img.shields.io/github/v/release/9t29zhmwdh-coder/agent-governance-console?color=3F8E7E)](https://github.com/9t29zhmwdh-coder/agent-governance-console/releases) [![License](https://img.shields.io/github/license/9t29zhmwdh-coder/agent-governance-console?color=lightgrey)](LICENSE)

</div>

> **How it runs:** AGC is not a hosted service and not a desktop app. `agc-api` is a small REST API server you run yourself with `cargo run`, on `127.0.0.1:8080` by default. There is no installer and nothing runs in the background; you start and stop the process yourself.

![Agent Governance Console](docs/screenshot.png)

---

> 🌱 New here? → [Step-by-step guide for beginners](GETTING_STARTED.md)

---

## Overview

Agent Governance Console (AGC) is an early-stage Rust workspace (`agc-core`, `agc-api`, `agc-cli`) for governing, observing and auditing AI agent workflows. The core library already models trace spans, governance policies and audit records with a tested API; the REST API currently exposes read-only health and count endpoints, with ingestion, policy loading and audit export planned for v0.2.0 (see [ROADMAP.md](ROADMAP.md)). Azure Monitor, Microsoft Sentinel and Entra ID integration are planned for v0.3.0+ and not implemented yet.

**In practice:** today you get a tested Rust library for modeling agent traces, policies and audit records, plus a REST API that reports how many of each have been loaded. It is a foundation to build on, not yet a drop-in governance layer for production agent traffic.

---

## Features

| Feature | Status |
|---------|--------|
| **Trace model** (`TraceSpan`, `TraceStore`) | Available: in-memory store, sorted ingestion, tested |
| **Audit model** (`AuditRecord`, `AuditLog`) | Available: NDJSON/CSV export methods, tested (not yet exposed via API) |
| **Policy model** (`GovernancePolicy`, `PolicyRule`) | Available: data model only; rule evaluation is a stub until v0.2.0 |
| **REST API** | Available now: `/health`, `/api/v1/traces/count`, `/api/v1/audit/count`, `/api/v1/policies/count` |
| **Trace ingestion via API** | Planned v0.2.0: `POST /api/v1/traces` |
| **Policy loading & evaluation via API** | Planned v0.2.0: `POST /api/v1/policies`, real-time gating |
| **Audit export via API** | Planned v0.2.0: `GET /api/v1/audit/export.ndjson` / `.csv` |
| **Azure Monitor / Sentinel / Entra ID** | Planned v0.3.0+, see [ROADMAP.md](ROADMAP.md) |

Full current vs. planned endpoint list: [docs/api_reference.md](docs/api_reference.md).

---

## Requirements

- Rust 1.78+
- Docker (optional, for containerised deployment)
- Azure subscription (optional, for Monitor / Sentinel integration)

---

## Quickstart

```bash
# Build all crates
cargo build --workspace

# Start API server (default: http://127.0.0.1:8080)
cargo run --bin agc-api

# Health check
curl http://127.0.0.1:8080/health

# Counts (all zero until ingestion lands in v0.2.0)
curl http://127.0.0.1:8080/api/v1/traces/count
curl http://127.0.0.1:8080/api/v1/audit/count
curl http://127.0.0.1:8080/api/v1/policies/count

# Run tests
cargo test --workspace
```

---

## Uninstall / Cleanup

`agc-api` keeps everything in memory: stopping the process (Ctrl-C) removes all ingested data, there is nothing to clean up on disk. Delete the `target/` build directory to reclaim build cache space.

---

## Documentation

- [Architecture](ARCHITECTURE.md)
- [Azure Integration Guide](docs/azure_integration.md)
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
