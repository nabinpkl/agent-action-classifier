# SPEC.md

Technical shape for `agent-action-classifier`. Scope: the **how**, at
contract-altitude. This file states contracts (data shapes, the decision interface,
budgets, invariants), not mechanism (no module layout, no function bodies, no
"X calls Y"). Mechanism lives in the code; the executable conformance corpus is the
real, drift-proof spec. See [PRD.md](PRD.md) for the what/why and [docs/adr/](docs/adr/)
for the decisions this builds on.

> Type sketches below are illustrative **contracts**, not final source. They pin field
> names and shapes precisely where prose is ambiguous; they will be refined as the
> conformance corpus forces precision.

## Stack

- **Policy Decision Point (PDP):** Rust library crate. Pure, no I/O, no async. Exposes
  one decision entry point. This is the deep module and the primary test target.
- **Host:** Python, via a PyO3 binding over the PDP. Holds all impure edges, the
  enforcement adapter (PEP), the context/approval source (PIP), the LLM judge, and the
  audit sink.
- **Build:** `cargo` for the core, `maturin` for the PyO3 wheel, `just` for workflows
  (`just check` = build + test + lint + fmt; `just bench`).
- **Policy format:** authored data (PAP). Cedar is the *reference model*; v0 uses a
  minimal declarative format and our own evaluator (adopting Cedar's engine would skip
  the systems lesson, [ADR-0001](docs/adr/0001-build-as-a-learning-project.md)). The
  in-memory `Policy` contract below is what is load-bearing; the on-disk syntax is
  free to change.

## Module boundaries (XACML P*Ps, [ADR-0005](docs/adr/0005-organization-policy-supremacy-and-authz-architecture.md))

| Role | Responsibility | Purity | Side |
|---|---|---|---|
| **PDP** | decide a verdict from action + policy + context | pure | Rust |
| **PEP** | intercept the action, map to canonical, enforce/observe the verdict | impure | Python |
| **PAP** | author and load the org policy | impure (load) | Python loads, Rust holds |
| **PIP** | supply context (scoped approvals; later, trajectory) | impure | Python |
| **Audit sink** | persist the decision record | impure | Python |
| **Judge** | semantic verdict for the escalate lane | impure (LLM) | Python |

Source-code dependencies point inward: the PDP defines the contracts it needs; the
impure edges implement them. The PDP is environment-independent by construction, which
is also what makes it polyglot-embeddable later.

## Data contracts

### Canonical action (the fixed schema)

The single most load-bearing decision: a **closed set of action variants**, not dynamic
JSON. Fixed shape buys zero-allocation matching, a cheap FFI boundary, and stable
bindings at once. Adding a new action kind is a deliberate code change, by design.

```
CanonicalAction {
  agent_id:   AgentId,        // which agent took the action
  session_id: SessionId,      // trajectory id (carried in v0 for the future stateful lane)
  seq:        u64,            // monotonic index within the session
  at:         Timestamp,
  source:     Provenance,     // provider name + opaque raw-payload id, for audit
  operation:  Operation,      // the closed variant set below
}

Operation =                   // closed enum; v0 covers the ASI05 surface
  | ShellExec    { command: String, cwd: String }
  | FileWrite    { path: String, byte_len: u64 }
  | NetworkFetch { url: String }
```

The v0 corpus shape is modeled on a real LangGraph tool-call payload, then normalized
into this struct (the PEP's job).

### Policy and rules (PAP)

```
Policy { rules: [Rule] }

Rule {
  id:        RuleId,          // stable, appears in the decision record for audit
  owasp_tag: OwaspClause,     // e.g. "ASI05"; the organizing/audit layer, not the logic
  lane:      Lane,            // Deterministic | Semantic
  match:     Matcher,         // structured predicate (deterministic) or judge-prompt (semantic)
  outcome:   Outcome,         // HardDeny | HardAllow | RequiresApproval
}
```

`HardDeny` = explicit deny (supreme, unoverridable). `HardAllow` = explicit allow.
`RequiresApproval` = an implicit deny the org delegates to scoped user approval.

### Context (PIP)

```
Context {
  approvals: [Approval],      // scoped; only resolve RequiresApproval, never HardDeny
  // trajectory window: reserved for the stateful lane, absent in v0
}

Approval {
  scope:      ApprovalScope,  // ThisAction | CommandClass(pattern) | SessionWindow(window)
  granted_by: UserId,
  expires:    Timestamp,
}
```

### Decision (PDP output)

```
Decision {
  verdict:    Verdict,        // Allow | Deny | Escalate | Flag
  gate_type:  GateType,       // Hard | Soft   (EU AI Act Art 12 distinction)
  owasp:      Option<OwaspClause>, // the clause that fired; None on the engine-default escalate (no clause matched)
  rule_id:    Option<RuleId>, // the rule that fired, if any (None alongside owasp on the default escalate)
  lane:       Lane,           // which lane resolved it
  rationale:  String,
}
```

`latency_ns` is **not** on `Decision`: measuring it inside `decide` would inject
nondeterminism into a value that must exact-match in conformance. Timing is measured by
the caller (bench/host) and attached at the decision-record layer below, not produced by
the pure core.

### The decision interface (the PDP contract)

```
decide(action: &CanonicalAction, policy: &Policy, context: &Context) -> Decision
```

Pure. Deterministic. No I/O. The escalate lane is **not** executed here, `decide`
returns `verdict = Escalate` and the host runs the judge. This keeps the core
deterministic and conformance-testable.

### Evaluation contract (precedence, [ADR-0005](docs/adr/0005-organization-policy-supremacy-and-authz-architecture.md))

For the clauses **applicable** to the action, in this order of authority:

1. Any applicable `HardDeny` matches -> `Deny`, `Hard`. (deny-overrides; supreme)
2. An applicable `RequiresApproval` with a valid in-scope `Approval` in context -> `Allow`, `Soft`; without one -> `Escalate`, `Soft`.
3. An applicable `HardAllow`, **and no higher-lane (semantic/stateful) clause applies** -> `Allow`, `Hard`.
4. An applicable semantic clause, or any remaining ambiguity -> `Escalate` (to the judge).
5. Nothing applicable / unresolved -> default `Escalate`, then **fail closed to `Deny`** if escalation does not resolve.

### Judge (host, impure)

```
judge(action, context, clause, policy_excerpt) -> JudgeVerdict { verdict, rationale, confidence }
```

Receives action + context (trajectory, intent, scoped approval). Reasons **under org
supremacy**: it decides whether the action violates org policy, with approval as a
mitigating-not-overriding factor. Nondeterministic by nature, kept out of the
exact-match conformance suite.

### Decision-log record (audit sink, OPA/AAT-shaped, chain-ready)

```
DecisionRecord {
  at, action, verdict, gate_type, owasp, rule_id, lane, rationale, latency_ns,
  prev_hash: Option<Hash>,    // chain-ready; null in v0, SHA-256 chain is roadmap
}
```

JSON. Decision-type mapping to OPA's vocabulary: `Allow->Allowed`, `Deny->Denied`,
`Escalate->Advice`, evaluation failure -> `Error`. The record is produced at the
decision layer (model-independent), which is what makes it audit-defensible.

## Evaluation approach

- **Conformance corpus = the spec for deterministic lanes.** Hand-authored
  `CanonicalAction` + `Policy` -> expected `Decision`, asserted at 100% exact-match on
  `verdict` / `owasp` / `gate_type` / `rule_id`. Same corpus drives the latency bench.
- **Judge = graded eval**, agreement target 80-90% (ref: ASSERT, human-to-human ~90%),
  never exact-match.
- **Benchmarks = reference-or-frontier** ([ADR-0006](docs/adr/0006-reference-or-frontier-measurement.md)).

## Budgets

- **Stage-1 `decide` (deterministic):** target p99 `< 100µs`; ref Microsoft `<0.1ms`
  inline. The real goal is *provably negligible* against a 500ms-2s LLM call, measured
  not assumed.
- **PyO3 FFI crossing:** measured, target negligible; ref pydantic-core / polars.
- **Judge:** bounded by LLM latency, paid only on escalation. **Escalation rate**
  (fraction reaching the judge) is a `frontier` metric.

## Known constraints

- **Closed action set:** no dynamic/arbitrary actions; a new kind is a code change.
  Deliberate, for matching speed and FFI cheapness.
- **Stateless v0:** `session_id` / `seq` / `Context` carry the seams for the stateful
  lane, but trajectory clauses (ASI03/06/08) are not implemented.
- **Framework-layer is advisory and bypassable** ([ADR-0003](docs/adr/0003-govern-at-framework-layer-defer-kernel.md)); not a hardened enforcement boundary.
- **Judge nondeterminism** is contained to the semantic lane and excluded from the
  hard spec.
- **No live enforcement in v0:** verdicts are produced and logged; acting on them
  against a live agent needs a live PEP (roadmap).
