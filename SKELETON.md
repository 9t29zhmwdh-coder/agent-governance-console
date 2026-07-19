# Agent Governance Console: Professional Repo Skeleton

**Generated:** 2026-06-16 | **Release:** v0.1.0 | **Stack:** Rust (agc-core, agc-api/Axum, agc-cli)

---

## Canonical File Tree

```
agent-governance-console/
├── Cargo.toml                              ← workspace root
├── agc-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                          ← public API + unit tests (6 tests)
│       ├── trace.rs                        ← TraceSpan, TraceStore
│       ├── policy.rs                       ← GovernancePolicy, PolicyEngine
│       ├── audit.rs                        ← AuditRecord, AuditLog (NDJSON/CSV)
│       └── telemetry.rs                    ← TelemetryConfig, TelemetrySink
├── agc-api/
│   ├── Cargo.toml
│   └── src/main.rs                         ← Axum REST server
├── agc-cli/
│   ├── Cargo.toml
│   └── src/main.rs
├── examples/
│   ├── trace_ingestion.rs
│   └── policy_enforcement.rs
├── scripts/
│   ├── azure_setup.sh                      ← provision Azure Monitor resources
│   └── export_audit.sh                     ← export audit log via REST API
├── docs/
│   ├── azure_integration.md
│   ├── trace_schema.md
│   ├── policy_dsl.md
│   └── api_reference.md
├── .github/
│   ├── workflows/ci.yml
│   ├── ISSUE_TEMPLATE/bug_report.md
│   ├── ISSUE_TEMPLATE/feature_request.md
│   └── PULL_REQUEST_TEMPLATE.md
├── README.md
├── README.de.md
├── ARCHITECTURE.md
├── PRIVACY.md
├── ROADMAP.md
├── CONTRIBUTING.md
├── SECURITY.md
├── CODE_OF_CONDUCT.md
├── CHANGELOG.md
├── RELEASES.md
├── SKELETON.md                             ← this file
└── TEMPLATE_NOTES.md
```

---

## File Contents

### `agc-core/src/lib.rs` (public API + tests)

```rust
//! Agent Governance Console: core library.

pub mod audit;
pub mod policy;
pub mod telemetry;
pub mod trace;

pub use audit::{AuditLog, AuditOutcome, AuditRecord};
pub use policy::{GovernancePolicy, PolicyAction, PolicyCondition, PolicyEngine, PolicyRule};
pub use telemetry::{TelemetryConfig, TelemetrySink};
pub use trace::{TraceLevel, TraceSpan, TraceStore};

#[derive(Debug, Clone, Default)]
pub struct ConsoleConfig {
    pub telemetry: TelemetryConfig,
    pub api_bind: String,
    pub audit_export_path: Option<std::path::PathBuf>,
}

impl ConsoleConfig {
    pub fn default_local() -> Self {
        Self {
            api_bind: "127.0.0.1:8080".into(),
            telemetry: TelemetryConfig { enabled: false, ..Default::default() },
            audit_export_path: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn trace_store_ingest_and_count() { /* ... */ }

    #[test]
    fn audit_log_export_csv_contains_header() {
        let log = AuditLog::new();
        assert!(log.export_csv().starts_with("id,timestamp,agent_id,action,outcome,policy_id\n"));
    }

    #[test]
    fn audit_log_export_ndjson_roundtrips() { /* ... */ }

    #[test]
    fn policy_engine_returns_applicable_rules() { /* ... */ }

    #[test]
    fn telemetry_disabled_by_default() {
        let cfg = ConsoleConfig::default_local();
        assert!(!TelemetrySink::from_config(&cfg.telemetry).is_enabled());
    }

    #[test]
    fn telemetry_enabled_when_configured() {
        let cfg = TelemetryConfig {
            enabled: true,
            endpoint: Some("https://example.azure.com/otlp".into()),
            ..Default::default()
        };
        assert!(TelemetrySink::from_config(&cfg).is_enabled());
    }
}
```

---

### `.github/workflows/ci.yml`

```yaml
name: CI
on:
  push: { branches: [main] }
  pull_request: { branches: [main] }
env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"
jobs:
  check:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: "clippy, rustfmt" }
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo test --workspace
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-audit --locked
      - run: cargo audit
```

> **Note:** Push to `.github/workflows/ci.yml` requires `workflows` OAuth scope.
> Run `gh auth refresh -s workflows` once, then push this file.

---

### `README.md` (EN) · `README.de.md` (DE) · `ARCHITECTURE.md` · `PRIVACY.md`
### `ROADMAP.md` · `CONTRIBUTING.md` · `SECURITY.md` · `CHANGELOG.md` · `RELEASES.md`

→ See individual files in `/tmp/agc/`

---

## Migration Checklist

### Step 1: Create GitHub repo

```bash
gh repo create 9t29zhmwdh-coder/agent-governance-console \
  --public \
  --description "Governance, tracing, policy enforcement and observability for agentic workflows"
```

### Step 2: Prepare blobs (Git Tree API, single commit)

```bash
export PATH="/usr/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"
OWNER="9t29zhmwdh-coder"
REPO="agent-governance-console"
GH="/opt/homebrew/bin/gh"
B64="/usr/bin/base64"
TR="/usr/bin/tr"
SRC="/tmp/agc"

FILES=(
  "Cargo.toml" "agc-core/Cargo.toml" "agc-core/src/lib.rs"
  "agc-core/src/trace.rs" "agc-core/src/policy.rs"
  "agc-core/src/audit.rs" "agc-core/src/telemetry.rs"
  "agc-api/Cargo.toml" "agc-api/src/main.rs"
  "agc-cli/Cargo.toml" "agc-cli/src/main.rs"
  "examples/trace_ingestion.rs" "examples/policy_enforcement.rs"
  "scripts/azure_setup.sh" "scripts/export_audit.sh"
  "docs/azure_integration.md" "docs/trace_schema.md"
  "docs/policy_dsl.md" "docs/api_reference.md"
  "README.md" "README.de.md" "ARCHITECTURE.md" "PRIVACY.md"
  "ROADMAP.md" "CONTRIBUTING.md" "SECURITY.md" "CODE_OF_CONDUCT.md"
  "CHANGELOG.md" "RELEASES.md" "SKELETON.md" "TEMPLATE_NOTES.md"
  ".github/workflows/ci.yml"
  ".github/ISSUE_TEMPLATE/bug_report.md"
  ".github/ISSUE_TEMPLATE/feature_request.md"
  ".github/PULL_REQUEST_TEMPLATE.md"
)

TREE_JSON="["
FIRST=true
for f in "${FILES[@]}"; do
  SHA=$($GH api "repos/$OWNER/$REPO/git/blobs" --method POST \
    --input - <<< "{\"content\":\"$($B64 < "$SRC/$f" | $TR -d '\n')\",\"encoding\":\"base64\"}" \
    --jq '.sha')
  [ "$FIRST" = "true" ] && FIRST=false || TREE_JSON+=","
  TREE_JSON+="{\"path\":\"$f\",\"mode\":\"100644\",\"type\":\"blob\",\"sha\":\"$SHA\"}"
done
TREE_JSON+="]"
```

### Step 3: Create tree, commit, branch

```bash
TREE_SHA=$(echo "{\"tree\":$TREE_JSON}" | \
  $GH api "repos/$OWNER/$REPO/git/trees" --method POST --input - --jq '.sha')

COMMIT_SHA=$(echo "{\"message\":\"scaffold: Add project template files\",\"tree\":\"$TREE_SHA\",\"parents\":[]}" | \
  $GH api "repos/$OWNER/$REPO/git/commits" --method POST --input - --jq '.sha')

$GH api "repos/$OWNER/$REPO/git/refs" \
  --method POST -f ref="refs/heads/main" -f sha="$COMMIT_SHA"
```

### Step 4: Set default branch

```bash
$GH api "repos/$OWNER/$REPO" --method PATCH -f default_branch="main"
```

### Step 5: Validate

```bash
$GH api "repos/$OWNER/$REPO/contents/SKELETON.md" --jq '.name'
$GH api "repos/$OWNER/$REPO/contents/agc-core/src/lib.rs" --jq '.name'
```

### Step 6: Run tests locally

```bash
cd /tmp/agc && cargo check --workspace && cargo test --workspace
```

### Step 7: Push CI workflow (requires workflows scope)

```bash
gh auth refresh -s workflows
```

### Step 8: Add topics

```bash
$GH api "repos/$OWNER/$REPO/topics" \
  --method PUT \
  -f "names[]=rust" -f "names[]=opentelemetry" -f "names[]=governance" \
  -f "names[]=azure" -f "names[]=agents" -f "names[]=observability"
```

### Step 9: Tag initial commit

```bash
INIT_SHA=$($GH api "repos/$OWNER/$REPO/git/ref/heads/main" --jq '.object.sha')
$GH api "repos/$OWNER/$REPO/git/refs" \
  --method POST -f ref="refs/tags/v0.1.0" -f sha="$INIT_SHA"
```

### Step 10: Create release

```bash
$GH api "repos/$OWNER/$REPO/releases" \
  --method POST \
  -f tag_name="v0.1.0" \
  -f name="v0.1.0: Initial import" \
  -f body="Initial import, earliest commit date: <EARLIEST_COMMIT_DATE>

Governance, tracing, policy enforcement and observability for agentic workflows.
Trace ingestion, audit log (NDJSON/CSV export), policy engine stubs, opt-in Azure Monitor telemetry." \
  -F prerelease=true --jq '.html_url'
```

---

## Release Metadata (JSON)

```json
{
  "repo": "agent-governance-console",
  "owner": "9t29zhmwdh-coder",
  "tag": "v0.1.0",
  "name": "v0.1.0: Initial import",
  "earliest_commit_date": "<EARLIEST_COMMIT_DATE>",
  "prerelease": true,
  "body": "Initial import, earliest commit date: <EARLIEST_COMMIT_DATE>\n\nGovernance, tracing, policy enforcement and observability for agentic workflows.\nTrace ingestion, audit log (NDJSON/CSV export), policy engine stubs, opt-in Azure Monitor telemetry.",
  "topics": ["rust", "opentelemetry", "governance", "azure", "agents", "observability"],
  "license": "MIT",
  "stack": "Rust (agc-core, agc-api/Axum, agc-cli)",
  "platform": "Linux / Windows / macOS",
  "generated": "2026-06-16"
}
```

---

## PR Description (EN)

```markdown
## Summary

- Add full project skeleton for `agent-governance-console`
- Rust workspace: `agc-core` (trace, policy, audit, telemetry), `agc-api` (Axum REST), `agc-cli`
- 6 unit tests covering trace ingestion, audit export, policy resolution, telemetry opt-in
- Opt-in telemetry: disabled by default, OTLP-ready for Azure Monitor
- Append-only `AuditLog` with NDJSON and CSV export
- Bilingual documentation: README (EN / DE), ARCHITECTURE, PRIVACY, ROADMAP
- CI matrix (Ubuntu / macOS / Windows) + security audit job
- Azure integration guide, trace schema, policy DSL reference, API reference
- Examples: `trace_ingestion.rs`, `policy_enforcement.rs`
- Scripts: `azure_setup.sh`, `export_audit.sh`

## Test Plan

- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace`: all 6 tests green
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `curl http://127.0.0.1:8080/health` returns `{"status":"ok"}`
- [ ] README renders correctly on GitHub
- [ ] CI workflow triggers on push
```

## PR-Beschreibung (DE)

```markdown
## Zusammenfassung

- Vollständiges Projekt-Skeleton für `agent-governance-console`
- Rust-Workspace: `agc-core` (Trace, Policy, Audit, Telemetrie), `agc-api` (Axum REST), `agc-cli`
- 6 Unit-Tests für Trace-Ingestion, Audit-Export, Policy-Auflösung und Telemetrie-Opt-in
- Opt-in-Telemetrie: standardmässig deaktiviert, OTLP-bereit für Azure Monitor
- Unveränderliches `AuditLog` mit NDJSON- und CSV-Export
- Zweisprachige Dokumentation: README (EN / DE), ARCHITECTURE, PRIVACY, ROADMAP
- CI-Matrix (Ubuntu / macOS / Windows) + Security-Audit-Job
- Azure-Integrationshandbuch, Trace-Schema, Policy-DSL-Referenz, API-Referenz
- Beispiele: `trace_ingestion.rs`, `policy_enforcement.rs`
- Skripte: `azure_setup.sh`, `export_audit.sh`

## Testplan

- [ ] `cargo check --workspace` erfolgreich
- [ ] `cargo test --workspace`: alle 6 Tests grün
- [ ] `cargo clippy --workspace -- -D warnings` sauber
- [ ] `curl http://127.0.0.1:8080/health` gibt `{"status":"ok"}` zurück
- [ ] README wird auf GitHub korrekt dargestellt
- [ ] CI-Workflow wird bei Push ausgelöst
```

---

*agent-governance-console · RayStudio · Rafael Yilmaz · MIT License · 2026*
