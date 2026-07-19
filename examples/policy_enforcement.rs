//! Example: loading a governance policy and resolving applicable rules.
//!
//! Run with: cargo run --example policy_enforcement

use agc_core::{
    AuditLog, AuditOutcome, AuditRecord,
    GovernancePolicy, PolicyAction, PolicyCondition, PolicyEngine, PolicyRule,
};
use chrono::Utc;
use uuid::Uuid;

fn main() {
    let mut engine = PolicyEngine::new();
    let mut audit  = AuditLog::new();

    // Load a policy that blocks shell tool calls for all agents
    engine.load_policy(GovernancePolicy {
        policy_id: "p-shell-block".into(),
        name: "Block unrestricted shell access".into(),
        agent_scope: vec![],
        rules: vec![PolicyRule {
            rule_id: "r1".into(),
            description: "Block shell tool calls".into(),
            condition: PolicyCondition::OperationMatches {
                pattern: "tool_call:shell".into(),
            },
            action: PolicyAction::Block {
                reason: "Unrestricted shell access is not permitted by governance policy".into(),
            },
        }],
    });

    println!("Policies loaded : {}", engine.policy_count());

    // Simulate an agent requesting shell access
    let agent_id  = "react-agent-1";
    let operation = "tool_call:shell";

    let rules = engine.applicable_rules(agent_id, operation);
    println!("Applicable rules: {}", rules.len());

    for rule in &rules {
        println!("  Rule: {}, action: {:?}", rule.rule_id, rule.action);

        // Record the block in the audit log
        audit.append(AuditRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            agent_id: agent_id.into(),
            action: operation.into(),
            outcome: AuditOutcome::Blocked,
            policy_id: Some("p-shell-block".into()),
            details: serde_json::json!({"rule_id": rule.rule_id}),
        });
    }

    println!("Audit records   : {}", audit.record_count());
    println!("Blocked records : {}", audit.blocked_records().len());

    // Export NDJSON for Azure Log Analytics
    let ndjson = audit.export_ndjson();
    println!("\nNDJSON export:\n{ndjson}");
}
