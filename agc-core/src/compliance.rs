//! Compliance report export, aligned with Microsoft's six Responsible AI
//! principles (Fairness, Reliability and Safety, Privacy and Security,
//! Inclusiveness, Transparency, Accountability -- see
//! <https://learn.microsoft.com/azure/machine-learning/concept-responsible-ai>).
//!
//! AGC's own data (governance policies, the audit log, trace spans) only
//! speaks directly to four of the six: Accountability, Transparency,
//! Reliability and Safety, and Privacy and Security. Fairness and
//! Inclusiveness require observing an AI system's actual outputs against
//! protected attributes or user diversity -- data this governance/audit
//! tool never collects and structurally cannot report on. The report says
//! so explicitly rather than silently omitting them, see `docs/compliance.md`.

use crate::audit::AuditOutcome;
use crate::{AuditLog, PolicyEngine, TraceStore};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Facts about the running deployment's security posture that
/// `agc-core` itself has no way to observe (RBAC and Managed Identity
/// live in `agc-api`/`agc-azure`, outside this dependency-light core
/// crate) but that belong in a Privacy and Security report section.
/// The caller (`agc-api`) fills this in from its own `AppState`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SecurityPosture {
    pub rbac_enabled: bool,
    pub telemetry_managed_identity: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct OutcomeCounts {
    pub allowed: usize,
    pub blocked: usize,
    pub warned: usize,
    pub alerted: usize,
}

impl OutcomeCounts {
    fn total(&self) -> usize {
        self.allowed + self.blocked + self.warned + self.alerted
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceReport {
    pub tenant_id: String,
    pub generated_at: DateTime<Utc>,
    /// Timestamps of the oldest and newest audit records, if any exist.
    pub period: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub policy_count: usize,
    pub total_audit_records: usize,
    pub outcomes: OutcomeCounts,
    /// `(policy_id, decision_count)`, sorted by count descending.
    pub records_by_policy: Vec<(String, usize)>,
    pub total_spans: usize,
    pub error_spans: usize,
    /// Agents that hit a block-action rule 3 or more times -- the same
    /// "repeated policy blocks" signal `agc-core::sentinel` flags for
    /// Sentinel, reused here for the Reliability and Safety section.
    /// `(agent_id, block_count)`, sorted by count descending.
    pub repeated_block_agents: Vec<(String, usize)>,
    pub security: SecurityPosture,
}

impl ComplianceReport {
    /// Builds a report from a tenant's own trace/audit stores plus the
    /// (global, shared) policy engine. `security` comes from the caller,
    /// see `SecurityPosture`'s doc comment for why.
    pub fn generate(tenant_id: &str, audit: &AuditLog, trace: &TraceStore, policy: &PolicyEngine, security: SecurityPosture) -> Self {
        let records = audit.all_records();

        let period = match (records.first(), records.last()) {
            (Some(first), Some(last)) => Some((first.timestamp, last.timestamp)),
            _ => None,
        };

        let mut outcomes = OutcomeCounts::default();
        let mut by_policy: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut blocks_by_agent: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for record in &records {
            match record.outcome {
                AuditOutcome::Allowed => outcomes.allowed += 1,
                AuditOutcome::Blocked => {
                    outcomes.blocked += 1;
                    *blocks_by_agent.entry(record.agent_id.clone()).or_insert(0) += 1;
                }
                AuditOutcome::Warned => outcomes.warned += 1,
                AuditOutcome::Alerted => outcomes.alerted += 1,
            }
            if let Some(policy_id) = &record.policy_id {
                *by_policy.entry(policy_id.clone()).or_insert(0) += 1;
            }
        }

        let mut records_by_policy: Vec<(String, usize)> = by_policy.into_iter().collect();
        records_by_policy.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let mut repeated_block_agents: Vec<(String, usize)> =
            blocks_by_agent.into_iter().filter(|(_, count)| *count >= 3).collect();
        repeated_block_agents.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        Self {
            tenant_id: tenant_id.to_string(),
            generated_at: Utc::now(),
            period,
            policy_count: policy.policy_count(),
            total_audit_records: records.len(),
            outcomes,
            records_by_policy,
            total_spans: trace.span_count(),
            error_spans: trace.error_spans().len(),
            repeated_block_agents,
            security,
        }
    }

    /// Renders the report as Markdown, suitable for handing to a
    /// compliance/audit team or attaching to a review.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# AGC Compliance Report: {}\n\n", self.tenant_id));
        out.push_str(&format!("Generated: {}\n\n", self.generated_at.to_rfc3339()));
        match self.period {
            Some((start, end)) => out.push_str(&format!("Audit period: {} to {} ({} record(s))\n\n", start.to_rfc3339(), end.to_rfc3339(), self.total_audit_records)),
            None => out.push_str("Audit period: no audit records for this tenant yet.\n\n"),
        }
        out.push_str("Aligned with [Microsoft's Responsible AI principles](https://learn.microsoft.com/azure/machine-learning/concept-responsible-ai). Covers the four principles AGC's governance/audit data can actually attest to; see \"Out of scope\" below for the other two.\n\n");

        out.push_str("## Accountability\n\n");
        out.push_str(&format!("- {} governance polic{} loaded and enforced.\n", self.policy_count, if self.policy_count == 1 { "y" } else { "ies" }));
        out.push_str(&format!("- {} audit decision(s) recorded: {} allowed, {} blocked, {} warned, {} alerted.\n", self.outcomes.total(), self.outcomes.allowed, self.outcomes.blocked, self.outcomes.warned, self.outcomes.alerted));
        if self.records_by_policy.is_empty() {
            out.push_str("- No policy rule has matched any recorded action yet.\n\n");
        } else {
            out.push_str("- Decisions by policy:\n\n");
            for (policy_id, count) in &self.records_by_policy {
                out.push_str(&format!("  - `{policy_id}`: {count}\n"));
            }
            out.push('\n');
        }

        out.push_str("## Transparency\n\n");
        out.push_str("- Every governed decision above is individually traceable: agent, action, matched policy, timestamp, and outcome are all recorded in the tenant's audit log (`GET /api/v1/audit`, `/audit/export.ndjson`, `/audit/export.csv`).\n");
        out.push_str(&format!("- {} of {} decisions matched an explicit policy rule; the rest were allowed with no rule in scope.\n\n", self.records_by_policy.iter().map(|(_, c)| c).sum::<usize>(), self.outcomes.total()));

        out.push_str("## Reliability and Safety\n\n");
        out.push_str(&format!("- {} trace span(s) ingested, {} at error level ({:.1}%).\n", self.total_spans, self.error_spans, error_rate_pct(self.error_spans, self.total_spans)));
        if self.repeated_block_agents.is_empty() {
            out.push_str("- No agent has hit 3 or more policy blocks; no repeated-failure pattern detected.\n\n");
        } else {
            out.push_str("- Agents with 3 or more policy blocks (a likely compromised, misconfigured, or actively-probing agent):\n\n");
            for (agent_id, count) in &self.repeated_block_agents {
                out.push_str(&format!("  - `{agent_id}`: {count} blocks\n"));
            }
            out.push('\n');
        }

        out.push_str("## Privacy and Security\n\n");
        out.push_str(&format!("- Tenant isolation: this report covers only tenant `{}`; trace and audit data never cross tenant boundaries (see ARCHITECTURE.md).\n", self.tenant_id));
        out.push_str(&format!("- REST API access control (RBAC): {}.\n", if self.security.rbac_enabled { "enabled" } else { "disabled -- every request is treated as Admin" }));
        out.push_str(&format!("- Outbound telemetry authentication: {}.\n\n", if self.security.telemetry_managed_identity { "Managed Identity token attached to OTLP export" } else { "no Managed Identity token attached to OTLP export (either telemetry is off, or Managed Identity wasn't requested/available)" }));

        out.push_str("## Out of scope: Fairness, Inclusiveness\n\n");
        out.push_str("AGC governs and audits *agent behavior* (what actions ran, whether a policy allowed them); it does not observe the *content* an underlying AI model generates, so it cannot assess whether outputs treat different groups fairly or serve a diverse range of users -- that requires model-level evaluation tooling (e.g. Azure AI Foundry's fairness and safety evaluators), not a governance/audit layer like this one.\n");
        out
    }
}

fn error_rate_pct(errors: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (errors as f64 / total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditOutcome, AuditRecord};
    use crate::trace::{TraceLevel, TraceSpan};
    use uuid::Uuid;

    fn record(agent_id: &str, outcome: AuditOutcome, policy_id: Option<&str>) -> AuditRecord {
        AuditRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            agent_id: agent_id.to_string(),
            action: "tool_call".into(),
            outcome,
            policy_id: policy_id.map(|s| s.to_string()),
            details: serde_json::json!({}),
        }
    }

    fn error_span(agent_id: &str) -> TraceSpan {
        TraceSpan {
            span_id: Uuid::new_v4(),
            trace_id: Uuid::new_v4(),
            parent_span_id: None,
            agent_id: agent_id.to_string(),
            operation: "llm_call".into(),
            level: TraceLevel::Error,
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
            attributes: serde_json::json!({}),
        }
    }

    #[test]
    fn generate_counts_outcomes_and_policy_hits_correctly() {
        let mut audit = AuditLog::new();
        audit.append(record("agent-1", AuditOutcome::Allowed, None));
        audit.append(record("agent-1", AuditOutcome::Blocked, Some("p1")));
        audit.append(record("agent-2", AuditOutcome::Warned, Some("p2")));
        audit.append(record("agent-2", AuditOutcome::Blocked, Some("p1")));

        let trace = TraceStore::new();
        let policy = PolicyEngine::new();
        let report = ComplianceReport::generate("tenant-a", &audit, &trace, &policy, SecurityPosture::default());

        assert_eq!(report.total_audit_records, 4);
        assert_eq!(report.outcomes.allowed, 1);
        assert_eq!(report.outcomes.blocked, 2);
        assert_eq!(report.outcomes.warned, 1);
        assert_eq!(report.outcomes.alerted, 0);
        assert_eq!(report.records_by_policy, vec![("p1".to_string(), 2), ("p2".to_string(), 1)]);
    }

    #[test]
    fn generate_flags_agents_with_three_or_more_blocks() {
        let mut audit = AuditLog::new();
        for _ in 0..3 {
            audit.append(record("noisy-agent", AuditOutcome::Blocked, Some("p1")));
        }
        audit.append(record("quiet-agent", AuditOutcome::Blocked, Some("p1")));

        let trace = TraceStore::new();
        let policy = PolicyEngine::new();
        let report = ComplianceReport::generate("tenant-a", &audit, &trace, &policy, SecurityPosture::default());

        assert_eq!(report.repeated_block_agents, vec![("noisy-agent".to_string(), 3)]);
    }

    #[test]
    fn generate_reports_span_error_rate() {
        let audit = AuditLog::new();
        let mut trace = TraceStore::new();
        trace.ingest(error_span("agent-1"));
        trace.ingest(error_span("agent-1"));
        let policy = PolicyEngine::new();
        let report = ComplianceReport::generate("tenant-a", &audit, &trace, &policy, SecurityPosture::default());

        assert_eq!(report.total_spans, 2);
        assert_eq!(report.error_spans, 2);
    }

    #[test]
    fn generate_with_no_data_reports_an_empty_period() {
        let audit = AuditLog::new();
        let trace = TraceStore::new();
        let policy = PolicyEngine::new();
        let report = ComplianceReport::generate("tenant-a", &audit, &trace, &policy, SecurityPosture::default());

        assert!(report.period.is_none());
        assert_eq!(report.total_audit_records, 0);
    }

    #[test]
    fn to_markdown_mentions_all_four_covered_principles_and_discloses_the_two_out_of_scope() {
        let mut audit = AuditLog::new();
        audit.append(record("agent-1", AuditOutcome::Blocked, Some("p1")));
        let trace = TraceStore::new();
        let policy = PolicyEngine::new();
        let security = SecurityPosture { rbac_enabled: true, telemetry_managed_identity: false };
        let md = ComplianceReport::generate("tenant-a", &audit, &trace, &policy, security).to_markdown();

        for heading in ["## Accountability", "## Transparency", "## Reliability and Safety", "## Privacy and Security", "## Out of scope: Fairness, Inclusiveness"] {
            assert!(md.contains(heading), "missing section {heading}");
        }
        assert!(md.contains("tenant-a"));
        assert!(md.contains("enabled"), "should reflect that RBAC is enabled in this report");
    }

    #[test]
    fn error_rate_pct_handles_zero_spans_without_dividing_by_zero() {
        assert_eq!(error_rate_pct(0, 0), 0.0);
    }
}
