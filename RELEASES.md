# Release Guide: Agent Governance Console

## Creating the Initial Import Release (v0.1.0)

Replace `<EARLIEST_COMMIT_DATE>` and `<INITIAL_SHA>` with actual values.

### Step 1: Tag the initial commit

```bash
gh api repos/9t29zhmwdh-coder/agent-governance-console/git/refs \
  --method POST \
  -f ref="refs/tags/v0.1.0" \
  -f sha="<INITIAL_SHA>"
```

### Step 2: Create the GitHub Release

```bash
gh release create v0.1.0 \
  --repo 9t29zhmwdh-coder/agent-governance-console \
  --title "v0.1.0: Initial import" \
  --notes "Initial import, earliest commit date: <EARLIEST_COMMIT_DATE>

Governance, tracing, policy enforcement and observability for agentic workflows.
Trace ingestion, audit log (NDJSON/CSV), policy engine stubs, opt-in Azure Monitor telemetry." \
  --prerelease
```

### Step 3: Verify

```bash
gh release list --repo 9t29zhmwdh-coder/agent-governance-console
```

---

## Release Checklist for Future Versions

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo audit` clean
- [ ] `CHANGELOG.md` updated
- [ ] Version bumped in `Cargo.toml` (workspace root)
- [ ] PR merged to `main`
- [ ] Tag created and release published
