# Copilot Instructions for Agent Governance Console

Agent Governance Console (AGC) is a Rust workspace (`agc-core`, `agc-api`, `agc-cli`, `agc-azure`) for tracing, policy enforcement and audit logging of AI agent activity, with real Azure Monitor telemetry/audit export, Microsoft Graph integration, and Microsoft Sentinel analytics rule export.

## Code style
- Functions stay small and single-purpose, prefer under 20 lines
- Naming: verb+noun for functions, clear intent for variables, no x/temp/data
- Constants in UPPER_SNAKE_CASE
- Comments explain WHY, never WHAT
- No speculative abstractions: solve the task at hand

## Text and documentation
- Never use em-dash, en-dash, or a spaced hyphen as a sentence-break substitute, anywhere. Rephrase instead
- README.md and README.de.md must stay in sync: same structure, same sections, both languages updated together
- Any functional change needs a CHANGELOG.md entry and follows semantic versioning: patch for fixes/docs, minor for new features, major for breaking changes
- No separate License badge in README (intentional project convention, not an omission)

## Git workflow
- This repo enforces branch protection on main: no direct pushes, no force pushes, a pull request is required for every change
- Semantic commit messages: type(scope): description (feat, fix, security, refactor, test, docs)
- One commit = one logical change

## Security
- Never commit secrets, API keys, or tokens
- Validate input at actual boundaries, do not add defensive checks for cases that cannot occur internally
- Flag anything that looks like a security regression instead of silently working around it

## Before opening a PR
- Run the existing test suite and build; do not open a PR with failing checks
- Keep the diff scoped to the task described in the issue, no unrelated refactors
