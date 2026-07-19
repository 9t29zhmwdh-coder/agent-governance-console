# Trace Schema: Agent Governance Console

## TraceSpan

AGC traces are compatible with the OpenTelemetry data model.

```json
{
  "span_id":        "uuid-v4",
  "trace_id":       "uuid-v4",
  "parent_span_id": "uuid-v4 | null",
  "agent_id":       "string: identifies the agent instance",
  "operation":      "string: e.g. 'reasoning', 'tool_call:shell', 'llm_call'",
  "level":          "debug | info | warn | error",
  "started_at":     "ISO 8601 UTC, e.g. 2026-06-16T10:00:00.000Z",
  "ended_at":       "ISO 8601 UTC | null (null if span is open)",
  "attributes": {
    "tokens_in":    1024,
    "tokens_out":   256,
    "model":        "mistral-7b",
    "tool":         "shell",
    "error":        "optional error message"
  }
}
```

## Attribute Conventions

| Key | Type | Description |
|-----|------|-------------|
| `tokens_in` | integer | Input token count (for `llm_call` spans) |
| `tokens_out` | integer | Output token count |
| `model` | string | Model identifier used for inference |
| `tool` | string | Tool name (for `tool_call:*` spans) |
| `error` | string | Error message (for `error` level spans) |
| `command` | string | Shell command (for `tool_call:shell`, privacy sensitive) |
| `url` | string | HTTP URL (for `tool_call:http`) |

## Ingestion (v0.2.0+)

```bash
curl -X POST http://127.0.0.1:8080/api/v1/traces \
  -H "Content-Type: application/json" \
  -d '{
    "span_id": "...",
    "trace_id": "...",
    "agent_id": "my-agent",
    "operation": "tool_call:shell",
    "level": "info",
    "started_at": "2026-06-16T10:00:00Z",
    "ended_at": "2026-06-16T10:00:01Z",
    "attributes": {"tool": "shell", "command": "ls /tmp"}
  }'
```
