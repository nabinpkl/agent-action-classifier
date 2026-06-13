# SPEC.md

Technical shape for `agent-action-classifier`. Scope: the **how**, at
contract-altitude. This file states contracts (data shapes, the decision interface,
budgets, invariants), not mechanism (no module layout, no function bodies). Mechanism
lives in the code; the executable conformance corpus is the real, drift-proof spec. See
[PRD.md](PRD.md) for the what/why and [docs/adr/](docs/adr/) for the decisions this builds
on, especially [ADR-0017](docs/adr/0017-adopt-cedar-engine-org-modeled-central-plane.md)
(Cedar engine + org-first model), which supersedes the earlier hand-rolled-engine and
`Operation`-enum shape.

> Type sketches below are illustrative **contracts**, not final source. They pin field
> names and shapes where prose is ambiguous; they are refined as the conformance corpus
> forces precision.

## Stack

- **Policy Decision Point (PDP):** the embedded **Cedar** engine (`cedar-policy` Rust
  crate, [ADR-0017](docs/adr/0017-adopt-cedar-engine-org-modeled-central-plane.md)).
  Cedar evaluates an authorization request (principal, action, resource, context) against
  the org's policies and entity hierarchy and returns allow/deny. It is the deterministic
  decision and the primary latency target. **Gate:** a latency spike must confirm embedded
  eval meets the budget below before the legacy hand-rolled core is removed.
- **Host:** Python, wrapping the PDP and holding the impure edges — the **central plane**
  (PAP: org graph + Cedar policy store), the **per-agent hook** (PEP), the
  context/approval source (PIP), the LLM **judge**, and the audit sink.
- **Build:** `cargo` for the core + Cedar, `maturin` for the PyO3 wheel if the PDP is
  embedded in-process, `just` for workflows (`just check`, `just bench`).
- **Policy language:** **Cedar** (adopted, not a reference model;
  [ADR-0017](docs/adr/0017-adopt-cedar-engine-org-modeled-central-plane.md)). Policies and
  the entity hierarchy are authored on the central plane.

## Module boundaries (XACML P*Ps, [ADR-0005](docs/adr/0005-organization-policy-supremacy-and-authz-architecture.md))

| Role | Responsibility | Side |
|---|---|---|
| **PDP** | decide allow/deny from request + policies + entities (Cedar) | Cedar engine |
| **PEP** | each agent's PreToolUse hook: intercept the tool call, resolve effective policy, enforce | `enforce` binary |
| **PAP** | the central plane: author + hold the org graph and Cedar policies; cascade by node | host |
| **PIP** | supply context (scoped approvals; later, trajectory) | host |
| **Audit sink** | persist the decision record | host |
| **Judge** | semantic verdict for the escalate lane | host (LLM) |

Source-code dependencies point inward: the decision uses Cedar; the impure edges
(hooks, plane, context, audit) implement what the decision needs.

The **PEP is realized as `enforce`**, a Rust command-hook binary
([ADR-0021](docs/adr/0021-pep-as-rust-command-hook-binary.md)), not the Python host: one
artifact serves both Claude and Codex (their PreToolUse payloads converged), it normalizes the
payload to the canonical action and `decide`s, and it returns allow (exit 0) / deny (exit 2 +
reason) / ask. Deny and allow are cross-provider; **ask is provider-specific** — Claude's
`permissionDecision:"ask"` dialog, but Codex (which rejects that schema) and an unknown provider
degrade an escalate to an exit-2 block ([ADR-0024](docs/adr/0024-escalate-is-provider-specific-codex-degrades-to-block.md)).
It governs **file mutations** (the path resolves to a
`DataScope`) and **shell commands** (Bash normalizes to `execute`, the command line classified
host-side into `context.command.kind`, gated by Cedar rules;
[ADR-0023](docs/adr/0023-host-derives-attributes-cedar-decides.md)). It **fails closed** on
internal error, and on a governed call whose config map is missing (the binary owns its exit
code). Per-call cost is ~5ms measured (process spawn + plane compile + ~15µs decide);
the warm-handle HTTP sidecar is the roadmap for when per-call rate or policy-set size makes that
material. `codex exec` fires no hooks (interactive Codex only); the Python host remains for the
programmatic FFI and the future judge.

## Data contracts

### Authorization request (the canonical action)

The tool call, normalized into the four axes Cedar evaluates. Replaces the placeholder
`Operation` enum.

```
Request {
  principal: Agent,         // the agent, with its inheritance chain to user/team/org
  action:    Action,        // the tool-call kind (e.g. read, write, share, invoke)
  resource:  DataScope,     // what data/scope the call touches, with attributes
  context:   Context,       // scoped approvals; later, trajectory
  at:        Timestamp,
  source:    Provenance,    // provider/runtime + opaque raw-payload id, for audit
}
```

The principal carries the **inheritance chain** (agent ∈ user ∈ team ∈ org); the resource
is a **data scope** with attributes (tenant, sensitivity, region, etc.). MCP tool
descriptions are an input that helps populate `action`/`resource`, not a replacement for
this schema. The action set is small and closed; the resource/scope space is open and
attributed (ABAC), which is why Cedar (entity hierarchy + ABAC) is the fit.

### Org model and policies (PAP)

The central plane holds two things Cedar consumes:

```
Entities {                  // the org graph
  org, team, role, user, agent nodes;
  parent edges (agent in user in team in org) = inheritance;
  attributes per entity (data-scope membership, tenant, ...).
}

Policies [                  // Cedar policies, each annotated
  permit/forbid (principal, action, resource) when { ... };
  @owasp("ASI05") @id("...") @lane("deterministic"|"semantic") ...
]
```

A policy attached at a node applies to everything beneath it (propagation = inheritance).
`forbid` overrides `permit`; an unmatched request is denied (Cedar-native default-deny),
which *is* the org-supremacy authority model
([ADR-0005](docs/adr/0005-organization-policy-supremacy-and-authz-architecture.md)).
"Requires approval" is an *implicit* deny the org delegates to scoped approval.

### Context (PIP)

```
Context {
  approvals: [Approval],    // scoped; only resolve requires-approval, never an explicit forbid
  // trajectory window: reserved for the stateful lane, absent in v0
}

Approval {
  scope:      ApprovalScope, // ThisCall | Class(pattern) | SessionWindow(window)
  granted_by: UserId,
  expires:    Timestamp,
}
```

### Decision (PDP output, host-shaped)

Cedar returns allow/deny; the host wraps that into the audit-bearing decision the cascade
and log need:

```
Decision {
  verdict:    Verdict,        // Allow | Deny | Escalate | Flag
  gate_type:  GateType,       // Hard | Soft   (EU AI Act Art 12 distinction)
  owasp:      Option<Clause>, // from the deciding policy's annotation; None on default-deny
  policy_id:  Option<Id>,     // the deciding policy, if any
  lane:       Lane,           // Deterministic (Cedar) | Semantic (judge)
  rationale:  String,
}
```

`latency_ns` is **not** on `Decision`: it is nondeterministic and would break exact-match
conformance. Timing is measured by the caller and attached at the decision-record layer.

### The decision interface (the PDP contract)

```
decide(request, policies, entities, context) -> Decision
```

Deterministic for the Cedar lane. The escalate lane is **not** executed here — `decide`
returns `verdict = Escalate` and the host runs the judge — which keeps the deterministic
core conformance-testable.

### Evaluation contract (precedence, [ADR-0005](docs/adr/0005-organization-policy-supremacy-and-authz-architecture.md))

Cedar-native, with the cascade layered on:

1. An applicable `forbid` -> `Deny`, `Hard` (deny-overrides; supreme).
2. A requires-approval policy with a valid in-scope `Approval` -> `Allow`, `Soft`; without one -> `Escalate`, `Soft`.
3. An applicable `permit`, **and no higher-lane (semantic/stateful) clause applies** -> `Allow`, `Hard`.
4. A semantic clause, or remaining ambiguity -> `Escalate` (to the judge).
5. No applicable policy -> Cedar default-deny; the host treats it as `Escalate`, then **fails closed to `Deny`** if escalation does not resolve.

### Judge (host, impure)

```
judge(request, context, clause, policy_excerpt) -> JudgeVerdict { verdict, rationale, confidence }
```

Reasons **under org supremacy**: decides whether the tool call violates org policy, with
approval as a mitigating-not-overriding factor. Nondeterministic; excluded from exact-match
conformance.

### Decision-log record (audit sink, OPA/AAT-shaped, chain-ready)

```
DecisionRecord {
  at, request, verdict, gate_type, owasp, policy_id, lane, rationale, latency_ns,
  prev_hash: Option<Hash>,    // chain-ready; null in v0, SHA-256 chain is roadmap
}
```

JSON. OPA mapping: `Allow->Allowed`, `Deny->Denied`, `Escalate->Advice`, evaluation failure
-> `Error`. Produced at the decision layer (model-independent), which makes it
audit-defensible. Realized in the `enforce` binary
([ADR-0022](docs/adr/0022-decision-log-audit-sink-in-enforce.md)): given `--audit-log <path>`
it appends one record per *governed* decision (JSON lines), with `latency_ns` measured around
`decide`. The write **fails closed** — an unrecordable decision is denied, not silently allowed.

## Evaluation approach

- **Conformance corpus = the spec for the deterministic lane.** External JSON: an org model
  (entities) + Cedar policies + authored requests -> expected keys, asserted at 100%
  exact-match on `verdict` / `gate_type` / `owasp` / `policy_id`. The same corpus drives the
  latency bench. It now tests the Cedar integration (mapping + policy + inheritance
  resolution), not a bespoke engine.
- **Judge = graded eval**, agreement target 80-90% (ref ASSERT, human-to-human ~90%), never
  exact-match.
- **Benchmarks = reference-or-frontier** ([ADR-0006](docs/adr/0006-reference-or-frontier-measurement.md)).

## Budgets

- **Deterministic decide (Cedar):** target p99 `< 100µs`; ref Microsoft `<0.1ms` inline,
  and Cedar's own ~single-digit-µs embedded eval. The real goal is *provably negligible*
  against a 500ms-2s LLM call, measured not assumed. Cedar eval scales with policy/entity
  count; the spike measures the actual policy shape.
- **Host/FFI crossing:** measured, target negligible; ref pydantic-core / polars.
- **Judge:** bounded by LLM latency, paid only on escalation. **Escalation rate** is a
  `frontier` metric.

## Known constraints

- **Single org in v0:** the entity model is shaped for multi-tenant but does not implement
  tenant isolation. Trigger to revisit: the plane hosts more than one org.
- **Cedar's entity hierarchy bounds relationship expressiveness.** Rich ReBAC beyond it is
  deferred (trigger: relationships outgrow the entity model -> OpenFGA/Zanzibar or a
  relationship store).
- **One central plane:** no multi-machine distribution (OPAL-style) in v0. Trigger: agents
  span machines or must run while the plane is down.
- **Closed action set, open resource space:** a new action kind is a code change; data scopes
  are open and attributed.
- **Stateless v0:** the request carries seams for the stateful lane, but trajectory clauses
  (ASI03/06/08) are not implemented.
- **Framework-layer enforcement is advisory and bypassable**
  ([ADR-0003](docs/adr/0003-govern-at-framework-layer-defer-kernel.md)).
- **Judge nondeterminism** is contained to the semantic lane and excluded from the hard spec.
