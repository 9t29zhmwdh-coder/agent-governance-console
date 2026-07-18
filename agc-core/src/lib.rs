//! Agent Governance Console: core library.
//!
//! Provides trace ingestion, governance policy enforcement, audit logging
//! and opt-in telemetry for agentic workflow observability.

pub mod audit;
pub mod policy;
pub mod sentinel;
pub mod telemetry;
pub mod trace;

pub use audit::{AuditLog, AuditOutcome, AuditRecord};
pub use policy::{GovernancePolicy, PolicyAction, PolicyCondition, PolicyEngine, PolicyRule};
pub use sentinel::{builtin_rules as sentinel_builtin_rules, SentinelRule};
pub use telemetry::{TelemetryConfig, TelemetrySink};
pub use trace::{TraceLevel, TraceSpan, TraceStore};

/// Top-level console configuration.
#[derive(Debug, Clone, Default)]
pub struct ConsoleConfig {
    pub telemetry: TelemetryConfig,
    /// Bind address for the Axum REST API.
    pub api_bind: String,
    /// Persist audit log to file on shutdown (path, if Some).
    pub audit_export_path: Option<std::path::PathBuf>,
    /// Directory holding each tenant's SQLite-backed audit log
    /// (`{dir}/{tenant_id}.sqlite`, created lazily on that tenant's first
    /// request). `None` keeps the previous in-memory-only behavior
    /// (records vanish when the process exits).
    pub audit_db_dir: Option<std::path::PathBuf>,
}

impl ConsoleConfig {
    pub fn default_local() -> Self {
        Self {
            api_bind: "127.0.0.1:8080".into(),
            telemetry: TelemetryConfig { enabled: false, ..Default::default() },
            audit_export_path: None,
            audit_db_dir: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_span(trace_id: Uuid, level: TraceLevel, op: &str) -> TraceSpan {
        TraceSpan {
            span_id: Uuid::new_v4(),
            trace_id,
            parent_span_id: None,
            agent_id: "agent-test".into(),
            operation: op.into(),
            level,
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
            attributes: serde_json::json!({}),
        }
    }

    #[test]
    fn trace_store_ingest_and_count() {
        let mut store = TraceStore::new();
        let id = Uuid::new_v4();
        store.ingest(make_span(id, TraceLevel::Info, "tool_call"));
        store.ingest(make_span(id, TraceLevel::Error, "llm_call"));
        assert_eq!(store.span_count(), 2);
        assert_eq!(store.spans_for_trace(&id).len(), 2);
        assert_eq!(store.error_spans().len(), 1);
    }

    #[test]
    fn audit_log_export_csv_contains_header() {
        let log = AuditLog::new();
        let csv = log.export_csv();
        assert!(csv.starts_with("id,timestamp,agent_id,action,outcome,policy_id\n"));
    }

    #[test]
    fn audit_log_export_ndjson_roundtrips() {
        let mut log = AuditLog::new();
        log.append(AuditRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            agent_id: "agent-1".into(),
            action: "tool_execute".into(),
            outcome: AuditOutcome::Allowed,
            policy_id: None,
            details: serde_json::json!({"tool": "shell"}),
        });
        let ndjson = log.export_ndjson();
        let parsed: serde_json::Value = serde_json::from_str(&ndjson).unwrap();
        assert_eq!(parsed["agent_id"], "agent-1");
    }

    #[test]
    fn policy_engine_returns_applicable_rules() {
        use crate::policy::{GovernancePolicy, PolicyAction, PolicyCondition, PolicyRule};
        let mut engine = PolicyEngine::new();
        engine.load_policy(GovernancePolicy {
            policy_id: "p1".into(),
            name: "Default".into(),
            agent_scope: vec![],
            rules: vec![PolicyRule {
                rule_id: "r1".into(),
                description: "Block on error".into(),
                condition: PolicyCondition::SpanLevelAtLeast { level: "error".into() },
                action: PolicyAction::Block { reason: "Error threshold".into() },
            }],
        });
        let rules = engine.applicable_rules("any-agent", "any-op");
        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn telemetry_disabled_by_default() {
        let cfg = ConsoleConfig::default_local();
        let sink = TelemetrySink::from_config(&cfg.telemetry);
        assert!(!sink.is_enabled());
    }

    #[test]
    fn telemetry_enabled_when_configured() {
        let cfg = TelemetryConfig {
            enabled: true,
            endpoint: Some("https://example.azure.com/otlp".into()),
            service_name: "agc".into(),
            include_agent_ids: false,
        };
        let sink = TelemetrySink::from_config(&cfg);
        assert!(sink.is_enabled());
    }
}
