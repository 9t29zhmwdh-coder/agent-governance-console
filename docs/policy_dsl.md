# Policy DSL Reference: Agent Governance Console

## Overview

Policies are defined as JSON (v0.1.0) or YAML (planned v0.4.0). Each policy has a scope and a list of rules. Rules evaluate conditions against incoming spans and trigger actions.

## JSON Schema (v0.1.0)

```json
{
  "policy_id": "p-shell-block",
  "name": "Block unrestricted shell access",
  "agent_scope": [],
  "rules": [
    {
      "rule_id": "r1",
      "description": "Block shell tool calls for all agents",
      "condition": {
        "type": "operation_matches",
        "pattern": "tool_call:shell"
      },
      "action": {
        "type": "block",
        "reason": "Shell access requires explicit allowlist"
      }
    }
  ]
}
```

## Conditions

| Type | Fields | Description |
|------|--------|-------------|
| `span_level_at_least` | `level: string` | Fires when span level ≥ threshold |
| `token_budget_exceeded` | `max_tokens: integer` | Fires when `tokens_in + tokens_out` > max |
| `operation_matches` | `pattern: string` | Fires when operation equals pattern (exact, v0.1) |

## Actions

| Type | Fields | Description |
|------|--------|-------------|
| `warn` | `message: string` | Appends a warn record to audit log |
| `block` | `reason: string` | Appends a blocked record; API returns 403 |
| `alert` | `channel: string` | Stub: logs alert; webhook delivery in v0.3.0 |

## Agent Scope

```json
"agent_scope": []               // applies to all agents
"agent_scope": ["agent-1"]      // applies only to agent-1
```

## YAML DSL (v0.4.0 preview)

```yaml
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
```
