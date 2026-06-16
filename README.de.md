<div align="center">

<img src="RayStudio.png" alt="RayStudio" width="80" />

# Agent Governance Console

**Governance, Tracing, Policy Enforcement und Observability für agentische Workflows.**

Ausführungs-Traces einlesen, Governance-Regeln anwenden, Audit-Protokolle exportieren — mit optionaler Azure Monitor-Integration.

[![CI](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions/workflows/ci.yml/badge.svg)](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions)
[![Lizenz: MIT](https://img.shields.io/badge/Lizenz-MIT-blue.svg)](LICENSE)
[![Plattform: Linux / Windows / macOS](https://img.shields.io/badge/Plattform-Linux%20%7C%20Windows%20%7C%20macOS-blue)](#)

</div>

---

## Übersicht

Agent Governance Console (AGC) ist ein Enterprise-Toolkit zur Steuerung, Beobachtung und Überprüfung von KI-Agent-Workflows. Es stellt eine REST-API zur Trace-Aufnahme, eine Governance-Policy-Engine, ein unveränderliches Audit-Protokoll und optionale Azure Monitor- / OTLP-Telemetrie bereit.

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
- Docker (optional — für containerisiertes Deployment)
- Azure Monitor / OTLP-Endpunkt (optional — für Telemetrie)

---

<div align="center">

**RayStudio · Rafael Yilmaz · MIT-Lizenz · 2026**

</div>
