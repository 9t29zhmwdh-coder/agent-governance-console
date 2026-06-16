use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Severity level of a trace span.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum TraceLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// A single span in an agent execution trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    pub span_id: Uuid,
    pub trace_id: Uuid,
    pub parent_span_id: Option<Uuid>,
    pub agent_id: String,
    pub operation: String,
    pub level: TraceLevel,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    /// Structured attributes (tool name, token counts, model ID, etc.)
    pub attributes: serde_json::Value,
}

impl TraceSpan {
    pub fn duration_ms(&self) -> Option<i64> {
        let end = self.ended_at?;
        Some((end - self.started_at).num_milliseconds())
    }

    pub fn is_error(&self) -> bool {
        self.level >= TraceLevel::Error
    }
}

/// In-memory trace store for ingestion and retrieval.
#[derive(Debug, Default)]
pub struct TraceStore {
    spans: Vec<TraceSpan>,
}

impl TraceStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ingest(&mut self, span: TraceSpan) {
        spans_insert_sorted(&mut self.spans, span);
    }

    pub fn spans_for_trace(&self, trace_id: &Uuid) -> Vec<&TraceSpan> {
        self.spans.iter().filter(|s| &s.trace_id == trace_id).collect()
    }

    pub fn error_spans(&self) -> Vec<&TraceSpan> {
        self.spans.iter().filter(|s| s.is_error()).collect()
    }

    pub fn span_count(&self) -> usize {
        self.spans.len()
    }
}

fn spans_insert_sorted(spans: &mut Vec<TraceSpan>, span: TraceSpan) {
    let pos = spans.partition_point(|s| s.started_at <= span.started_at);
    spans.insert(pos, span);
}
