use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditOutcome {
    Allowed,
    Blocked,
    Warned,
    Alerted,
}

impl AuditOutcome {
    fn as_str(self) -> &'static str {
        match self {
            AuditOutcome::Allowed => "allowed",
            AuditOutcome::Blocked => "blocked",
            AuditOutcome::Warned => "warned",
            AuditOutcome::Alerted => "alerted",
        }
    }

    fn parse(s: &str) -> rusqlite::Result<Self> {
        match s {
            "allowed" => Ok(AuditOutcome::Allowed),
            "blocked" => Ok(AuditOutcome::Blocked),
            "warned" => Ok(AuditOutcome::Warned),
            "alerted" => Ok(AuditOutcome::Alerted),
            other => Err(rusqlite::Error::InvalidColumnType(
                4,
                format!("unknown audit outcome '{other}'"),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

/// Append-only audit log, backed by SQLite so records survive a process
/// restart. `AuditLog::new()` uses an in-memory database (matching the
/// previous Vec-backed behavior, and what tests/examples want); `open()`
/// points at a real file for actual persistence.
pub struct AuditLog {
    conn: Connection,
}

impl AuditLog {
    pub fn new() -> Self {
        let conn = Connection::open_in_memory().expect("opening in-memory SQLite connection");
        Self::from_connection(conn).expect("creating audit_records table")
    }

    /// Opens (or creates) a SQLite-backed audit log at `path`. Existing
    /// records from a previous run are preserved and included in every
    /// query, not just newly appended ones.
    pub fn open(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_records (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                action TEXT NOT NULL,
                outcome TEXT NOT NULL,
                policy_id TEXT,
                details TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_audit_records_agent_id ON audit_records(agent_id);
            CREATE INDEX IF NOT EXISTS idx_audit_records_outcome ON audit_records(outcome);",
        )?;
        Ok(Self { conn })
    }

    pub fn append(&mut self, record: AuditRecord) {
        self.try_append(&record).expect("audit log append failed");
    }

    /// Fallible counterpart to `append`, for callers that want to handle a
    /// write failure (e.g. a full disk) instead of panicking.
    pub fn try_append(&mut self, record: &AuditRecord) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO audit_records (id, timestamp, agent_id, action, outcome, policy_id, details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.id.to_string(),
                record.timestamp.to_rfc3339(),
                record.agent_id,
                record.action,
                record.outcome.as_str(),
                record.policy_id,
                record.details.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn record_count(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM audit_records", [], |row| row.get::<_, i64>(0))
            .unwrap_or(0) as usize
    }

    fn query_records(&self, where_clause: &str, param: Option<&str>) -> Vec<AuditRecord> {
        let sql = format!(
            "SELECT id, timestamp, agent_id, action, outcome, policy_id, details
             FROM audit_records {where_clause} ORDER BY timestamp ASC"
        );
        let Ok(mut stmt) = self.conn.prepare(&sql) else { return vec![] };
        let mapped = match param {
            Some(p) => stmt.query_map(params![p], row_to_record),
            None => stmt.query_map([], row_to_record),
        };
        match mapped {
            Ok(rows) => rows.filter_map(Result::ok).collect(),
            Err(_) => vec![],
        }
    }

    fn all_records(&self) -> Vec<AuditRecord> {
        self.query_records("", None)
    }

    /// Export as newline-delimited JSON (NDJSON) for Azure Log Analytics ingest.
    pub fn export_ndjson(&self) -> String {
        self.all_records()
            .iter()
            .map(|r| serde_json::to_string(r).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Export as CSV (header + rows).
    pub fn export_csv(&self) -> String {
        let mut out = String::from("id,timestamp,agent_id,action,outcome,policy_id\n");
        for r in self.all_records() {
            out.push_str(&format!(
                "{},{},{},{},{},{}\n",
                r.id,
                r.timestamp.to_rfc3339(),
                r.agent_id,
                r.action,
                r.outcome.as_str(),
                r.policy_id.as_deref().unwrap_or("")
            ));
        }
        out
    }

    pub fn records_for_agent(&self, agent_id: &str) -> Vec<AuditRecord> {
        self.query_records("WHERE agent_id = ?1", Some(agent_id))
    }

    pub fn blocked_records(&self) -> Vec<AuditRecord> {
        self.query_records("WHERE outcome = ?1", Some(AuditOutcome::Blocked.as_str()))
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

fn row_to_record(row: &Row) -> rusqlite::Result<AuditRecord> {
    let id: String = row.get(0)?;
    let timestamp: String = row.get(1)?;
    let outcome: String = row.get(4)?;
    let details: String = row.get(6)?;

    Ok(AuditRecord {
        id: Uuid::parse_str(&id).map_err(|e| {
            rusqlite::Error::InvalidColumnType(0, e.to_string(), rusqlite::types::Type::Text)
        })?,
        timestamp: DateTime::parse_from_rfc3339(&timestamp)
            .map(|t| t.with_timezone(&Utc))
            .map_err(|e| {
                rusqlite::Error::InvalidColumnType(1, e.to_string(), rusqlite::types::Type::Text)
            })?,
        agent_id: row.get(2)?,
        action: row.get(3)?,
        outcome: AuditOutcome::parse(&outcome)?,
        policy_id: row.get(5)?,
        details: serde_json::from_str(&details).unwrap_or(serde_json::Value::Null),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(agent_id: &str, outcome: AuditOutcome) -> AuditRecord {
        AuditRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            agent_id: agent_id.to_string(),
            action: "tool_execute".to_string(),
            outcome,
            policy_id: Some("p1".to_string()),
            details: serde_json::json!({"tool": "shell"}),
        }
    }

    #[test]
    fn append_and_count_roundtrip() {
        let mut log = AuditLog::new();
        log.append(sample("agent-1", AuditOutcome::Allowed));
        log.append(sample("agent-1", AuditOutcome::Blocked));
        assert_eq!(log.record_count(), 2);
    }

    #[test]
    fn records_for_agent_filters_correctly() {
        let mut log = AuditLog::new();
        log.append(sample("agent-1", AuditOutcome::Allowed));
        log.append(sample("agent-2", AuditOutcome::Allowed));
        let matches = log.records_for_agent("agent-1");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].agent_id, "agent-1");
    }

    #[test]
    fn blocked_records_filters_by_outcome() {
        let mut log = AuditLog::new();
        log.append(sample("agent-1", AuditOutcome::Allowed));
        log.append(sample("agent-1", AuditOutcome::Blocked));
        log.append(sample("agent-2", AuditOutcome::Blocked));
        assert_eq!(log.blocked_records().len(), 2);
    }

    #[test]
    fn export_ndjson_roundtrips_through_json() {
        let mut log = AuditLog::new();
        log.append(sample("agent-1", AuditOutcome::Warned));
        let ndjson = log.export_ndjson();
        let parsed: AuditRecord = serde_json::from_str(&ndjson).unwrap();
        assert_eq!(parsed.agent_id, "agent-1");
        assert_eq!(parsed.outcome, AuditOutcome::Warned);
    }

    #[test]
    fn export_csv_has_header_and_row() {
        let mut log = AuditLog::new();
        log.append(sample("agent-1", AuditOutcome::Alerted));
        let csv = log.export_csv();
        assert!(csv.starts_with("id,timestamp,agent_id,action,outcome,policy_id\n"));
        assert!(csv.contains("agent-1"));
        assert!(csv.contains("alerted"));
    }

    #[test]
    fn records_survive_reopening_a_file_backed_log() {
        let dir = std::env::temp_dir().join(format!("agc-audit-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("audit.sqlite");

        {
            let mut log = AuditLog::open(&db_path).unwrap();
            log.append(sample("agent-1", AuditOutcome::Allowed));
        }
        // Reopen: a fresh AuditLog over the same file should see the prior record,
        // proving persistence actually survives the connection (and process) going away.
        let reopened = AuditLog::open(&db_path).unwrap();
        assert_eq!(reopened.record_count(), 1);
        // Windows locks open files; the connection must close before the
        // directory can be removed, unlike on POSIX where an unlinked file
        // stays deletable while still open.
        drop(reopened);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ordering_is_by_timestamp_ascending() {
        let mut log = AuditLog::new();
        let mut first = sample("agent-1", AuditOutcome::Allowed);
        first.timestamp = Utc::now() - chrono::Duration::seconds(10);
        let mut second = sample("agent-1", AuditOutcome::Allowed);
        second.timestamp = Utc::now();
        // Insert out of chronological order to prove the query sorts, not the insert order.
        log.append(second.clone());
        log.append(first.clone());
        let records = log.records_for_agent("agent-1");
        assert_eq!(records[0].id, first.id);
        assert_eq!(records[1].id, second.id);
    }
}
