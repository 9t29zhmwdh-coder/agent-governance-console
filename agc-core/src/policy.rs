use crate::trace::{TraceLevel, TraceSpan};
use serde::{Deserialize, Serialize};

/// Errors from parsing or loading policies.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("invalid policy document: {0}")]
    Parse(#[from] serde_norway::Error),
    #[error("reading policy directory: {0}")]
    Io(#[from] std::io::Error),
}

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

impl GovernancePolicy {
    /// Parses a policy from a YAML document. Since YAML 1.2 is a JSON
    /// superset, this also accepts plain JSON policy documents (the
    /// `POST /api/v1/policies` format from v0.2.0) unchanged: one parser
    /// for both, per `docs/policy_dsl.md`.
    pub fn from_yaml(s: &str) -> Result<Self, PolicyError> {
        Ok(serde_norway::from_str(s)?)
    }

    /// Serializes back to YAML, e.g. for round-tripping a policy loaded
    /// via the JSON API into a file for hot-reload.
    pub fn to_yaml(&self) -> Result<String, PolicyError> {
        Ok(serde_norway::to_string(self)?)
    }

    /// Renders a best-effort, structurally valid Rego module for this
    /// policy: real starting point for hand-porting to OPA, not a full
    /// semantic translation of AGC's condition/action model (in
    /// particular, `span_level_at_least` becomes an equality check on the
    /// literal level string here, not a real severity-order comparison,
    /// since Rego has no built-in enum ordering).
    pub fn to_rego_stub(&self) -> String {
        let package = format!("agc.policies.{}", rego_ident(&self.policy_id));
        let mut out = format!(
            "package {package}\n\n# Generated from AGC GovernancePolicy \"{}\" ({}).\n# Structural stub: hand-port the condition semantics below into real\n# Rego logic before using this in production OPA evaluation.\n\ndefault allow = true\n",
            self.policy_id, self.name
        );
        for rule in &self.rules {
            let rule_name = rego_ident(&rule.rule_id);
            let cond_expr = rule.condition.to_rego_expr();
            let (head, msg) = match &rule.action {
                PolicyAction::Block { reason } => ("deny", reason.clone()),
                PolicyAction::Warn { message } => ("warn", message.clone()),
                PolicyAction::Alert { channel } => ("alert", channel.clone()),
            };
            out.push_str(&format!(
                "\n# Rule {}: {}\n{head}[\"{rule_name}\"] {{\n    {cond_expr}\n    msg := {:?}\n}}\n",
                rule.rule_id, rule.description, msg
            ));
        }
        out
    }
}

/// Converts a policy/rule ID into a valid Rego identifier: lowercase
/// ASCII letters, digits and underscores only, never starting with a
/// digit.
fn rego_ident(s: &str) -> String {
    let mut ident: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    let needs_prefix = match ident.chars().next() {
        Some(c) => c.is_ascii_digit(),
        None => true,
    };
    if needs_prefix {
        ident.insert(0, '_');
    }
    ident
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

    /// Best-effort Rego expression for this condition; see
    /// [`GovernancePolicy::to_rego_stub`] for the caveats.
    fn to_rego_expr(&self) -> String {
        match self {
            PolicyCondition::SpanLevelAtLeast { level } => {
                format!("input.span.level == {:?}  # NOTE: equality only, not a real severity-order comparison", level.to_lowercase())
            }
            PolicyCondition::TokenBudgetExceeded { max_tokens } => {
                format!("input.span.attributes.tokens > {max_tokens}")
            }
            PolicyCondition::OperationMatches { pattern } => {
                if pattern.contains('*') {
                    let regex = format!("^{}$", regex_escape_except_star(pattern).replace('*', ".*"));
                    format!("regex.match({regex:?}, input.span.operation)")
                } else {
                    format!("input.span.operation == {pattern:?}")
                }
            }
        }
    }
}

/// Escapes Rego/regex metacharacters in a glob pattern, leaving `*` alone
/// so the caller can turn it into `.*` afterward.
fn regex_escape_except_star(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    for c in pattern.chars() {
        if c != '*' && ".+?()[]{}|^$\\".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
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

    /// Removes all loaded policies.
    pub fn clear(&mut self) {
        self.policies.clear();
    }

    /// Replaces all loaded policies with the `*.yaml`/`*.yml`/`*.json`
    /// files found directly in `dir` (non-recursive). Each file is parsed
    /// independently; the first parse error aborts the whole reload and
    /// leaves the engine's previous policy set untouched, so a single
    /// malformed file can't silently drop the rest. Returns the number of
    /// policies loaded.
    pub fn load_policies_from_dir(&mut self, dir: &std::path::Path) -> Result<usize, PolicyError> {
        let mut loaded = Vec::new();
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            let path = entry.path();
            let is_policy_file = matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("yaml") | Some("yml") | Some("json")
            );
            if !path.is_file() || !is_policy_file {
                continue;
            }
            let content = std::fs::read_to_string(&path)?;
            loaded.push(GovernancePolicy::from_yaml(&content)?);
        }
        let count = loaded.len();
        self.policies = loaded;
        Ok(count)
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

    #[test]
    fn from_yaml_parses_the_docs_policy_dsl_example() {
        let yaml = "
policy_id: p-token-budget
name: Token budget enforcement
agent_scope: []
rules:
  - rule_id: r1
    description: Alert when token budget exceeds 4096
    condition:
      type: token_budget_exceeded
      max_tokens: 4096
    action:
      type: alert
      channel: azure-monitor
";
        let policy = GovernancePolicy::from_yaml(yaml).unwrap();
        assert_eq!(policy.policy_id, "p-token-budget");
        assert_eq!(policy.rules.len(), 1);
        assert!(matches!(
            policy.rules[0].condition,
            PolicyCondition::TokenBudgetExceeded { max_tokens: 4096 }
        ));
    }

    #[test]
    fn from_yaml_also_accepts_plain_json_since_yaml_is_a_superset() {
        let json = serde_json::json!({
            "policy_id": "p1", "name": "n", "agent_scope": [],
            "rules": [{"rule_id": "r1", "description": "d",
                "condition": {"type": "operation_matches", "pattern": "tool_*"},
                "action": {"type": "warn", "message": "m"}}]
        })
        .to_string();
        let policy = GovernancePolicy::from_yaml(&json).unwrap();
        assert_eq!(policy.policy_id, "p1");
    }

    #[test]
    fn from_yaml_rejects_malformed_documents() {
        assert!(GovernancePolicy::from_yaml("not: [valid, policy").is_err());
    }

    #[test]
    fn yaml_round_trips_through_to_yaml_and_from_yaml() {
        let original = GovernancePolicy {
            policy_id: "p1".into(),
            name: "n".into(),
            agent_scope: vec!["agent-1".into()],
            rules: vec![PolicyRule {
                rule_id: "r1".into(),
                description: "d".into(),
                condition: PolicyCondition::SpanLevelAtLeast { level: "error".into() },
                action: PolicyAction::Block { reason: "x".into() },
            }],
        };
        let yaml = original.to_yaml().unwrap();
        let parsed = GovernancePolicy::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.policy_id, original.policy_id);
        assert_eq!(parsed.rules.len(), 1);
    }

    #[test]
    fn to_rego_stub_contains_package_and_one_rule_per_action_type() {
        let policy = GovernancePolicy {
            policy_id: "p-shell-block".into(),
            name: "Block shell".into(),
            agent_scope: vec![],
            rules: vec![
                PolicyRule {
                    rule_id: "r1".into(),
                    description: "block".into(),
                    condition: PolicyCondition::OperationMatches { pattern: "tool_call:shell".into() },
                    action: PolicyAction::Block { reason: "no shell".into() },
                },
                PolicyRule {
                    rule_id: "r2".into(),
                    description: "warn".into(),
                    condition: PolicyCondition::TokenBudgetExceeded { max_tokens: 100 },
                    action: PolicyAction::Warn { message: "budget".into() },
                },
                PolicyRule {
                    rule_id: "r3".into(),
                    description: "alert".into(),
                    condition: PolicyCondition::SpanLevelAtLeast { level: "error".into() },
                    action: PolicyAction::Alert { channel: "azure-monitor".into() },
                },
            ],
        };
        let rego = policy.to_rego_stub();
        assert!(rego.starts_with("package agc.policies.p_shell_block"));
        assert!(rego.contains("deny[\"r1\"]"));
        assert!(rego.contains("warn[\"r2\"]"));
        assert!(rego.contains("alert[\"r3\"]"));
        assert!(rego.contains("input.span.attributes.tokens > 100"));
    }

    #[test]
    fn rego_ident_sanitizes_hyphens_and_leading_digits() {
        assert_eq!(rego_ident("p-shell-block"), "p_shell_block");
        assert_eq!(rego_ident("1abc"), "_1abc");
        assert_eq!(rego_ident(""), "_");
    }

    #[test]
    fn load_policies_from_dir_loads_yaml_and_json_files_sorted_and_ignores_others() {
        let dir = std::env::temp_dir().join(format!("agc-policy-dir-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("b.yaml"),
            "policy_id: p-b\nname: B\nagent_scope: []\nrules: []\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("a.json"),
            r#"{"policy_id":"p-a","name":"A","agent_scope":[],"rules":[]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("README.md"), "not a policy").unwrap();

        let mut engine = PolicyEngine::new();
        let count = engine.load_policies_from_dir(&dir).unwrap();
        assert_eq!(count, 2);
        assert_eq!(engine.policy_count(), 2);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_policies_from_dir_replaces_rather_than_appends() {
        let dir = std::env::temp_dir().join(format!("agc-policy-dir-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.yaml"),
            "policy_id: p-a\nname: A\nagent_scope: []\nrules: []\n",
        )
        .unwrap();

        let mut engine = PolicyEngine::new();
        engine.load_policies_from_dir(&dir).unwrap();
        assert_eq!(engine.policy_count(), 1);

        // A second reload of the same (unchanged) directory must not double
        // the count -- this is what makes hot-reload safe to call on every
        // filesystem event instead of only once.
        engine.load_policies_from_dir(&dir).unwrap();
        assert_eq!(engine.policy_count(), 1);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn load_policies_from_dir_leaves_previous_policies_on_parse_error() {
        let dir = std::env::temp_dir().join(format!("agc-policy-dir-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bad.yaml"), "not: [valid, policy").unwrap();

        let mut engine = PolicyEngine::new();
        engine.load_policy(GovernancePolicy {
            policy_id: "existing".into(),
            name: "n".into(),
            agent_scope: vec![],
            rules: vec![],
        });

        assert!(engine.load_policies_from_dir(&dir).is_err());
        assert_eq!(engine.policy_count(), 1, "a failed reload must not clear the previous good state");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
