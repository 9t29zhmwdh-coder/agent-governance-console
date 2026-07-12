# Azure Integration Guide: Agent Governance Console

## Overview

AGC supports three Azure integration points, all opt-in:

| Integration | Purpose | Required AAD Permissions |
|-------------|---------|--------------------------|
| Azure Monitor (OTLP) | Telemetry span export | `Monitoring Metrics Publisher` |
| Azure Log Analytics | Audit NDJSON ingest | `Log Analytics Contributor` |
| Microsoft Graph | Read agent app registrations | `Application.Read.All` |

---

## Azure Monitor (OTLP)

### Setup

1. Run `./scripts/azure_setup.sh` to provision the Log Analytics Workspace and Application Insights instance.
2. Copy the connection string output into your AGC configuration:

```toml
[telemetry]
enabled = true
endpoint = "InstrumentationKey=<key>;IngestionEndpoint=https://<region>.in.applicationinsights.azure.com/"
service_name = "agc"
include_agent_ids = false
```

3. Restart AGC. Spans will appear in Application Insights → Live Metrics.

### OTLP Endpoint (alternative)

Azure Monitor supports native OTLP ingestion (preview):
```
https://<region>.otelcollector.azure.com/v1/traces
```
Use with a Managed Identity for secretless authentication.

---

## Azure Log Analytics: Audit Ingest

Export the audit log as NDJSON and push via the Data Collection Rules (DCR) API:

```bash
# Export
./scripts/export_audit.sh ndjson

# Push to DCR endpoint (replace placeholders)
curl -X POST \
  "https://<DCR_ENDPOINT>/dataCollectionRules/<DCR_RULE_ID>/streams/Custom-AGCAudit_CL?api-version=2023-01-01" \
  -H "Authorization: Bearer $(az account get-access-token --resource https://monitor.azure.com/ --query accessToken -o tsv)" \
  -H "Content-Type: application/json" \
  --data-binary @audit-$(date +%Y%m%d)*.ndjson
```

---

## Microsoft Graph: Agent App Registrations

Query the app registrations used as agent identities:

```bash
az rest \
  --method GET \
  --url "https://graph.microsoft.com/v1.0/applications?\$filter=tags/any(t:t eq 'agc-agent')" \
  --resource "https://graph.microsoft.com/"
```

Tag agent app registrations with `agc-agent` in Azure AD to enable this query.

---

## Managed Identity (Recommended for Production)

Assign a User-Assigned Managed Identity to the AGC deployment. Grant it:
- `Monitoring Metrics Publisher` on the Application Insights resource
- `Log Analytics Contributor` on the workspace

No client secrets or certificate rotation required.
