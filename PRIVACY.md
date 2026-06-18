# Privacy Policy : Agent Governance Console

## Summary / Zusammenfassung

AGC is privacy-by-default. Telemetry is opt-in and disabled at compile-time unless explicitly configured. No data is transmitted without operator consent.

AGC ist standardmässig privacy-by-default. Telemetrie ist opt-in und ohne explizite Konfiguration deaktiviert. Ohne Einwilligung des Operators werden keine Daten übertragen.

---

## Data Inventory / Datenbestand

| Data Type | Storage | Transmitted? | Notes |
|-----------|---------|-------------|-------|
| Trace spans | In-memory | No (unless OTLP enabled) | Discarded on restart |
| Audit records | In-memory / file export | No | Exported only on operator request |
| Policy definitions | In-memory | No | Loaded from local config |
| Agent IDs | In-memory | Opt-in | `include_agent_ids: false` by default in telemetry |
| Service metrics | In-memory | Opt-in via OTLP | Only when `telemetry.enabled: true` and endpoint set |

---

## Telemetry Opt-In / Telemetrie-Opt-in

Telemetry is **disabled by default**. To enable:

```toml
[telemetry]
enabled = true
endpoint = "https://<your-azure-monitor>.azure.com/v2/track"
service_name = "agc"
include_agent_ids = false  # set true only if agent IDs are non-personal
```

Enabling telemetry transmits OpenTelemetry spans to the configured endpoint. No personal data is included unless `include_agent_ids = true` and agent IDs contain personal information.

---

## Enterprise Considerations / Enterprise-Hinweise

- **GDPR / nDSG:** Audit records may contain pseudonymous agent IDs. Assess whether agent IDs qualify as personal data in your deployment.
- **Data Residency:** Telemetry endpoint must be in-region for EU deployments (e.g. `westeurope` Azure Monitor).
- **Retention:** Audit export files are under operator control. Define a retention policy in your DMS.
- **Microsoft Graph:** Integration with Microsoft Graph (docs/azure_integration.md) requires additional AAD permissions scoped to the operator's tenant.

---

## Contact / Kontakt

Security issues: see [SECURITY.md](SECURITY.md)

**Last updated: 2026-06-16**
