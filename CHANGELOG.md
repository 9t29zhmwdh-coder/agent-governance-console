# Changelog: Agent Governance Console

All notable changes to this project will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.8] - 2026-07-13

### Added

- README.de.md was missing whole "Security"/"Sicherheit" and "Contributing"/"Mitwirken" sections that README.md has; added both.
- README.md's "Documentation" list was missing the "Trace Schema" doc link that README.de.md already had; added it for parity in both languages.

### Fixed

- README.de.md section order now matches README.md (Voraussetzungen/Requirements moved from the end to before Schnellstart/Quickstart).

## [0.1.7] - 2026-07-12

### Fixed

- Removed em-dashes/en-dashes across 17 files (docs, source comments, scripts, Cargo.toml descriptions), Swiss German orthography rule. Deleted stale scaffold bookkeeping files SKELETON.md and TEMPLATE_NOTES.md.
- Corrected a version drift: Cargo.toml's workspace version was still at 0.1.5 while the latest release tag was already v0.1.6.
## [0.1.6] - 2026-07-11

### Added

- Documented Dual-Licensing readiness assessment in ROADMAP.md: candidate for Community/Commercial split (governance/audit tooling for regulated environments), but blocked on auth, multi-tenancy and persistence, all already planned for later milestones.

## [0.1.5] - 2026-07-11

### Fixed

- Updated actions/checkout and codecov/codecov-action to their latest major versions in CI. GitHub is deprecating the Node.js 20 runtime, and actions still targeting it (like the previous actions/checkout@v4) were being forced onto Node 24 and crashing during their post-run cleanup step.

## [0.1.4] - 2026-07-11

### Fixed

- Corrected README hero section: only the title image and title stay centered, tagline, description and badges are now left aligned like the rest of the document

## [0.1.3] - 2026-07-10

### Fixed

- Removed em-dashes from CHANGELOG.md, replaced with colons/plain hyphens

## [0.1.2] - 2026-07-10

### Changed

- Moved the "New here? -> beginners guide" callout in README.md to the top of the file (previously only appeared near Requirements)

### Added

- Added the "New here?" beginner guide callout to README.de.md (was missing)

## [0.1.0] - <EARLIEST_COMMIT_DATE>

### Added

- Rust workspace with `agc-core`, `agc-api` (Axum) and `agc-cli` crates
- `TraceStore`: sorted in-memory span store; query by trace ID; filter error spans
- `TraceSpan`: OpenTelemetry-compatible span model with structured attributes
- `PolicyEngine`: load `GovernancePolicy` objects; resolve applicable rules per agent/operation
- `PolicyRule`: `Warn`, `Block`, `Alert` actions on span-level, token-budget and operation conditions
- `AuditLog`: append-only record store; NDJSON (Azure Log Analytics) and CSV export
- `TelemetryConfig` / `TelemetrySink`: opt-in OTLP telemetry; disabled by default
- `ConsoleConfig`: typed configuration with `default_local()` (binds to 127.0.0.1:8080)
- Axum REST API: `/health`, `/api/v1/traces/count`, `/api/v1/audit/count`
- 6 unit tests covering trace ingestion, audit export, policy resolution, telemetry opt-in
- Bilingual README (EN / DE)
- Full documentation skeleton: ARCHITECTURE, PRIVACY, ROADMAP, CONTRIBUTING, SECURITY
- Azure integration guide, trace schema, policy DSL reference, API reference
- Examples: `trace_ingestion.rs`, `policy_enforcement.rs`
- Scripts: `azure_setup.sh`, `export_audit.sh`
