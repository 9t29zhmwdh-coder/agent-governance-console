use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An immutable audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub action: String,
    pub outcome: AuditOutcome,
    pub policy_id: Option<String>,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditOutcome {
    Allowed,
    Blocked,
    Warned,
    Alerted,
}

/// Append-only audit log.
#[derive(Debug, Default)]
pub struct AuditLog {
    records: Vec<AuditRecord>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, record: AuditRecord) {
        self.records.push(record);
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Export as newline-delimited JSON (NDJSON) for Azure Log Analytics ingest.
    pub fn export_ndjson(&self) -> String {
        self.records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Export as CSV (header + rows).
    pub fn export_csv(&self) -> String {
        let mut out = String::from("id,timestamp,agent_id,action,outcome,policy_id\n");
        for r in &self.records {
            out.push_str(&format!(
                "{},{},{},{},{},{}\n",
                r.id,
                r.timestamp.to_rfc3339(),
                r.agent_id,
                r.action,
                serde_json::to_string(&r.outcome).unwrap_or_default().trim_matches('"'),
                r.policy_id.as_deref().unwrap_or("")
            ));
        }
        out
    }

    pub fn records_for_agent(&self, agent_id: &str) -> Vec<&AuditRecord> {
        self.records.iter().filter(|r| r.agent_id == agent_id).collect()
    }

    pub fn blocked_records(&self) -> Vec<&AuditRecord> {
        self.records.iter().filter(|r| r.outcome == AuditOutcome::Blocked).collect()
    }
}
