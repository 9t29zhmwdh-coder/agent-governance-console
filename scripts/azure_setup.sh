#!/usr/bin/env bash
# azure_setup.sh: Provision Azure Monitor resources for AGC telemetry and
# audit ingestion (Log Analytics workspace, Application Insights, the
# custom audit table, its Data Collection Endpoint + Rule, and a demo
# agent app registration).
#
# Requirements: Azure CLI (az), authenticated with a principal that has
# Contributor rights on the target resource group.
#
# NOTE: written correct-by-construction against the documented az CLI and
# DCR JSON contracts, but not run end-to-end against a real Azure
# subscription (none was available while writing it, see the v0.3.0
# CHANGELOG entry). The Rust HTTP clients this feeds (agc-azure/) do have
# real, mock-server-verified tests; this script is the one piece of the
# v0.3.0 milestone that is unverified against live Azure. If a step's az
# CLI syntax has since changed, please open an issue.
#
# Usage: AZURE_RG=my-rg AZURE_LOCATION=westeurope ./scripts/azure_setup.sh
set -euo pipefail

RG="${AZURE_RG:?Set AZURE_RG}"
LOCATION="${AZURE_LOCATION:-westeurope}"
WORKSPACE_NAME="${AZURE_WORKSPACE_NAME:-agc-log-analytics}"
APP_INSIGHTS_NAME="${AZURE_APP_INSIGHTS_NAME:-agc-app-insights}"
DCE_NAME="${AZURE_DCE_NAME:-agc-dce}"
DCR_NAME="${AZURE_DCR_NAME:-agc-audit-dcr}"
TABLE_NAME="${AZURE_TABLE_NAME:-AGCAudit_CL}"
APP_REGISTRATION_NAME="${AZURE_APP_REGISTRATION_NAME:-agc-agent-demo}"

echo "=== AGC Azure Setup ==="
echo "Resource group : $RG"
echo "Location       : $LOCATION"

# 1. Log Analytics Workspace
echo -e "\n[1/6] Creating Log Analytics Workspace: $WORKSPACE_NAME"
az monitor log-analytics workspace create \
  --resource-group "$RG" \
  --workspace-name "$WORKSPACE_NAME" \
  --location "$LOCATION" \
  --output table

WORKSPACE_ID=$(az monitor log-analytics workspace show \
  --resource-group "$RG" \
  --workspace-name "$WORKSPACE_NAME" \
  --query id -o tsv)

# 2. Application Insights, connected to the workspace (classic exporter
# path; the OTLP path in docs/azure_integration.md doesn't need this, but
# it's kept for anyone using the connection-string-based config).
echo -e "\n[2/6] Creating Application Insights: $APP_INSIGHTS_NAME"
az monitor app-insights component create \
  --app "$APP_INSIGHTS_NAME" \
  --location "$LOCATION" \
  --resource-group "$RG" \
  --workspace "$WORKSPACE_ID" \
  --output table

CONN_STRING=$(az monitor app-insights component show \
  --app "$APP_INSIGHTS_NAME" \
  --resource-group "$RG" \
  --query connectionString -o tsv)

# 3. Custom table for AGC audit records, matching agc_core::AuditRecord.
echo -e "\n[3/6] Creating custom table: ${TABLE_NAME}"
az monitor log-analytics workspace table create \
  --resource-group "$RG" \
  --workspace-name "$WORKSPACE_NAME" \
  --name "$TABLE_NAME" \
  --columns \
    TimeGenerated=datetime \
    id=guid \
    timestamp=datetime \
    agent_id=string \
    action=string \
    outcome=string \
    policy_id=string \
    details=dynamic \
  --output table

# 4. Data Collection Endpoint (DCE): the ingestion URL agc-cli's
# `push-audit` command (and the raw curl example below) post to.
echo -e "\n[4/6] Creating Data Collection Endpoint: $DCE_NAME"
az monitor data-collection endpoint create \
  --resource-group "$RG" \
  --name "$DCE_NAME" \
  --location "$LOCATION" \
  --public-network-access Enabled \
  --output table

DCE_ID=$(az monitor data-collection endpoint show \
  --resource-group "$RG" \
  --name "$DCE_NAME" \
  --query id -o tsv)
DCE_ENDPOINT=$(az monitor data-collection endpoint show \
  --resource-group "$RG" \
  --name "$DCE_NAME" \
  --query logsIngestion.endpoint -o tsv)

# 5. Data Collection Rule (DCR): routes the "Custom-<table>" stream into
# the workspace table created in step 3.
echo -e "\n[5/6] Creating Data Collection Rule: $DCR_NAME"
DCR_BODY_FILE="$(mktemp)"
cat > "$DCR_BODY_FILE" <<JSON
{
  "location": "$LOCATION",
  "properties": {
    "dataCollectionEndpointId": "$DCE_ID",
    "streamDeclarations": {
      "Custom-${TABLE_NAME}": {
        "columns": [
          {"name": "TimeGenerated", "type": "datetime"},
          {"name": "id", "type": "string"},
          {"name": "timestamp", "type": "string"},
          {"name": "agent_id", "type": "string"},
          {"name": "action", "type": "string"},
          {"name": "outcome", "type": "string"},
          {"name": "policy_id", "type": "string"},
          {"name": "details", "type": "dynamic"}
        ]
      }
    },
    "destinations": {
      "logAnalytics": [
        {"workspaceResourceId": "$WORKSPACE_ID", "name": "agcWorkspace"}
      ]
    },
    "dataFlows": [
      {
        "streams": ["Custom-${TABLE_NAME}"],
        "destinations": ["agcWorkspace"],
        "outputStream": "Custom-${TABLE_NAME}"
      }
    ]
  }
}
JSON
az monitor data-collection rule create \
  --resource-group "$RG" \
  --name "$DCR_NAME" \
  --rule-file "$DCR_BODY_FILE" \
  --output table
rm -f "$DCR_BODY_FILE"

DCR_IMMUTABLE_ID=$(az monitor data-collection rule show \
  --resource-group "$RG" \
  --name "$DCR_NAME" \
  --query immutableId -o tsv)

# 6. Demo agent app registration, tagged so `agc-cli azure list-agents`
# (Microsoft Graph) has something real to find.
echo -e "\n[6/6] Creating demo app registration: $APP_REGISTRATION_NAME"
az ad app create \
  --display-name "$APP_REGISTRATION_NAME" \
  --tags "agc-agent" \
  --output table

echo -e "\n=== Done ==="
echo "Telemetry (OTLP span export):"
echo "  AGC_TELEMETRY_ENDPOINT=<Azure Monitor OTLP endpoint, see docs/azure_integration.md>"
echo "  AGC_TELEMETRY_SERVICE_NAME=agc"
echo "  (classic Application Insights connection string, if not using OTLP: $CONN_STRING)"
echo ""
echo "Audit export (Logs Ingestion API via Managed Identity, no client secret):"
echo "  agc-cli azure push-audit --file audit.ndjson \\"
echo "    --dce-endpoint $DCE_ENDPOINT \\"
echo "    --dcr-id $DCR_IMMUTABLE_ID \\"
echo "    --stream Custom-${TABLE_NAME}"
echo ""
echo "Agent app registrations (Microsoft Graph):"
echo "  agc-cli azure list-agents"
