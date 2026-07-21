# Policy DSL Reference: Agent Governance Console

## Overview

Policies are defined as YAML or JSON: one parser handles both, since
YAML 1.2 is a JSON superset (`GovernancePolicy::from_yaml`). Each policy
has a scope and a list of rules. Rules evaluate conditions against
incoming spans and trigger actions.

Three ways to load a policy:
- `POST /api/v1/policies` (JSON body); see `docs/api_reference.md`.
- Hot-reload: point `agc-api` at a directory via `AGC_POLICY_DIR`; every
  `*.yaml`/`*.yml`/`*.json` file in it (non-recursive) is loaded at
  startup and reloaded on every filesystem change. A parse error in any
  file aborts that reload and keeps the previous policy set: a bad edit
- `POST /api/v1/policies` (JSON body): see `docs/api_reference.md`.
- Hot-reload: point `agc-api` at a directory via `AGC_POLICY_DIR`; every
  `*.yaml`/`*.yml`/`*.json` file in it (non-recursive) is loaded at
  startup and reloaded on every filesystem change. A parse error in any
  file aborts that reload and keeps the previous policy set, so a bad edit
  never silently wipes a working policy set.
- `agc-cli policy validate <file>`: parses a file and reports whether
  it's valid, without needing a running server.

## Schema

```yaml
policy_id: p-shell-block
name: Block unrestricted shell access
agent_scope: []
rules:
  - rule_id: r1
    description: Block shell tool calls for all agents
    condition:
      type: operation_matches
      pattern: "tool_call:shell"
    action:
      type: block
      reason: Shell access requires explicit allowlist
```

The equivalent JSON is exactly what you'd expect (same keys, JSON
syntax) and is accepted by every loading path above too.

## Conditions

| Type | Fields | Description |
|------|--------|-------------|
| `span_level_at_least` | `level: string` | Fires when span level ≥ threshold (`debug`/`info`/`warn`/`error`) |
| `token_budget_exceeded` | `max_tokens: integer` | Fires when the span's `attributes.tokens` value exceeds `max_tokens` |
| `operation_matches` | `pattern: string` | Fires when the operation matches `pattern`; a single `*` wildcard is supported (`tool_*`), anything else requires an exact match |

## Actions

| Type | Fields | Description |
|------|--------|-------------|
| `warn` | `message: string` | Appends a `warned` audit record; the span is still ingested |
| `block` | `reason: string` | Appends a `blocked` audit record; the span is rejected with `403` and never stored |
| `alert` | `channel: string` | Appends an `alerted` audit record; external delivery (e.g. a real webhook) is a future roadmap item, not implemented yet: today this only records the decision |

## Agent Scope

```yaml
agent_scope: []               # applies to all agents
agent_scope: [agent-1]        # applies only to agent-1
```

## Rego Export (stub)

```bash
agc-cli policy to-rego my-policy.yaml
```

Renders a **structural** [Open Policy Agent](https://www.openpolicyagent.org/)
Rego module: one `package`, one `deny`/`warn`/`alert` partial rule per
policy rule, named after the rule's `rule_id`. This is a starting point
for hand-porting to real OPA evaluation, not a full semantic
translation of AGC's condition model, in particular:

- `span_level_at_least` becomes a plain string equality check
  (`input.span.level == "error"`), not a real severity-order comparison,
  since Rego has no built-in enum ordering. Port the real ordering logic
  by hand if you need it.
- `operation_matches` with a `*` wildcard becomes `regex.match(...)`
  with the glob converted to a regex; an exact-match pattern becomes a
  string equality check.
- `token_budget_exceeded` becomes a numeric comparison against
  `input.span.attributes.tokens`, assuming your OPA input document
  shapes trace spans the same way AGC does.

Example output for the schema above:

```rego
package agc.policies.p_shell_block

# Generated from AGC GovernancePolicy "p-shell-block" (Block unrestricted shell access).
# Structural stub: hand-port the condition semantics below into real
# Rego logic before using this in production OPA evaluation.

default allow = true

# Rule r1: Block shell tool calls for all agents
deny["r1"] {
    input.span.operation == "tool_call:shell"
    msg := "Shell access requires explicit allowlist"
}
```
