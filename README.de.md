<div align="center">

<img src="RayStudio.png" alt="RayStudio" width="80" />

# Agent Governance Console

**Governance, Tracing, Policy Enforcement und Observability für agentische Workflows.**

Ausführungs-Traces einlesen, Governance-Regeln anwenden, Audit-Protokolle exportieren; mit optionaler Azure Monitor-Integration.

Ausgerichtet an den [Microsoft Responsible AI-Grundsaetzen](https://learn.microsoft.com/de-de/azure/machine-learning/concept-responsible-ai) und konzipiert für Enterprise KI-Governance-Teams in regulierten Microsoft-Cloud-Umgebungen.

[![CI](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions/workflows/ci.yml/badge.svg)](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions) ![Azure Ready](https://img.shields.io/badge/Azure-Ready-0078d4?logo=microsoftazure&logoColor=white) ![Platform](https://img.shields.io/badge/Platform-Windows_%7C_Ubuntu-lightgrey) ![Rust](https://img.shields.io/badge/Rust-CE422B?logo=rust&logoColor=white) ![AI | Claude Code](https://img.shields.io/badge/AI-Claude_Code-black?logo=anthropic&logoColor=white) ![AI | Copilot](https://img.shields.io/badge/AI-Copilot-black?logo=github&logoColor=white)

</div>

---

## Übersicht

Agent Governance Console (AGC) ist ein Enterprise-Toolkit zur Steuerung, Beobachtung und Überprüfung von AI-Agent-Workflows. Es stellt eine REST-API zur Trace-Aufnahme, eine Governance-Policy-Engine, ein unveränderliches Audit-Protokoll und optionale Azure Monitor- / OTLP-Telemetrie bereit.

## Funktionen

| Funktion | Beschreibung |
|----------|--------------|
| **Trace-Ingestion** | OpenTelemetry-kompatibler Span-Import und In-Memory-Store |
| **Policy-Engine** | Regelbasierte Governance: Warnen, Blockieren oder Alertieren bei Span-Bedingungen |
| **Audit-Log** | Unveränderliche Protokolleinträge mit NDJSON- und CSV-Export |
| **Azure-Integration** | Hinweise für Azure Monitor (OTLP), Microsoft Graph, Log Analytics |
| **Opt-in-Telemetrie** | Standardmässig deaktiviert; OTLP-Endpunkt konfigurieren zum Aktivieren |
| **REST-API** | Axum-basierte HTTP-API für Trace- und Audit-Abfragen |

## Schnellstart

```bash
# Build
cargo build --workspace

# API-Server starten (Standard: http://127.0.0.1:8080)
cargo run --bin agc-api

# Health-Check
curl http://127.0.0.1:8080/health

# Tests ausführen
cargo test --workspace
```

## Dokumentation

- [Architektur](ARCHITECTURE.md)
- [Azure-Integrationshandbuch](docs/azure_integration.md)
- [Trace-Schema](docs/trace_schema.md)
- [Policy-DSL-Referenz](docs/policy_dsl.md)
- [API-Referenz](docs/api_reference.md)
- [Roadmap](ROADMAP.md)
- [Datenschutzrichtlinie](PRIVACY.md)

## Voraussetzungen

- Rust 1.78+
- Docker (optional, für containerisiertes Deployment)
- Azure Monitor / OTLP-Endpunkt (optional, für Telemetrie)

---

**Autor:** [Rafael Yilmaz](https://github.com/9t29zhmwdh-coder) · **Status:** Active · v0.1.0 · **Lizenz:** MIT
