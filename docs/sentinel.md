# Microsoft Sentinel Export Guide: Agent Governance Console

## Overview

`agc-cli sentinel export` turns AGC's audit table into ready-to-use
Microsoft Sentinel analytics rule templates. It ships 4 built-in rules,
the same "governance signal" categories a human security analyst would
ask for first:

| Rule | Severity | Flags |
|------|----------|-------|
| Repeated policy blocks by one agent | Medium | An agent hitting block-action policy rules 3+ times within a 15-minute window |
| New agent's first recorded action was blocked | Medium | An `agent_id` whose very first audit record is already a block |
| Spike in warned/alerted audit volume | Low | An hour with more than 50 warned/alerted audit records portfolio-wide |
| Agent triggering many distinct policies | High | An agent matching 3+ distinct `policy_id` values within an hour |

The Rust implementation lives in `agc-core::sentinel`: `SentinelRule`,
`builtin_rules(table)`, `to_kql()`, `to_arm_resource()`. All 4 rules are
correct against the exact column names `scripts/azure_setup.sh`'s custom
table declares (`id`, `timestamp`, `agent_id`, `action`, `outcome`,
`policy_id`, `details`, plus Log Analytics' own `TimeGenerated`); a
dedicated test tokenizes every query and rejects any column-like
identifier not in that real schema. **Not verified against a live
Sentinel workspace** (none was available while building this), same
Sentinel workspace** (none was available while building this), the same
disclosed-limitation pattern as the rest of this portfolio's Azure
integrations, see `docs/azure_integration.md`.

---

## Usage

```bash
# One .kql file per rule, ready to paste into Sentinel's
# "Analytics rules -> Create -> Set rule logic" query editor
agc-cli sentinel export --table AGCAudit_CL --format kql --output-dir ./sentinel-rules

# A single ARM template deploying all 4 rules as
# Microsoft.SecurityInsights/alertRules resources
agc-cli sentinel export --table AGCAudit_CL --format arm --output-dir ./sentinel-rules
az deployment group create --resource-group my-rg --template-file ./sentinel-rules/agc-sentinel-rules.json
```

`--table` defaults to `AGCAudit_CL` (what `scripts/azure_setup.sh`
provisions); pass a different value if you renamed the table via that
script's `AZURE_TABLE_NAME`. `--output-dir` defaults to the current
directory and is created if it doesn't exist.

### ARM resource shape

Each rule renders as a `Microsoft.SecurityInsights/alertRules` resource,
`kind: Scheduled`, `apiVersion: 2023-02-01-preview`: 15-minute query
frequency, 1-hour lookback period, `triggerOperator: GreaterThan`,
`triggerThreshold: 0`, 1-hour suppression window (disabled by default).
Adjust these in the deployed rule via the Sentinel portal or Azure CLI
after import if a different cadence fits your workspace.

---

## What's verified vs. not

Real, tested end-to-end: `builtin_rules()`'s shape and column references
(5 unit tests in `agc-core::sentinel`), and the CLI itself (`sentinel
export` actually writing real KQL text and valid, well-formed ARM JSON
to disk for both formats, plus its error path for an unknown `--format`
and a custom `--table` name, checked by hand against the written files,
not just that the command exits `0`).

Not verified: the queries have not been run against a live Sentinel
workspace, so their KQL syntax is correct-by-construction against the
documented schema and Kusto Query Language reference, not confirmed by
Sentinel actually accepting and firing them.
