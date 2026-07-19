# Security Policy: Agent Governance Console

## Supported Versions / Unterstützte Versionen

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅        |

## Reporting a Vulnerability / Sicherheitslücke melden

**Do not open a public GitHub issue for security vulnerabilities.**

Report via [GitHub Security Advisory](https://github.com/9t29zhmwdh-coder/agent-governance-console/security/advisories/new)
or contact the maintainer via the GitHub profile.

Response within 7 business days.

---

## Security Design / Sicherheitsarchitektur

| Property | Detail |
|----------|--------|
| Network (default) | Binds to `127.0.0.1:8080`, no external exposure |
| Telemetry | Opt-in; disabled by default |
| Audit log | Append-only; no delete endpoint |
| Secrets | No credentials stored (OTLP endpoint in env var or config file) |
| Dependencies | Pinned in `Cargo.lock`; audited with `cargo audit` |
| API auth | None in v0.1 (local-only); AAD JWT planned for v1.0 |

## Supply Chain

```bash
cargo audit  # check advisory database
cargo deny check  # license + duplicate dependency check
```

**Last updated: 2026-06-16**
