//! Example: ingesting trace spans and querying error spans.
//!
//! Run with: cargo run --example trace_ingestion

use agc_core::{TraceLevel, TraceSpan, TraceStore};
use chrono::Utc;
use uuid::Uuid;

fn main() {
    let mut store = TraceStore::new();
    let trace_id = Uuid::new_v4();

    // Simulate a ReAct agent execution with 3 spans
    store.ingest(TraceSpan {
        span_id: Uuid::new_v4(),
        trace_id,
        parent_span_id: None,
        agent_id: "react-agent-1".into(),
        operation: "reasoning".into(),
        level: TraceLevel::Info,
        started_at: Utc::now(),
        ended_at: Some(Utc::now()),
        attributes: serde_json::json!({"tokens_in": 512, "tokens_out": 128, "model": "mistral-7b"}),
    });

    store.ingest(TraceSpan {
        span_id: Uuid::new_v4(),
        trace_id,
        parent_span_id: None,
        agent_id: "react-agent-1".into(),
        operation: "tool_call:shell".into(),
        level: TraceLevel::Info,
        started_at: Utc::now(),
        ended_at: Some(Utc::now()),
        attributes: serde_json::json!({"tool": "shell", "command": "ls /tmp"}),
    });

    // Simulate a failed tool call
    store.ingest(TraceSpan {
        span_id: Uuid::new_v4(),
        trace_id,
        parent_span_id: None,
        agent_id: "react-agent-1".into(),
        operation: "tool_call:http".into(),
        level: TraceLevel::Error,
        started_at: Utc::now(),
        ended_at: Some(Utc::now()),
        attributes: serde_json::json!({"error": "connection refused", "url": "http://internal:9000"}),
    });

    println!("Total spans    : {}", store.span_count());
    println!("Spans in trace : {}", store.spans_for_trace(&trace_id).len());
    println!("Error spans    : {}", store.error_spans().len());

    for span in store.error_spans() {
        println!(
            "  [ERROR] op={} agent={} attrs={}",
            span.operation, span.agent_id, span.attributes
        );
    }
}
