use serde::{Deserialize, Serialize};

/// Governance policy for an agent or agent group.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GovernancePolicy {
    pub policy_id: String,
    /// Human-readable name.
    pub name: String,
    /// Agent IDs this policy applies to (empty = applies to all).
    pub agent_scope: Vec<String>,
    pub rules: Vec<PolicyRule>,
}

/// A single policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub rule_id: String,
    pub description: String,
    pub condition: PolicyCondition,
    pub action: PolicyAction,
}

/// Conditions that trigger a rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyCondition {
    /// Triggered when a span's level meets or exceeds the threshold.
    SpanLevelAtLeast { level: String },
    /// Triggered when token count exceeds a threshold.
    TokenBudgetExceeded { max_tokens: u64 },
    /// Triggered when operation name matches a pattern.
    OperationMatches { pattern: String },
}

/// Actions taken when a rule fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyAction {
    /// Log a warning to the audit trail.
    Warn { message: String },
    /// Block the operation and return an error.
    Block { reason: String },
    /// Emit an alert (webhook / Azure Monitor stub).
    Alert { channel: String },
}

/// Policy evaluation engine.
#[derive(Debug, Default)]
pub struct PolicyEngine {
    policies: Vec<GovernancePolicy>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_policy(&mut self, policy: GovernancePolicy) {
        self.policies.push(policy);
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    /// Returns matching rules for a given agent and operation (stub: full eval in v0.2.0).
    pub fn applicable_rules(&self, agent_id: &str, _operation: &str) -> Vec<&PolicyRule> {
        self.policies
            .iter()
            .filter(|p| p.agent_scope.is_empty() || p.agent_scope.iter().any(|a| a == agent_id))
            .flat_map(|p| p.rules.iter())
            .collect()
    }
}
