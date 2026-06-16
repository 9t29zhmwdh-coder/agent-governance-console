# Contributing — Agent Governance Console

## Contributing / Mitwirken

Contributions are welcome. Please read this guide before opening a pull request.

Beiträge sind willkommen. Bitte diesen Leitfaden lesen, bevor ein Pull Request geöffnet wird.

---

## Workflow

1. Fork the repository
2. Create a branch: `git checkout -b feat/your-feature`
3. Make your changes
4. Run checks:
   ```bash
   cargo fmt --all
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```
5. Push and open a PR against `main`

## Commit Style

```
type: short description (≤72 chars)
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`

## Privacy Rules

- Telemetry must remain opt-in — never enable by default
- No credentials, API keys or OTLP endpoints in source
- No real agent execution traces or audit records in test fixtures

## Code Style

- `rustfmt` default settings (enforced in CI)
- `clippy --workspace -- -D warnings` must pass
- No `unwrap()` in library code or API handlers
- `AuditLog` is append-only — never expose a delete/clear method

## API Versioning

REST endpoints are versioned under `/api/v1/`. Breaking changes require a new version prefix.

## Reporting Issues

[Bug Report](.github/ISSUE_TEMPLATE/bug_report.md) · [Feature Request](.github/ISSUE_TEMPLATE/feature_request.md)
