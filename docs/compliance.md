# Compliance Report Guide: Agent Governance Console

## Overview

`GET /api/v1/compliance/report` generates a report aligned with
[Microsoft's six Responsible AI principles](https://learn.microsoft.com/azure/machine-learning/concept-responsible-ai)
(Fairness, Reliability and Safety, Privacy and Security, Inclusiveness,
Transparency, Accountability) from a tenant's own real trace/audit data
-- not a template, an actual computed summary of what happened.

**AGC's data only speaks directly to four of the six principles.** It
governs and audits *agent behavior* -- what actions ran, which policy
rule matched, whether the action was allowed, blocked, warned about, or
alerted on. It never observes the *content* an underlying AI model
generates, so it structurally cannot assess Fairness (do outputs treat
different groups equitably?) or Inclusiveness (do outputs serve a
diverse range of users?). The report says this explicitly, in its own
"Out of scope" section, rather than silently omitting the two principles
or claiming coverage it doesn't have.

| Principle | Covered? | What the report shows |
|-----------|----------|------------------------|
| Accountability | Yes | Policies loaded, decision counts by outcome and by policy |
| Transparency | Yes | Every decision is individually traceable; % of decisions that matched an explicit policy rule |
| Reliability and Safety | Yes | Trace span error rate; agents with 3+ repeated policy blocks |
| Privacy and Security | Yes | Tenant isolation, RBAC status, OTLP Managed Identity status |
| Fairness | No | Requires model-output-level evaluation (e.g. Azure AI Foundry's fairness evaluators), not a governance/audit layer |
| Inclusiveness | No | Same as Fairness |

---

## Usage

```bash
# Markdown (default) -- ready to hand to a compliance/audit team as-is
curl -H "X-Tenant-Id: acme" http://127.0.0.1:8080/api/v1/compliance/report

# JSON -- for programmatic consumption
curl -H "X-Tenant-Id: acme" "http://127.0.0.1:8080/api/v1/compliance/report?format=json"
```

Tenant-scoped like every other trace/audit endpoint (`X-Tenant-Id`
required, `400` if missing). With RBAC enabled, requires at least the
`Viewer` role, same as every other `GET` endpoint.

### Report sections

- **Accountability**: policy count, decision counts (allowed/blocked/warned/alerted), decisions broken down by `policy_id`.
- **Transparency**: a statement that every decision is individually traceable via the audit log, plus the fraction of decisions that matched an explicit policy rule vs. passed through unmatched.
- **Reliability and Safety**: trace span error rate (`error_spans / total_spans`), and any agent that hit 3 or more policy blocks -- the same "repeated policy blocks" signal `agc-core::sentinel` flags for Sentinel (see `docs/sentinel.md`), reused here.
- **Privacy and Security**: confirms tenant isolation (this report only ever covers the requesting tenant), whether RBAC is enabled, and whether the OTLP telemetry export is Managed-Identity-authenticated (see `docs/azure_integration.md`).
- **Out of scope**: the Fairness/Inclusiveness disclosure above, verbatim in every report.

---

## What's verified vs. not

Real, tested: `agc-core::compliance` has 6 unit tests covering outcome
counting, policy-decision grouping, the repeated-block-agent threshold,
span error-rate math (including the zero-spans case), the empty-tenant
case, and that every section heading (including the out-of-scope
disclosure) actually appears in the rendered Markdown. The REST endpoint
itself has 2 integration tests: one drives a real policy through 3 real
blocked requests and checks the exact numbers the report produces in
both Markdown and JSON, the other checks the missing-tenant-header
rejection.

Not verified: this report has not been reviewed by a compliance or
legal professional, and is not a substitute for one -- it's a factual
summary of what AGC's own governance data recorded, not a certification
of regulatory compliance.
