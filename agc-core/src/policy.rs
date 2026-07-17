use crate::trace::{TraceLevel, TraceSpan};
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

impl PolicyCondition {
    /// Evaluates this condition against a real span. `TokenBudgetExceeded`
    /// reads the `tokens` key from `span.attributes` (the convention this
    /// engine expects callers to populate); missing or non-numeric
    /// attributes never match rather than erroring.
    pub fn matches(&self, span: &TraceSpan) -> bool {
        match self {
            PolicyCondition::SpanLevelAtLeast { level } => parse_level(level)
                .map(|threshold| span.level >= threshold)
                .unwrap_or(false),
            PolicyCondition::TokenBudgetExceeded { max_tokens } => span
                .attributes
                .get("tokens")
                .and_then(|v| v.as_u64())
                .map(|tokens| tokens > *max_tokens)
                .unwrap_or(false),
            PolicyCondition::OperationMatches { pattern } => glob_match(pattern, &span.operation),
        }
    }
}

fn parse_level(s: &str) -> Option<TraceLevel> {
    match s.to_lowercase().as_str() {
        "debug" => Some(TraceLevel::Debug),
        "info" => Some(TraceLevel::Info),
        "warn" => Some(TraceLevel::Warn),
        "error" => Some(TraceLevel::Error),
        _ => None,
    }
}

/// Minimal single-wildcard glob match (only `*` is special, no other glob
/// syntax): `tool_*` matches `tool_call`, `*_error` matches `llm_error`,
/// `tool_*_call` matches `tool_shell_call`. A pattern without `*` requires
/// an exact match.
fn glob_match(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == text;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        match text[pos..].find(part) {
            Some(found) => {
                if i == 0 && found != 0 {
                    return false;
                }
                pos += found + part.len();
            }
            None => return false,
        }
    }
    match parts.last() {
        Some(last) if !last.is_empty() => text.ends_with(last),
        _ => true,
    }
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

    /// Returns rules whose policy is in scope for `agent_id`, regardless of
    /// whether their condition currently matches anything. Useful for
    /// listing what *could* fire; for the real-time gate on ingestion, use
    /// [`PolicyEngine::evaluate`] instead.
    pub fn applicable_rules(&self, agent_id: &str, _operation: &str) -> Vec<&PolicyRule> {
        self.policies
            .iter()
            .filter(|p| p.agent_scope.is_empty() || p.agent_scope.iter().any(|a| a == agent_id))
            .flat_map(|p| p.rules.iter())
            .collect()
    }

    /// Real-time policy gate: returns every rule, across all in-scope
    /// policies, whose condition actually matches `span`. Returns
    /// `(policy_id, rule)` pairs (cloned, so callers aren't tied to this
    /// engine's lock/lifetime) in policy-then-rule declaration order; if
    /// any matched rule is a `Block`, the caller is expected to reject the
    /// span (see `agc-api`'s trace ingestion handler).
    pub fn evaluate(&self, span: &TraceSpan) -> Vec<(String, PolicyRule)> {
        self.policies
            .iter()
            .filter(|p| {
                p.agent_scope.is_empty() || p.agent_scope.iter().any(|a| a == &span.agent_id)
            })
            .flat_map(|p| {
                p.rules
                    .iter()
                    .filter(|r| r.condition.matches(span))
                    .map(move |r| (p.policy_id.clone(), r.clone()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn span(agent_id: &str, level: TraceLevel, operation: &str, attrs: serde_json::Value) -> TraceSpan {
        TraceSpan {
            span_id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            parent_span_id: None,
            agent_id: agent_id.into(),
            operation: operation.into(),
            level,
            started_at: chrono::Utc::now(),
            ended_at: None,
            attributes: attrs,
        }
    }

    #[test]
    fn span_level_at_least_matches_higher_and_equal_levels() {
        let cond = PolicyCondition::SpanLevelAtLeast { level: "warn".into() };
        assert!(cond.matches(&span("a", TraceLevel::Warn, "op", serde_json::json!({}))));
        assert!(cond.matches(&span("a", TraceLevel::Error, "op", serde_json::json!({}))));
        assert!(!cond.matches(&span("a", TraceLevel::Info, "op", serde_json::json!({}))));
    }

    #[test]
    fn span_level_at_least_rejects_unknown_level_string() {
        let cond = PolicyCondition::SpanLevelAtLeast { level: "critical".into() };
        assert!(!cond.matches(&span("a", TraceLevel::Error, "op", serde_json::json!({}))));
    }

    #[test]
    fn token_budget_exceeded_reads_tokens_attribute() {
        let cond = PolicyCondition::TokenBudgetExceeded { max_tokens: 1000 };
        assert!(cond.matches(&span("a", TraceLevel::Info, "op", serde_json::json!({"tokens": 1500}))));
        assert!(!cond.matches(&span("a", TraceLevel::Info, "op", serde_json::json!({"tokens": 500}))));
        assert!(!cond.matches(&span("a", TraceLevel::Info, "op", serde_json::json!({}))));
    }

    #[test]
    fn operation_matches_supports_wildcard_glob() {
        let cond = PolicyCondition::OperationMatches { pattern: "tool_*".into() };
        assert!(cond.matches(&span("a", TraceLevel::Info, "tool_call", serde_json::json!({}))));
        assert!(!cond.matches(&span("a", TraceLevel::Info, "llm_call", serde_json::json!({}))));

        let exact = PolicyCondition::OperationMatches { pattern: "llm_call".into() };
        assert!(exact.matches(&span("a", TraceLevel::Info, "llm_call", serde_json::json!({}))));
        assert!(!exact.matches(&span("a", TraceLevel::Info, "llm_call_v2", serde_json::json!({}))));
    }

    #[test]
    fn evaluate_only_returns_actually_matching_rules() {
        let mut engine = PolicyEngine::new();
        engine.load_policy(GovernancePolicy {
            policy_id: "p1".into(),
            name: "Default".into(),
            agent_scope: vec![],
            rules: vec![
                PolicyRule {
                    rule_id: "r-error".into(),
                    description: "Block on error".into(),
                    condition: PolicyCondition::SpanLevelAtLeast { level: "error".into() },
                    action: PolicyAction::Block { reason: "too severe".into() },
                },
                PolicyRule {
                    rule_id: "r-tools".into(),
                    description: "Warn on tool calls".into(),
                    condition: PolicyCondition::OperationMatches { pattern: "tool_*".into() },
                    action: PolicyAction::Warn { message: "tool call".into() },
                },
            ],
        });

        let matches = engine.evaluate(&span("agent-1", TraceLevel::Info, "tool_call", serde_json::json!({})));
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.rule_id, "r-tools");
    }

    #[test]
    fn evaluate_respects_agent_scope() {
        let mut engine = PolicyEngine::new();
        engine.load_policy(GovernancePolicy {
            policy_id: "p1".into(),
            name: "Scoped".into(),
            agent_scope: vec!["agent-1".into()],
            rules: vec![PolicyRule {
                rule_id: "r1".into(),
                description: "Warn always".into(),
                condition: PolicyCondition::OperationMatches { pattern: "*".into() },
                action: PolicyAction::Warn { message: "hi".into() },
            }],
        });

        assert_eq!(engine.evaluate(&span("agent-1", TraceLevel::Info, "op", serde_json::json!({}))).len(), 1);
        assert_eq!(engine.evaluate(&span("agent-2", TraceLevel::Info, "op", serde_json::json!({}))).len(), 0);
    }
}
