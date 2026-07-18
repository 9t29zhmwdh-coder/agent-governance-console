//! Microsoft Sentinel analytics rule export: built-in KQL query templates
//! and ARM `Microsoft.SecurityInsights/alertRules` resource snippets for
//! the audit table `scripts/azure_setup.sh` provisions
//! (`AGCAudit_CL` by default; the table name is a parameter here since
//! `azure_setup.sh` lets an operator rename it via `AZURE_TABLE_NAME`).
//!
//! These are the same three "governance signal" categories a human
//! security analyst would ask for first: an agent repeatedly hitting the
//! policy gate, a brand-new agent whose very first recorded action is a
//! block, and a portfolio-wide spike in warn/alert volume. Correct against
//! the exact column names `azure_setup.sh`'s custom table declares
//! (`id`, `timestamp`, `agent_id`, `action`, `outcome`, `policy_id`,
//! `details`, plus Log Analytics' own `TimeGenerated`) -- not verified
//! against a live Sentinel workspace (none was available while building
//! this), same disclosed-limitation pattern as the rest of this
//! portfolio's Azure integrations.

use serde_json::json;

/// A single Sentinel analytics rule template.
#[derive(Debug, Clone)]
pub struct SentinelRule {
    pub name: &'static str,
    pub description: &'static str,
    /// One of Sentinel's four severities: Informational, Low, Medium, High.
    pub severity: &'static str,
    pub query: String,
}

impl SentinelRule {
    /// Renders this rule as a `Microsoft.SecurityInsights/alertRules`
    /// (kind `Scheduled`) ARM resource, deployable via
    /// `az deployment group create` once wrapped in a template's
    /// `resources` array.
    pub fn to_arm_resource(&self) -> serde_json::Value {
        json!({
            "type": "Microsoft.SecurityInsights/alertRules",
            "apiVersion": "2023-02-01-preview",
            "kind": "Scheduled",
            "properties": {
                "displayName": self.name,
                "description": self.description,
                "severity": self.severity,
                "enabled": true,
                "query": self.query,
                "queryFrequency": "PT15M",
                "queryPeriod": "PT1H",
                "triggerOperator": "GreaterThan",
                "triggerThreshold": 0,
                "suppressionDuration": "PT1H",
                "suppressionEnabled": false,
                "tactics": ["Impact"]
            }
        })
    }

    /// The KQL alone, as you'd paste into Sentinel's "Analytics rules ->
    /// Create -> Set rule logic" query editor.
    pub fn to_kql(&self) -> &str {
        &self.query
    }
}

/// Built-in rule set for `table` (the Log Analytics custom table name,
/// e.g. `AGCAudit_CL`).
pub fn builtin_rules(table: &str) -> Vec<SentinelRule> {
    vec![
        SentinelRule {
            name: "AGC: Repeated policy blocks by one agent",
            description: "Flags an agent that hit block-action policy rules 3 or more times within a 15-minute window -- a likely compromised, misconfigured, or actively-probing agent.",
            severity: "Medium",
            query: format!(
                "{table}\n| where outcome == \"blocked\"\n| summarize BlockCount = count(), Policies = make_set(policy_id) by agent_id, bin(TimeGenerated, 15m)\n| where BlockCount >= 3\n| project TimeGenerated, agent_id, BlockCount, Policies"
            ),
        },
        SentinelRule {
            name: "AGC: New agent's first recorded action was blocked",
            description: "Flags an agent_id whose very first audit record is already a block -- a newly deployed or newly observed agent immediately tripping governance, worth investigating before it's trusted further.",
            severity: "Medium",
            query: format!(
                "{table}\n| where outcome == \"blocked\"\n| summarize FirstBlockTime = min(TimeGenerated) by agent_id\n| join kind=inner ({table} | where outcome == \"blocked\") on agent_id\n| where TimeGenerated == FirstBlockTime\n| project TimeGenerated, agent_id, action, policy_id, details"
            ),
        },
        SentinelRule {
            name: "AGC: Spike in warned/alerted audit volume",
            description: "Flags an hour with more than 50 warned or alerted audit records portfolio-wide -- a broad anomaly (noisy policy, misbehaving deployment, or genuine incident) worth a human look.",
            severity: "Low",
            query: format!(
                "{table}\n| where outcome in (\"warned\", \"alerted\")\n| summarize EventCount = count() by bin(TimeGenerated, 1h)\n| where EventCount > 50"
            ),
        },
        SentinelRule {
            name: "AGC: Agent triggering many distinct policies",
            description: "Flags an agent matching 3 or more distinct policy_id values within an hour -- broad rule-breaking behavior across multiple governance rules at once, rather than one narrow, expected violation.",
            severity: "High",
            query: format!(
                "{table}\n| where isnotempty(policy_id)\n| summarize DistinctPolicies = dcount(policy_id) by agent_id, bin(TimeGenerated, 1h)\n| where DistinctPolicies >= 3\n| project TimeGenerated, agent_id, DistinctPolicies"
            ),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_returns_four_rules_referencing_the_given_table() {
        let rules = builtin_rules("AGCAudit_CL");
        assert_eq!(rules.len(), 4);
        for rule in &rules {
            assert!(rule.query.starts_with("AGCAudit_CL"), "query should start with the table name: {}", rule.query);
            assert!(!rule.name.is_empty());
            assert!(!rule.description.is_empty());
            assert!(matches!(rule.severity, "Informational" | "Low" | "Medium" | "High"));
        }
    }

    #[test]
    fn builtin_rules_uses_a_custom_table_name() {
        let rules = builtin_rules("MyCustomTable_CL");
        for rule in &rules {
            assert!(rule.query.contains("MyCustomTable_CL"));
            assert!(!rule.query.contains("AGCAudit_CL"));
        }
    }

    #[test]
    fn every_rule_uses_only_columns_azure_setup_sh_actually_creates() {
        // Guards against a typo referencing a column the custom table
        // (scripts/azure_setup.sh) doesn't actually have: tokenize each
        // query into identifier-like words and require every one that
        // looks like a column reference (lowercase, contains '_' or is a
        // known bare name) to be in the real schema.
        let known_columns = ["TimeGenerated", "id", "timestamp", "agent_id", "action", "outcome", "policy_id", "details"];
        let kql_keywords = [
            "where", "summarize", "count", "make_set", "by", "bin", "join", "kind", "inner", "on", "project", "min",
            "in", "isnotempty", "dcount", "AGCAudit_CL",
        ];
        for rule in builtin_rules("AGCAudit_CL") {
            for word in rule.query.split(|c: char| !c.is_alphanumeric() && c != '_') {
                let looks_like_column = word.contains('_') || known_columns.contains(&word);
                if !word.is_empty() && looks_like_column && !kql_keywords.contains(&word) {
                    assert!(
                        known_columns.contains(&word),
                        "rule '{}' references unknown column-like identifier '{word}': {}",
                        rule.name,
                        rule.query
                    );
                }
            }
        }

        // Each rule references at least "outcome" or "policy_id", the two
        // columns these governance-focused rules are actually built around.
        for rule in builtin_rules("AGCAudit_CL") {
            assert!(
                rule.query.contains("outcome") || rule.query.contains("policy_id"),
                "rule '{}' doesn't reference outcome or policy_id: {}",
                rule.name,
                rule.query
            );
        }
    }

    #[test]
    fn to_arm_resource_produces_a_valid_scheduled_alert_rule_shape() {
        let rules = builtin_rules("AGCAudit_CL");
        let arm = rules[0].to_arm_resource();
        assert_eq!(arm["type"], "Microsoft.SecurityInsights/alertRules");
        assert_eq!(arm["kind"], "Scheduled");
        assert_eq!(arm["properties"]["displayName"], rules[0].name);
        assert_eq!(arm["properties"]["query"], rules[0].query);
        assert_eq!(arm["properties"]["severity"], rules[0].severity);
        assert!(arm["properties"]["enabled"].as_bool().unwrap());
    }

    #[test]
    fn to_kql_returns_the_raw_query() {
        let rules = builtin_rules("AGCAudit_CL");
        assert_eq!(rules[0].to_kql(), rules[0].query);
    }
}
