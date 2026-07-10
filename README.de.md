<div align="center">

<img src="RayStudio.png" alt="RayStudio" width="80" />

# Agent Governance Console

[🇬🇧 English Version](README.md)

**Governance, Tracing, Policy Enforcement und Observability für agentische Workflows.**

Ein Rust-Workspace als Grundlage für Tracing, Policy Enforcement und Audit-Logging von AI-Agent-Aktivität; Azure Monitor- und Microsoft-Sentinel-Integration sind auf der Roadmap.

Ausgerichtet an den [Microsoft Responsible AI Grundsätzen](https://learn.microsoft.com/de-de/azure/machine-learning/concept-responsible-ai) und konzipiert für Enterprise KI-Governance-Teams in regulierten Microsoft-Cloud-Umgebungen.

[![CI](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions/workflows/ci.yml/badge.svg)](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions) ![Azure Ready](https://img.shields.io/badge/Azure-Ready-0078d4?logo=microsoftazure&logoColor=white) ![Platform](https://img.shields.io/badge/Platform-Windows_%7C_Ubuntu-lightgrey) ![Rust](https://img.shields.io/badge/Rust-CE422B?logo=rust&logoColor=white) ![AI | Claude Code](https://img.shields.io/badge/AI-Claude_Code-black?logo=anthropic&logoColor=white) ![AI | Copilot](https://img.shields.io/badge/AI-Copilot-black?logo=github&logoColor=white) [![Release](https://img.shields.io/github/v/release/9t29zhmwdh-coder/agent-governance-console?color=3F8E7E)](https://github.com/9t29zhmwdh-coder/agent-governance-console/releases) [![License](https://img.shields.io/github/license/9t29zhmwdh-coder/agent-governance-console?color=lightgrey)](LICENSE)

</div>

> **So läuft das:** AGC ist kein gehosteter Dienst und keine Desktop-App. `agc-api` ist ein kleiner REST-API-Server, den du selbst mit `cargo run` startest, standardmässig auf `127.0.0.1:8080`. Es gibt keinen Installer und nichts läuft im Hintergrund, du startest und stoppst den Prozess selbst.

![Agent Governance Console](docs/screenshot.png)

---

> 🌱 Neu hier? → [Schritt-für-Schritt-Anleitung für Einsteiger](GETTING_STARTED.md)

---

## Übersicht

Agent Governance Console (AGC) ist ein früher Rust-Workspace (`agc-core`, `agc-api`, `agc-cli`) zur Steuerung, Beobachtung und Überprüfung von AI-Agent-Workflows. Die Core-Bibliothek modelliert Trace-Spans, Governance-Policies und Audit-Records bereits mit getesteter API; die REST-API bietet aktuell nur schreibgeschützte Health- und Zähl-Endpunkte, Ingestion, Policy-Laden und Audit-Export sind für v0.2.0 geplant (siehe [ROADMAP.md](ROADMAP.md)). Azure Monitor, Microsoft Sentinel und Entra ID sind für v0.3.0+ geplant und noch nicht umgesetzt.

**In der Praxis:** Aktuell bekommst du eine getestete Rust-Bibliothek zur Modellierung von Agent-Traces, -Policies und -Audit-Records, plus eine REST-API die meldet, wie viele davon geladen sind. Das ist eine Grundlage zum Weiterbauen, noch keine fertige Governance-Schicht für produktiven Agent-Traffic.

## Funktionen

| Funktion | Status |
|----------|--------|
| **Trace-Modell** (`TraceSpan`, `TraceStore`) | Verfügbar: In-Memory-Store, sortierte Ingestion, getestet |
| **Audit-Modell** (`AuditRecord`, `AuditLog`) | Verfügbar: NDJSON-/CSV-Export-Methoden, getestet (noch nicht über API) |
| **Policy-Modell** (`GovernancePolicy`, `PolicyRule`) | Verfügbar: nur Datenmodell, Regelauswertung ist bis v0.2.0 ein Stub |
| **REST-API** | Verfügbar: `/health`, `/api/v1/traces/count`, `/api/v1/audit/count`, `/api/v1/policies/count` |
| **Trace-Ingestion via API** | Geplant v0.2.0: `POST /api/v1/traces` |
| **Policy-Laden & Auswertung via API** | Geplant v0.2.0: `POST /api/v1/policies`, Echtzeit-Gating |
| **Audit-Export via API** | Geplant v0.2.0: `GET /api/v1/audit/export.ndjson` / `.csv` |
| **Azure Monitor / Sentinel / Entra ID** | Geplant ab v0.3.0, siehe [ROADMAP.md](ROADMAP.md) |

Vollständige Liste aktueller und geplanter Endpunkte: [docs/api_reference.md](docs/api_reference.md).

## Schnellstart

```bash
# Build
cargo build --workspace

# API-Server starten (Standard: http://127.0.0.1:8080)
cargo run --bin agc-api

# Health-Check
curl http://127.0.0.1:8080/health

# Zähler (alle 0 bis Ingestion in v0.2.0 kommt)
curl http://127.0.0.1:8080/api/v1/traces/count
curl http://127.0.0.1:8080/api/v1/audit/count
curl http://127.0.0.1:8080/api/v1/policies/count

# Tests ausführen
cargo test --workspace
```

## Deinstallation / Datenbereinigung

`agc-api` hält alles im Arbeitsspeicher: Stoppen des Prozesses (Ctrl-C) entfernt alle aufgenommenen Daten, es bleibt nichts auf der Platte zurück. Lösche `target/` um Build-Cache-Speicherplatz freizugeben.

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

**Autor:** [Rafael Yilmaz](https://github.com/9t29zhmwdh-coder) · **Status:** Early Release · ![version](https://img.shields.io/github/v/release/9t29zhmwdh-coder/agent-governance-console?color=6b7280&style=flat-square) · **Lizenz:** MIT
