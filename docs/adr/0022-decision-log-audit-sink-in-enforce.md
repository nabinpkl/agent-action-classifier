# ADR-0022: The decision-log audit sink lives in the `enforce` binary, fail-closed

Date: 2026-06-11
Status: Accepted

## Context
TASKS #5 is the decision-log record: the queryable, model-independent account of *what was
decided and why* that EU AI Act Art. 12 requires, and that
[ADR-0021](0021-pep-as-rust-command-hook-binary.md) explicitly deferred with the trigger "TASKS
#5". `SPEC.md` already pins the shape (`DecisionRecord`: the request, verdict, gate type, OWASP
clause, policy id, lane, rationale, `latency_ns`, and a chain-ready `prev_hash`). The open
questions were **where the sink runs** and **what happens when it cannot write**.

[ADR-0021](0021-pep-as-rust-command-hook-binary.md) moved the PEP into the Rust `enforce` binary.
That binary already holds the `Decision` and the canonical action at the moment of decision, so
routing the record back through the Python host (SPEC's original "audit sink = host" placement)
would add a round-trip for no gain.

## Decision
1. **The audit sink is part of the `enforce` binary** (`decision_record`), following the PEP into
   Rust. Given `--audit-log <path>`, it appends **one OPA/AAT-shaped JSON object per line** per
   *governed* decision. The flag is optional, so existing wiring and the demo are unchanged.
2. **The record mirrors SPEC's `DecisionRecord`.** `latency_ns` is the deterministic `decide()`
   cost, measured by the binary around the call and attached here (SPEC keeps timing off the pure
   `Decision`). `prev_hash` is null in v0.
3. **It fails CLOSED.** An audit-write failure propagates and denies the action — an unrecordable
   decision must not be silently allowed, because the record is the whole point. This matches the
   binary's existing fail-closed posture (it owns its exit code).
4. **Only governed decisions are recorded.** An out-of-scope call (ungoverned tool kind, or a path
   that maps to no scope) reaches no `decide()` and leaves no record — there is nothing to audit.

## Consequences
- **Audit-defensible at the decision layer**, model-independent, produced exactly where the verdict
  is. The OPA mapping (`allow`->Allowed, `deny`->Denied, `escalate`->Advice, error->Error) is a
  read-side concern over a stable record.
- **A misconfigured or unwritable audit log blocks governed actions** (loudly). That is the
  intended trade for a compliance record: no record, no allow. Auditing is opt-in (`--audit-log`),
  so this only bites where the operator asked for a record.
- **Ungoverned calls leave no audit trail**, by design. If "what the agent did that we *didn't*
  govern" ever needs recording, that is an observe/PostToolUse concern, not this sink.
- **Reinforces [ADR-0021](0021-pep-as-rust-command-hook-binary.md):** the per-call binary writes
  its own record; a warm-handle sidecar would centralize the sink instead (below).

## Deliberately deferred (each with a revisit trigger)
- **SHA-256 hash chain over `prev_hash`** + an append-only/tamper-evident store. Trigger: the audit
  log must be tamper-evident (PRD roadmap). The slot is present so the format does not change.
- **A central sink** (the plane aggregates records from many agents) instead of a per-agent local
  file. Trigger: the central plane serves/collects for more than one agent over a boundary (also
  the warm-handle sidecar, [ADR-0021](0021-pep-as-rust-command-hook-binary.md)).
- **Per-call provenance id.** `raw_payload_id` is still the session id placeholder; a true per-call
  id lands when the raw payload is persisted. Trigger: records must point back at an archived raw
  payload.
- **Rotation / retention.** v0 appends unbounded. Trigger: the log size or a retention policy
  matters in a real deployment.
