#!/usr/bin/env bash
# azure_setup.sh — Provision Azure Monitor resources for AGC telemetry
#
# Requirements: Azure CLI (az), authenticated with a principal that has
# Contributor rights on the target resource group.
#
# Usage: AZURE_RG=my-rg AZURE_LOCATION=westeurope ./scripts/azure_setup.sh
set -euo pipefail

RG="${AZURE_RG:?Set AZURE_RG}"
LOCATION="${AZURE_LOCATION:-westeurope}"
WORKSPACE_NAME="${AZURE_WORKSPACE_NAME:-agc-log-analytics}"
APP_INSIGHTS_NAME="${AZURE_APP_INSIGHTS_NAME:-agc-app-insights}"

echo "=== AGC Azure Setup ==="
echo "Resource group : $RG"
echo "Location       : $LOCATION"

# 1. Log Analytics Workspace
echo -e "\n[1/3] Creating Log Analytics Workspace: $WORKSPACE_NAME"
az monitor log-analytics workspace create \
  --resource-group "$RG" \
  --workspace-name "$WORKSPACE_NAME" \
  --location "$LOCATION" \
  --output table

# 2. Application Insights (connected to workspace)
WORKSPACE_ID=$(az monitor log-analytics workspace show \
  --resource-group "$RG" \
  --workspace-name "$WORKSPACE_NAME" \
  --query id -o tsv)

echo -e "\n[2/3] Creating Application Insights: $APP_INSIGHTS_NAME"
az monitor app-insights component create \
  --app "$APP_INSIGHTS_NAME" \
  --location "$LOCATION" \
  --resource-group "$RG" \
  --workspace "$WORKSPACE_ID" \
  --output table

# 3. Retrieve OTLP endpoint for TelemetryConfig
CONN_STRING=$(az monitor app-insights component show \
  --app "$APP_INSIGHTS_NAME" \
  --resource-group "$RG" \
  --query connectionString -o tsv)

echo -e "\n[3/3] Done. Add to your AGC config:"
echo "---"
echo "[telemetry]"
echo "enabled = true"
echo "endpoint = \"$CONN_STRING\""
echo "service_name = \"agc\""
echo "include_agent_ids = false"
echo "---"
