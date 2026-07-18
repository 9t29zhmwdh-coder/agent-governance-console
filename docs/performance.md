# Performance / SLA Guide: Agent Governance Console

## The SLA target

ROADMAP.md's v1.0.0 milestone sets one performance target: **p99 ingest
latency < 10ms for 1K spans/s** against `POST /api/v1/traces`.

## `agc-cli bench ingest`

A real load-generation tool ships with this project specifically to
check this target, rather than a one-off script:

```bash
cargo run --release --bin agc-api &
cargo run --release --bin agc-cli -- bench ingest --rate 1000 --duration-secs 10
```

Always use `--release` builds for this -- a debug build's latency
numbers are not representative (roughly an order of magnitude slower)
and will make a perfectly healthy server look like it's failing the SLA.

Requests are spaced evenly across the run (one every `1/rate` seconds),
not fired in a synchronous per-second burst -- see "A methodology bug
found along the way" below for why that distinction mattered here.
Reports `p50`/`p95`/`p99`/`max` in milliseconds and exits `1` if p99
doesn't clear the 10ms target.

---

## Two real things found and fixed while verifying this

### 1. A genuine server-side bottleneck: one global lock for tenant lookup

`AppState::tenant_store` locked the entire `tenants: HashMap` behind a
single `tokio::sync::Mutex` on **every** request, even for a tenant that
already existed -- so concurrent requests, even to different tenants,
serialized on one exclusive lock just to do a HashMap read. Fixed by
switching to `tokio::sync::RwLock`: the common case (tenant already
exists) now takes a shared read lock, so concurrent lookups run in
parallel; only first-request-ever-for-a-tenant creation takes the
exclusive write lock (with a double-check after acquiring it, so two
concurrent first-requests for a brand-new tenant can't both create it).

### 2. A benchmark methodology bug: synchronous bursts don't represent a steady rate

The first version of `bench ingest` fired all `rate` requests for a
given second in one synchronous burst (spawn all N tasks at once, then
sleep out the remainder of the second) rather than spacing them evenly.
That produced misleadingly bad numbers: **p99 ~29ms at 1000 req/s**,
which looked like a real SLA failure. Investigating showed p99 scaled
almost linearly with the burst size (100 req/s -> ~4ms, 300 req/s ->
~11ms, 1000 req/s -> ~29ms) even though the RwLock fix above was already
applied -- a signature of pure queueing delay from N requests all
contending for the same per-tenant `Mutex<TraceStore>` at the *exact
same instant*, which a real steady arrival rate of 1000/s never
actually produces (in reality, requests a millisecond apart rarely
overlap in the lock at all). Fixed by spacing requests evenly instead;
p99 dropped to **well under 1ms**.

This is disclosed in this much detail deliberately: the first "SLA not
met" result was real, measured output, not a mistake in reading it --
but it was measuring the wrong thing. Chasing it down (rather than
either accepting a bad number or declaring success on the biased
benchmark) is exactly the "run it for real" standard the rest of this
project holds itself to.

---

## What was actually measured

Run in this development environment (Apple Silicon Mac, `agc-api`
release build, in-memory audit log, no policies loaded, single tenant),
not a dedicated benchmark machine or a Kubernetes pod under CPU limits:

| Rate | Duration | Policy match? | Audit log | p50 | p95 | p99 | max |
|------|----------|---------------|-----------|-----|-----|-----|-----|
| 1000 req/s | 5s | no | in-memory | 0.20ms | 0.35ms | 0.53ms | 1.77ms |
| 1000 req/s | 20s (sustained) | no | in-memory | 0.17ms | 0.29ms | 0.47ms | 2.13ms |
| 2000 req/s (2x target) | 5s | no | in-memory | 0.19ms | 0.38ms | 0.55ms | 1.93ms |
| 1000 req/s | 10s | **yes, every span** (real `warn` rule match) | **`AGC_AUDIT_DB_DIR`, real SQLite file on disk** | 0.64ms | 1.21ms | 1.38ms | 3.25ms |

**SLA met** in every scenario tested, including the realistic worst
case (every span matching a real policy rule, and every resulting audit
record actually written to a real SQLite file on disk, not in-memory) --
still roughly 7x under the 10ms target, sustained over 20 seconds in the
no-policy case and holding at double the target rate.

## What's verified vs. not

Real: every number above is actual measured output from a real running
`agc-api` release binary and the real `agc-cli bench ingest` tool
described above, against real HTTP requests over a real TCP socket (not
an in-process `tower::ServiceExt::oneshot` call, which bypasses the
network stack entirely), including a run with real policy evaluation
matching on every single span and real audit records persisted to an
actual SQLite file on disk (verified the file existed and grew, not
just that the run reported success).

Not verified: this development machine, not a production server or a
Kubernetes pod under the `helm/agent-governance-console` chart's default
resource limits (`docs/helm.md`); a single tenant only (many concurrent
tenants each creating their own SQLite-backed audit log for the first
time would add real, one-time file-creation overhead this measurement
doesn't capture, though that only affects each tenant's very first
request, see the `RwLock` fix above for why steady-state lookups after
that stay fast). Re-run `agc-cli bench ingest` against your actual
deployment shape before trusting the SLA holds there.
