# Azure Integration Guide: Agent Governance Console

## Overview

AGC supports four Azure integration points, all opt-in and all authenticated
via **Managed Identity** — no client secret is ever configured:

| Integration | Purpose | Required AAD Permissions |
|-------------|---------|--------------------------|
| Azure Monitor (OTLP) | Telemetry span export | none required for a self-hosted OTel Collector target; `Monitoring Metrics Publisher` on the DCR if pointed at Azure Monitor's native OTLP endpoint with `AGC_TELEMETRY_MANAGED_IDENTITY` set |
| Azure Monitor Logs Ingestion | Audit record push to a DCR | `Monitoring Metrics Publisher` on the DCR |
| Microsoft Graph | Read agent app registrations | `Application.Read.All` |
| Managed Identity | Token acquisition for the two above | none (identity, not a permission) |

The Rust implementation lives in the `agc-azure` crate:
`ManagedIdentityCredential`, `MonitorIngestClient`, `GraphClient`,
`OtlpExporter`. All four are unit- and integration-tested against a local
mock HTTP server (`wiremock`); **Managed Identity's real endpoint (IMDS,
`169.254.169.254`) is only reachable from inside an Azure-hosted compute
resource**, so it cannot be exercised end-to-end from anywhere else,
including CI. `scripts/azure_setup.sh` is similarly correct-by-construction
against the documented `az` CLI/DCR contracts but has not been run against
a live subscription — see its header comment.

---

## Setup

Run `./scripts/azure_setup.sh` once, against a resource group where you
have Contributor rights:

```bash
AZURE_RG=my-rg AZURE_LOCATION=westeurope ./scripts/azure_setup.sh
```

It provisions, in order: a Log Analytics Workspace, an Application
Insights instance, a custom `AGCAudit_CL` table matching
`agc_core::AuditRecord`, a Data Collection Endpoint (DCE), a Data
Collection Rule (DCR) routing that table's stream into the workspace, and
a demo app registration tagged `agc-agent` (so `agc-cli azure list-agents`
has something real to find). It prints every value the steps below need.

Grant the compute resource running AGC (VM / App Service / Container /
AKS pod) a **system- or user-assigned managed identity**, then assign it
`Monitoring Metrics Publisher` on the DCR the script created.

---

## Telemetry: OTLP Span Export

`agc-api` exports spans over real OTLP/HTTP (via `opentelemetry-otlp`,
`agc_azure::OtlpExporter`) whenever telemetry is enabled with an endpoint:

```bash
AGC_TELEMETRY_ENDPOINT="https://<region>.otelcollector.azure.com/v1/traces" \
AGC_TELEMETRY_SERVICE_NAME="agc" \
cargo run --bin agc-api
```

`AGC_TELEMETRY_ENDPOINT` must be the **full traces endpoint URL, including
the `/v1/traces` path** — it's used exactly as given, not treated as a
base URL. A misconfigured endpoint logs a warning at startup and leaves
telemetry disabled rather than failing the whole server.

Every successfully ingested trace span (`POST /api/v1/traces`, see
`docs/api_reference.md`) is exported this way in the background; export
runs on the OTLP batch processor's own dedicated thread, never blocking
the request that triggered it.

For a self-hosted OpenTelemetry Collector in front of Application
Insights instead of Azure Monitor's native OTLP endpoint, point
`AGC_TELEMETRY_ENDPOINT` at the collector's `/v1/traces` path and use the
connection string `azure_setup.sh` prints in the collector's own Azure
Monitor exporter config.

### Managed Identity authentication for OTLP

Azure Monitor's native OTLP endpoint requires a Microsoft Entra token
(`https://monitor.azure.com/.default`, `Monitoring Metrics Publisher` on
the target DCR); a self-hosted OpenTelemetry Collector typically doesn't.
Set `AGC_TELEMETRY_MANAGED_IDENTITY=1` to fetch one via Managed Identity
(system-assigned) and send it as the export's `Authorization: Bearer`
header -- no client secret involved:

```bash
AGC_TELEMETRY_ENDPOINT="https://<region>.otelcollector.azure.com/v1/traces" \
AGC_TELEMETRY_MANAGED_IDENTITY=1 \
cargo run --bin agc-api
# or, for a user-assigned identity:
AGC_TELEMETRY_MANAGED_IDENTITY_CLIENT_ID="<client-id>" \
cargo run --bin agc-api
```

If the token fetch fails (e.g. not actually running on Azure), a warning
is logged and export proceeds without the header rather than failing
startup. **Known limitation:** the token is fetched once, at startup, and
not refreshed for the life of the process -- restart the server (or wait
for a future refresh mechanism) before a long-lived token expires.

---

## Audit Export: Azure Monitor Logs Ingestion (DCR)

Two steps: export the audit log locally, then push it to Azure with
Managed Identity authentication (no client secret anywhere in this flow).

```bash
# 1. Export from a running agc-api instance
./scripts/export_audit.sh ndjson
# -> audit-20260717-120000.ndjson

# 2. Push to the DCR azure_setup.sh created (values printed at the end of that script)
agc-cli azure push-audit \
  --file audit-20260717-120000.ndjson \
  --dce-endpoint "https://agc-dce-xxxx.westeurope-1.ingest.monitor.azure.com" \
  --dcr-id "dcr-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" \
  --stream "Custom-AGCAudit_CL"
```

`push-audit` acquires a token scoped to `https://monitor.azure.com/` via
Managed Identity, then POSTs the parsed records as a JSON array to:

```
{dce-endpoint}/dataCollectionRules/{dcr-id}/streams/{stream}?api-version=2023-01-01
```

For a user-assigned managed identity, add `--client-id <client-id>` to
either `agc-cli azure` subcommand.

### Equivalent raw curl (if you'd rather not use agc-cli)

```bash
curl -X POST \
  "https://<DCE_ENDPOINT>/dataCollectionRules/<DCR_IMMUTABLE_ID>/streams/Custom-AGCAudit_CL?api-version=2023-01-01" \
  -H "Authorization: Bearer $(az account get-access-token --resource https://monitor.azure.com/ --query accessToken -o tsv)" \
  -H "Content-Type: application/json" \
  --data-binary "[$(paste -sd, audit-20260717-120000.ndjson)]"
```

(The Logs Ingestion API expects a JSON array; the local export is NDJSON,
hence wrapping it with `paste`/`[...]` here — `agc-cli azure push-audit`
does this conversion for you.)

---

## Microsoft Graph: Agent App Registrations

```bash
agc-cli azure list-agents
```

Acquires a token scoped to `https://graph.microsoft.com/` via Managed
Identity (`Application.Read.All` permission required), then queries:

```
GET https://graph.microsoft.com/v1.0/applications?$filter=tags/any(t:t eq 'agc-agent')
```

Tag any app registration you want treated as an AGC agent identity with
`agc-agent` in Entra ID (`azure_setup.sh` does this for its demo
registration automatically).

### Equivalent raw az CLI

```bash
az rest \
  --method GET \
  --url "https://graph.microsoft.com/v1.0/applications?\$filter=tags/any(t:t eq 'agc-agent')" \
  --resource "https://graph.microsoft.com/"
```

---

## Managed Identity Details

`agc_azure::ManagedIdentityCredential` requests tokens from Azure's
Instance Metadata Service (IMDS) at `http://169.254.169.254/metadata/identity/oauth2/token`,
with a 2-second client timeout (IMDS responds in milliseconds when
present; off Azure the address is typically silently unroutable rather
than actively refused, so a short timeout matters — this was a real bug
found and fixed during development, see the v0.3.0 CHANGELOG entry).

- **System-assigned identity** (default): no configuration needed beyond
  assigning the identity to the compute resource and granting it the
  permissions in the table above.
- **User-assigned identity**: pass `--client-id <client-id>` to either
  `agc-cli azure` subcommand.

No client secret, certificate, or connection string is ever read from
AGC's own configuration for these two integrations — only the Managed
Identity token flow.
