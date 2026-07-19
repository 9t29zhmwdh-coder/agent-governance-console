<div align="center">

<img src="RayStudio.png" alt="RayStudio" width="120" />

# Agent Governance Console

</div>

[🇬🇧 English Version](README.md)

**Governance, Tracing, Policy Enforcement und Observability für agentische Workflows.**

Ein Rust-Workspace für Tracing, Policy Enforcement und Audit-Logging von AI-Agent-Aktivität, mit echtem Azure-Monitor-Telemetrie-/Audit-Export, Microsoft-Graph-Integration und Microsoft-Sentinel-Analytics-Rule-Export.

Ausgerichtet an den [Microsoft Responsible AI Grundsätzen](https://learn.microsoft.com/de-de/azure/machine-learning/concept-responsible-ai) und konzipiert für Enterprise KI-Governance-Teams in regulierten Microsoft-Cloud-Umgebungen.

[![CI](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions/workflows/ci.yml/badge.svg)](https://github.com/9t29zhmwdh-coder/agent-governance-console/actions) ![Azure Ready](https://img.shields.io/badge/Azure-Ready-0078d4?logo=microsoftazure&logoColor=white) ![Platform](https://img.shields.io/badge/Platform-Windows_%7C_Ubuntu-lightgrey) ![Rust](https://img.shields.io/badge/Rust-CE422B?logo=rust&logoColor=white) ![AI | Claude Code](https://img.shields.io/badge/AI-Claude_Code-black?logo=anthropic&logoColor=white) ![AI | Copilot](https://img.shields.io/badge/AI-Copilot-black?logo=github&logoColor=white) [![Release](https://img.shields.io/github/v/release/9t29zhmwdh-coder/agent-governance-console?color=3F8E7E)](https://github.com/9t29zhmwdh-coder/agent-governance-console/releases) [![License](https://img.shields.io/github/license/9t29zhmwdh-coder/agent-governance-console?color=lightgrey)](LICENSE)


> **So läuft das:** AGC ist kein gehosteter Dienst und keine Desktop-App. `agc-api` ist ein kleiner REST-API-Server, den du selbst mit `cargo run` startest, standardmässig auf `127.0.0.1:8080`. Es gibt keinen Installer und nichts läuft im Hintergrund, du startest und stoppst den Prozess selbst. Öffne `http://127.0.0.1:8080/dashboard` im Browser für eine leichtgewichtige UI über dieselbe API.

![Agent Governance Console](docs/screenshot.png)

---

> 🌱 Neu hier? → [Schritt-für-Schritt-Anleitung für Einsteiger](GETTING_STARTED.md)

---

## Übersicht

Agent Governance Console (AGC) ist ein Rust-Workspace (`agc-core`, `agc-api`, `agc-cli`, `agc-azure`) in Version 1.0.0 zur Steuerung, Beobachtung und Überprüfung von AI-Agent-Workflows. Die Core-Bibliothek modelliert Trace-Spans, Governance-Policies und Audit-Records mit getesteter API; die REST-API unterstützt volle Trace-Ingestion mit Echtzeit-Policy-Gate, Policy-Laden und paginierte/streambare Audit-Abfragen; und die Azure-Integration (OTLP-Telemetrie-Export, Managed-Identity-authentifizierter Audit-Push zu Azure Monitor, Microsoft-Graph-Agent-Lookup) ist real umgesetzt, nicht nur geplant (siehe [ROADMAP.md](ROADMAP.md)).

**In der Praxis:** Du kannst eine Governance-Policy laden, Agent-Trace-Spans dagegen posten und zutreffende Regeln in Echtzeit warnen, blockieren oder (protokolliert, noch nicht extern ausgeliefert) alarmieren lassen, wobei jede Entscheidung in einem abfragbaren, exportierbaren Audit-Log landet, das sich zu Azure Monitor pushen lässt. Trace- und Audit-Daten sind pro Tenant isoliert (`X-Tenant-Id`, je eigener Store); Policies bleiben geteilte Governance über alle Tenants hinweg. Optionales RBAC (`Authorization: Bearer <JWT>`, HS256 oder Entra ID) schützt Schreibzugriffe mit einer `Admin`-Rolle.

## Funktionen

| Funktion | Status |
|----------|--------|
| **Trace-Modell** (`TraceSpan`, `TraceStore`) | Verfügbar: In-Memory-Store, sortierte Ingestion, getestet |
| **Audit-Modell** (`AuditRecord`, `AuditLog`) | Verfügbar: SQLite-basiert (standardmässig im Speicher, oder persistent pro Tenant über `AGC_AUDIT_DB_DIR`), NDJSON-/CSV-Export, paginierte Abfrage, getestet und über API erreichbar |
| **Multi-Tenant-Isolation** | Verfügbar: `X-Tenant-Id`-Header (Pflicht, kein stiller Default) löst pro Tenant einen isolierten Trace+Audit-Store auf, lazy erzeugt; `GET /api/v1/tenants` listet bisher gesehene Tenants. Policies bleiben global/geteilt. |
| **Policy-Modell** (`GovernancePolicy`, `PolicyRule`) | Verfügbar: echte Bedingungsauswertung (Span-Level, Token-Budget, Operation-Glob), kein reines Datenmodell mehr |
| **Trace-Ingestion via API** | Verfügbar: `POST /api/v1/traces`, `GET /api/v1/traces/{trace_id}` |
| **Policy-Laden & Echtzeit-Gating via API** | Verfügbar: `POST /api/v1/policies`; jeder aufgenommene Span wird gegen geladene Policies ausgewertet, `block`-Regeln lehnen den Span mit `403` ab |
| **Audit-Abfrage & -Export via API** | Verfügbar: `GET /api/v1/audit?limit=&offset=`, `GET /api/v1/audit/export.ndjson` / `.csv` |
| **REST-API** | `/health`, `/api/v1/tenants`, `/api/v1/traces`, `/api/v1/traces/count`, `/api/v1/traces/{trace_id}`, `/api/v1/audit`, `/api/v1/audit/count`, `/api/v1/audit/export.ndjson`, `/api/v1/audit/export.csv`, `/api/v1/compliance/report`, `/api/v1/policies`, `/api/v1/policies/count` |
| **OTLP-Telemetrie-Export zu Azure Monitor** | Verfügbar: `AGC_TELEMETRY_ENDPOINT` verdrahtet einen echten OTLP/HTTP-Exporter in jeden aufgenommenen Span; `AGC_TELEMETRY_MANAGED_IDENTITY` authentifiziert ihn mit einem Managed-Identity-Token, kein Client-Secret |
| **Audit-Export zu Azure Monitor (DCR)** | Verfügbar: `agc-cli azure push-audit`, Managed-Identity-authentifiziert, kein Client-Secret |
| **Microsoft-Graph-Agent-Lookup** | Verfügbar: `agc-cli azure list-agents` (App-Registrierungen mit Tag `agc-agent`) |
| **YAML-Policy-DSL** | Verfügbar: `GovernancePolicy::from_yaml` parst YAML oder JSON (ein Parser, YAML ist ein JSON-Superset); `agc-cli policy validate` für Offline-Checks |
| **Policy-Hot-Reload** | Verfügbar: `AGC_POLICY_DIR` lädt und aktualisiert jede Policy-Datei in einem Verzeichnis live; ein fehlerhafter Edit behält den vorherigen Policy-Stand statt ihn zu löschen |
| **OPA/Rego-Export** | Verfügbar: `agc-cli policy to-rego` rendert einen strukturellen Rego-Stub pro Policy: ein Ausgangspunkt zum manuellen Portieren, keine vollständige semantische Übersetzung |
| **RBAC für REST-API** | Verfügbar: `AGC_JWT_SECRET` (HS256) oder `AGC_AAD_TENANT_ID` (Entra ID RS256) schützt Schreibzugriffe mit `Admin`, Lesezugriffe brauchen `Viewer`; opt-in, standardmässig aus |
| **Microsoft-Sentinel-Export** | Verfügbar: `agc-cli sentinel export --format kql\|arm` erzeugt 4 eingebaute Analytics-Rule-Vorlagen aus der AGC-Audit-Tabelle, als KQL-Dateien oder als deploybares ARM-Template, siehe [docs/sentinel.md](docs/sentinel.md) |
| **Compliance-Report-Export** | Verfügbar: `GET /api/v1/compliance/report` (Markdown oder `?format=json`), berichtet gegen 4 der 6 Microsoft-Responsible-AI-Prinzipien anhand echter Tenant-Audit-/Trace-Daten, siehe [docs/compliance.md](docs/compliance.md) |
| **Dashboard-UI** | Verfügbar: `GET /dashboard`, eine eigenständige statische Seite (kein Build-Schritt) mit Health, Tenants, Policies, Tenant-Traces, paginierter Audit-Tabelle und Compliance-Report, siehe [docs/dashboard.md](docs/dashboard.md) |
| **Kubernetes-Deployment (Helm-Chart)** | Verfügbar: `helm/agent-governance-console` plus ein Root-`Dockerfile` (Deployment, Service, optionales Ingress/HPA/PVC/Policy-ConfigMap, beide RBAC-Modi, Azure Workload Identity), siehe [docs/helm.md](docs/helm.md) |
| **Ingest-SLA (p99 < 10ms bei 1K Spans/s)** | Verifiziert: `agc-cli bench ingest`, gemessener p99 deutlich unter 1ms bei Zielrate, hält bei doppelter Rate und im realistischen Worst Case, siehe [docs/performance.md](docs/performance.md) |

Vollständige Liste aktueller und geplanter Endpunkte: [docs/api_reference.md](docs/api_reference.md).

## Voraussetzungen

- Rust 1.78+
- Docker (optional, für containerisiertes Deployment)
- Azure-Abonnement (optional, für OTLP-Telemetrie-Export, Audit-Push zu Azure Monitor und Microsoft-Graph-Agent-Lookup; siehe [docs/azure_integration.md](docs/azure_integration.md))

## Schnellstart

```bash
# Build
cargo build --workspace

# API-Server starten (Standard: http://127.0.0.1:8080)
cargo run --bin agc-api

# Gleich, aber jedes Tenants Audit-Log persistent in einer eigenen SQLite-Datei
AGC_AUDIT_DB_DIR=./agc-audit cargo run --bin agc-api

# Health-Check
curl http://127.0.0.1:8080/health

# Zähler (Trace-/Audit-Endpunkte brauchen einen Tenant)
curl http://127.0.0.1:8080/api/v1/traces/count -H "X-Tenant-Id: tenant-a"
curl http://127.0.0.1:8080/api/v1/audit/count -H "X-Tenant-Id: tenant-a"
curl http://127.0.0.1:8080/api/v1/policies/count

# Tests ausführen
cargo test --workspace
```

### Das Policy-Gate und Multi-Tenant-Isolation ausprobieren

```bash
# Policy laden, die alles ab Error-Level blockiert (global, gilt für jeden Tenant)
curl -X POST http://127.0.0.1:8080/api/v1/policies -H "content-type: application/json" -d '{
  "policy_id": "p1", "name": "Error gate", "agent_scope": [],
  "rules": [{"rule_id": "r1", "description": "Block on error",
    "condition": {"type": "span_level_at_least", "level": "error"},
    "action": {"type": "block", "reason": "too severe"}}]
}'

# Dieser Span wird in tenant-as Store normal aufgenommen (201)
curl -X POST http://127.0.0.1:8080/api/v1/traces -H "content-type: application/json" -H "X-Tenant-Id: tenant-a" -d '{
  "span_id": "3fa85f64-5717-4562-b3fc-2c963f66afa6", "trace_id": "3fa85f64-5717-4562-b3fc-2c963f66afa7",
  "parent_span_id": null, "agent_id": "agent-1", "operation": "tool_call", "level": "info",
  "started_at": "2026-07-17T12:00:00Z", "ended_at": null, "attributes": {}
}'

# Dieser wird vom (globalen) Policy-Gate abgelehnt (403) und nie gespeichert
curl -X POST http://127.0.0.1:8080/api/v1/traces -H "content-type: application/json" -H "X-Tenant-Id: tenant-a" -d '{
  "span_id": "3fa85f64-5717-4562-b3fc-2c963f66afa8", "trace_id": "3fa85f64-5717-4562-b3fc-2c963f66afa7",
  "parent_span_id": null, "agent_id": "agent-1", "operation": "risky_call", "level": "error",
  "started_at": "2026-07-17T12:00:01Z", "ended_at": null, "attributes": {}
}'

# tenant-bs Store bleibt unberührt: echte Isolation, keine gefilterte Ansicht
curl http://127.0.0.1:8080/api/v1/traces/count -H "X-Tenant-Id: tenant-b"   # {"span_count":0,...}

# tenant-as Block-Entscheidung steht in tenant-as Audit-Log
curl http://127.0.0.1:8080/api/v1/audit -H "X-Tenant-Id: tenant-a"
curl http://127.0.0.1:8080/api/v1/audit/export.csv -H "X-Tenant-Id: tenant-a"

# Jeder Tenant, der bisher mindestens eine Anfrage gestellt hat
curl http://127.0.0.1:8080/api/v1/tenants
```

Vollständige Endpunkt- und Policy-Schema-Referenz: [docs/api_reference.md](docs/api_reference.md).

### Die YAML-Policy-DSL und Hot-Reload ausprobieren

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

# Policy-Datei offline validieren, ohne laufenden Server
cargo run --bin agc-cli -- policy validate ./policies/block-errors.yaml

# agc-api auf das Verzeichnis zeigen: lädt alle Policy-Dateien beim Start
# und lädt automatisch neu, sobald sich eine Datei darin ändert
AGC_POLICY_DIR=./policies cargo run --bin agc-api

# Strukturellen Rego-Stub für eine Policy rendern (siehe docs/policy_dsl.md
# für genau was eine echte Übersetzung vs. ein Ausgangspunkt zum manuellen
# Portieren ist)
cargo run --bin agc-cli -- policy to-rego ./policies/block-errors.yaml
```

### RBAC ausprobieren (optional)

```bash
# HS256-JWT-Auth mit geteiltem Secret aktivieren
AGC_JWT_SECRET=s3cret cargo run --bin agc-api

# Kein Token: 401
curl -w "\nHTTP %{http_code}\n" http://127.0.0.1:8080/api/v1/traces/count -H "X-Tenant-Id: tenant-a"

# Ein Viewer-Token kann lesen, aber nicht schreiben (403 bei POST)
# Ein Admin-Token kann beides -- erzeuge eines mit einer beliebigen
# HS256-JWT-Bibliothek, gleiches Secret, Payload {"roles": ["admin"]}.
```

Oder `AGC_AAD_TENANT_ID` (+ optional `AGC_AAD_AUDIENCE`) auf einen echten Entra-ID-Tenant statt ein geteiltes Secret zeigen; vollständige RBAC-Sektion in `docs/api_reference.md`.

### Die Azure-Integration ausprobieren (optional)

```bash
# Azure-Ressourcen einmalig anlegen (braucht ein Azure-Abonnement und az CLI)
AZURE_RG=my-rg AZURE_LOCATION=westeurope ./scripts/azure_setup.sh

# Spans bei Ingestion über OTLP zu Azure Monitor exportieren
AGC_TELEMETRY_ENDPOINT="https://<region>.otelcollector.azure.com/v1/traces" cargo run --bin agc-api

# Entra-ID-App-Registrierungen mit Tag 'agc-agent' auflisten (Managed Identity + Microsoft Graph)
cargo run --bin agc-cli -- azure list-agents

# Lokalen Audit-Export zu einer Azure-Monitor-DCR pushen (Managed Identity, kein Client-Secret)
./scripts/export_audit.sh ndjson
cargo run --bin agc-cli -- azure push-audit --file audit-*.ndjson \
  --dce-endpoint "https://<name>.<region>-1.ingest.monitor.azure.com" \
  --dcr-id "dcr-..." --stream "Custom-AGCAudit_CL"
```

Vollständige Anleitung inkl. was Mock-getestet vs. gegen echtes Azure verifiziert ist: [docs/azure_integration.md](docs/azure_integration.md).

## Deinstallation / Datenbereinigung

Standardmässig hält `agc-api` alles im Arbeitsspeicher: Stoppen des Prozesses (Ctrl-C) entfernt alle aufgenommenen Daten, es bleibt nichts auf der Platte zurück. Falls du mit gesetztem `AGC_AUDIT_DB_DIR` gestartet hast, bleibt jedes Tenants Audit-Log in einer eigenen `{tenant_id}.sqlite`-Datei in diesem Verzeichnis erhalten; lösche das Verzeichnis für die gesamte Audit-Historie oder eine einzelne Datei für nur einen Tenant. Lösche `target/` um Build-Cache-Speicherplatz freizugeben.

## Dokumentation

- [Architektur](ARCHITECTURE.md)
- [Azure-Integrationshandbuch](docs/azure_integration.md)
- [Trace-Schema](docs/trace_schema.md)
- [Policy-DSL-Referenz](docs/policy_dsl.md)
- [API-Referenz](docs/api_reference.md)
- [Datenschutz & Telemetrie](PRIVACY.md)
- [Roadmap](ROADMAP.md)

## Sicherheit

Siehe [SECURITY.md](SECURITY.md) für Schwachstellenmeldungen. Alle Policy-Entscheidungen werden unveränderlich protokolliert; Audit-Records können über die API nicht verändert oder gelöscht werden.

## Mitwirken

Siehe [CONTRIBUTING.md](CONTRIBUTING.md).

---

**Autor:** [Rafael Yilmaz](https://github.com/9t29zhmwdh-coder) · **Status:** Early Release · ![version](https://img.shields.io/github/v/release/9t29zhmwdh-coder/agent-governance-console?color=6b7280&style=flat-square) · **Lizenz:** MIT
