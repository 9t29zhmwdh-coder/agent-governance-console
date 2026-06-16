# Changelog — Agent Governance Console

All notable changes to this project will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — <EARLIEST_COMMIT_DATE>

### Added

- Rust workspace with `agc-core`, `agc-api` (Axum) and `agc-cli` crates
- `TraceStore` — sorted in-memory span store; query by trace ID; filter error spans
- `TraceSpan` — OpenTelemetry-compatible span model with structured attributes
- `PolicyEngine` — load `GovernancePolicy` objects; resolve applicable rules per agent/operation
- `PolicyRule` — `Warn`, `Block`, `Alert` actions on span-level, token-budget and operation conditions
- `AuditLog` — append-only record store; NDJSON (Azure Log Analytics) and CSV export
- `TelemetryConfig` / `TelemetrySink` — opt-in OTLP telemetry; disabled by default
- `ConsoleConfig` — typed configuration with `default_local()` (binds to 127.0.0.1:8080)
- Axum REST API: `/health`, `/api/v1/traces/count`, `/api/v1/audit/count`
- 6 unit tests covering trace ingestion, audit export, policy resolution, telemetry opt-in
- Bilingual README (EN / DE)
- Full documentation skeleton: ARCHITECTURE, PRIVACY, ROADMAP, CONTRIBUTING, SECURITY
- Azure integration guide, trace schema, policy DSL reference, API reference
- Examples: `trace_ingestion.rs`, `policy_enforcement.rs`
- Scripts: `azure_setup.sh`, `export_audit.sh`
