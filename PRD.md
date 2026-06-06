# PRD.md

Product intent for `agent-action-classifier`.

> Status: Accepted (v0). This is a **learning project** (see [ADR-0001](docs/adr/0001-build-as-a-learning-project.md)); the optimization is comprehension, not shipping. Architecture decisions live in [docs/adr/](docs/adr/). Technical shape lives in [SPEC.md](SPEC.md).

## Problem Statement

Autonomous coding agents now take real actions (run shell, write files, hit the
network) faster than anyone can wire oversight for them. As of mid-2026, most agents
run with no logging and no enforceable policy: organizations cannot say what their
agents are allowed to do, cannot stop unauthorized actions, and cannot produce an
audit-defensible record of what was decided and why. Provider-level safety filters
(model guardrails) are not auditable and do not encode an *organization's* policy.
And crucially, a local user clicking "approve" does not mean the organization
permits the action.

As a learner, the deeper problem is that the enforcement architecture for agent
governance is unsettled (the OWASP taxonomy is converging, the *how* is not, see
[ADR-0004](docs/adr/0004-owasp-taxonomy-settled-enforcement-open.md)), so there is no
turnkey thing to read and copy. The way to understand it is to build the load-bearing
parts by hand: a fast rule engine, the policy-decision architecture, and the
audit trail, and measure them.

## Solution

A **policy-driven agent-action governance classifier**: given an agent action and an
organization policy, it decides `allow / deny / escalate / flag`, records *which
OWASP clause fired* and *why*, and emits an audit-defensible decision log.

It is built as the standard authorization architecture (XACML's P*Ps): a pure,
environment-independent **Policy Decision Point (PDP)** in Rust, driven from a
Python host that holds the impure edges (the **Policy Enforcement Point** adapter,
the **Policy Information Point** context/approval source, and the audit sink). The
PDP runs a **layered cascade**: a cheap deterministic rule engine settles the clear
cases at sub-millisecond latency and an LLM **judge** is consulted only for the
ambiguous, semantic ones. **Organization policy is supreme**: an explicit org deny
can never be overridden by user approval; scoped user approval only lifts an
*implicit* deny where the org policy delegates the decision.

v0 proves one vertical slice end to end (one OWASP clause, ASI05 Unsafe Code
Execution, on shell and file-write actions) and *measures* it, so the learning is
evidenced, not asserted. Everything else is captured as roadmap so it is not
forgotten.

## User Stories

1. As an **organization security/compliance author**, I want to write rules that say which agent actions are forbidden, allowed, or require approval, so that my organization's policy actually binds the agents we run.
2. As an **org policy author**, I want each rule tagged with the OWASP Agentic clause (ASI01-ASI10) it addresses, so that audit reports speak in a standard risk vocabulary and I can see which clauses I have no coverage for.
3. As an **org policy author**, I want an explicit deny to be unoverridable by any user approval, so that mandatory controls cannot be waived locally.
4. As an **org policy author**, I want to mark some actions as "requires approval", so that the organization can delegate specific discretionary decisions to the user without opening everything.
5. As a **governed agent's operator/user**, I want to approve a specific action or command-class within a scope, so that legitimate work proceeds without the org having to pre-allow everything.
6. As an **operator**, I want my approval to be scoped (this action / this class / this session window), so that a one-time "yes" cannot be replayed by an attacker as blanket consent.
7. As a **governed coding agent**, I want my proposed shell and file-write actions classified before they are recorded, so that unsafe code execution is caught rather than silently executed.
8. As an **auditor/compliance officer**, I want every decision logged with the action, verdict, OWASP clause, rationale, and whether it was a hard gate or a soft gate, so that I can satisfy EU AI Act Article 12's queryable-decision-record requirement.
9. As an **auditor**, I want the decision log to be model-independent and produced at the decision layer, so that it is audit-defensible rather than an opaque model output.
10. As an **auditor**, I want the decision log record shaped so tamper-evidence (hash chaining) can be added later without changing the format, so that today's records remain valid evidence tomorrow.
11. As a **platform integrator**, I want the decision point to be a pure, environment-independent component, so that I can call it from Python now and other languages later without changing its logic.
12. As a **developer-learner**, I want a hand-built deterministic rule engine, so that I understand rule matching, precedence, and zero-allocation evaluation from the inside.
13. As a **developer-learner**, I want the deterministic stage measured against a reference (Microsoft's <0.1ms inline figure), so that I can prove the classifier is negligible against a slow LLM call rather than assume it.
14. As a **developer-learner**, I want any result with no published baseline marked `frontier`, so that I am honest about where I am characterizing a tradeoff rather than hitting a known target.
15. As a **developer-learner**, I want the Python-to-Rust FFI overhead measured, so that I understand and can defend the boundary cost.
16. As a **developer-learner**, I want a conformance corpus that doubles as the benchmark, so that one artifact proves both correctness and latency.
17. As a **developer-learner**, I want the engine shaped for a later stateful (trajectory) lane even though v0 is stateless, so that adding ASI03/ASI06/ASI08 later is an extension, not a rewrite.
18. As a **policy evaluator (the PDP)**, I want to combine applicable rules with deny-overrides, so that a single deny wins regardless of other allows.
19. As a **PDP**, I want to only terminally "allow" an action when no higher-layer (stateful/semantic) clause applies to it, so that an early allow is sound and not a premature pass.
20. As a **PDP**, I want unresolved or semantically ambiguous actions to default to escalate, then fail closed to deny if unresolved, so that the system is fail-safe like standard authorization.
21. As a **judge (semantic lane)**, I want the action plus context (recent trajectory, stated intent, scoped approval state), so that I can reason about cases a deterministic rule cannot settle.
22. As a **judge**, I want to reason *under* org supremacy (deciding whether an action violates org policy, with approval as a mitigating-not-overriding factor), so that my verdicts respect the authority model.
23. As a **developer-learner**, I want the judge measured by graded agreement (target 80-90%, ref: ASSERT / human-to-human ~90%) rather than exact-match, so that nondeterminism is measured honestly and does not corrupt the deterministic spec.
24. As a **developer-learner**, I want v0 fed by a synthetic corpus whose action shape is modeled on a real LangGraph tool-call payload, so that I test against realistic actions without yet building live interception.
25. As a **developer-learner**, I want a writeup of what I built and measured, so that the comprehension is articulated and the benchmarks have a home.

## Implementation Decisions

- **Architecture = XACML P*Ps.** PDP (pure Rust core, the decision) / PEP (Python adapter, intercept + enforce) / PAP (the org policy file) / PIP (Python context + approval source). The PDP is environment-independent and holds no I/O. (Validates the dependency rule from AGENTS.md and the pure-core split.)
- **Polyglot core.** PDP in Rust, consumed from a Python host via PyO3 (precedent: pydantic-core, polars, ruff, HF tokenizers). The **fixed canonical action schema** is load-bearing: a fixed struct (not dynamic JSON) buys zero-alloc matching, a cheap FFI boundary, and stable bindings at once. Other binding shapes (uniffi, WASM, sidecar) are roadmap comparisons.
- **Layered cascade with early-exit.** An action is routed by kind, not pushed through fixed stages. Each layer returns `terminal-deny | terminal-allow | abstain`. Lanes: deterministic stateless (v0), stateful working-memory (roadmap), semantic escalate (judge). Most actions exit at the cheap deterministic layer; only the residue escalates.
- **Precedence = explicit-deny-overrides + default-fail-closed** (AWS IAM model, [ADR-0005](docs/adr/0005-organization-policy-supremacy-and-authz-architecture.md)). Explicit org deny is supreme and unoverridable. Combining algorithm is **deny-overrides** (XACML-named) among *applicable* clauses. Terminal-allow is gated on no higher-layer clause applying. Default residue is **escalate, then deny if unresolved**, an intentional, justified divergence from authz's default-deny (consent-based access control with a human-in-the-loop step).
- **Organization-policy supremacy over user approval** ([ADR-0005](docs/adr/0005-organization-policy-supremacy-and-authz-architecture.md)). Org policy yields hard-deny / hard-allow / requires-approval. Scoped user approval only resolves requires-approval (lifts an *implicit* deny), never an explicit deny.
- **Org policy = tagged concrete rules.** Each rule: matcher (which actions) + outcome (hard-deny/hard-allow/requires-approval) + lane (deterministic/semantic) + OWASP tag. OWASP is the organizing/audit layer, not the rules. Policy is authored data, never literals in the engine. Policy language: **Cedar as the reference model, but our own evaluator** (adopting Cedar's engine would skip the systems lesson); final format is a SPEC decision.
- **Judge present in v0, context-aware.** The escalate lane calls an LLM judge with action + context (trajectory, intent, scoped approval). This introduces nondeterminism by design, handled by the two-eval-regime split below.
- **Decision log in v0, OPA/AAT-shaped.** JSON record = action + verdict + OWASP clause + rule-id + rationale + lane + **gate-type (hard/soft, per EU AI Act Art 12)** + latency. References: OPA decision logs (Allowed/Denied/Advice/Error, where Advice = our escalate), IETF Agent Audit Trail draft. The record is **chain-ready** (carries a `prev_hash` slot); the SHA-256 hash chain itself is roadmap.
- **Measurement = reference-or-frontier** ([ADR-0006](docs/adr/0006-reference-or-frontier-measurement.md)). Every benchmark cites a baseline and reports a delta, or is tagged `frontier` (no baseline; we characterize the tradeoff). Genuinely frontier for us: escalation rate, the layered early-exit tradeoff, the approval-context effect on judge accuracy.
- **v0 vertical slice.** One clause (ASI05 Unsafe Code Execution), stateless, over shell + file-write actions, fed by a synthetic corpus modeled on a real LangGraph payload. Exercises every lane (a hard-deny, a hard-allow, a requires-approval, one semantic escalate) end to end.

## Testing Decisions

- **Good tests assert external behavior, not internals.** A test names an action plus a policy and asserts the verdict, OWASP clause, and gate-type. It never reaches into the engine's matching internals, so the hand-built engine can be rewritten freely as long as behavior holds.
- **The conformance corpus is the spec for the deterministic lanes.** A hand-authored set of ASI05 actions to expected verdicts, asserted at **100% exact-match**. This corpus is also the latency benchmark (one artifact, two jobs). The PDP core is the primary test target.
- **The policy model is tested** for correct rule application (matcher hits, outcome, applicability gating, deny-overrides precedence, explicit-vs-implicit deny behavior).
- **The decision-log shape is tested** for required fields present, gate-type tagged correctly, and chain-readiness (`prev_hash` slot present).
- **The judge is measured, not unit-tested.** A graded eval reports agreement with reference verdicts on the ambiguous cases (target 80-90%); it is deliberately kept out of the exact-match conformance suite because it is nondeterministic.
- **Benchmarks are tests with references.** Stage-1 latency vs the <0.1ms reference; PyO3 FFI overhead vs published crossing costs; anything without a baseline reported as `frontier`.
- Prior art: none in-repo yet (greenfield). The conformance corpus establishes the testing pattern for the project.

## Out of Scope

- **Live enforcement / blocking.** v0 classifies and logs (it produces verdicts including deny, but there is no live agent to stop because the corpus is synthetic). Acting on a verdict to pause/deny a live agent needs a live PEP and is roadmap.
- **Live provider adapters and the cross-provider divergence lesson.** v0 uses a synthetic corpus; live LangGraph interception, and a second adapter to feel where providers diverge, are roadmap.
- **Stateful / trajectory lanes.** ASI03 privilege accumulation, ASI06 memory poisoning, ASI08 cascading failures need working memory (Rete). The engine is *shaped* for this lane in v0 but does not implement it.
- **Tamper-evident audit (SHA-256 hash chain, append-only store, write/read/delete separation) and the Action Provenance Graph.** The record is chain-ready; the chain and provenance graph are roadmap.
- **Kernel-level enforcement.** Deferred as a later defense-in-depth lesson ([ADR-0003](docs/adr/0003-govern-at-framework-layer-defer-kernel.md)).
- **Additional OWASP clauses beyond ASI05**, and additional policy-language adoption (Cedar/Rego interop) beyond the reference model.
- **Productization concerns** (moat, adoption, multi-language bindings, managed policy administration UI) per the learning-project framing.

## Further Notes

- "Why now": EU AI Act Article 12 reaches full enforcement 2026-08-02, requiring queryable decision records that distinguish hard gates from soft gates, exactly the verdict/gate-type model here. Even as a learning project, this builds toward a thing about to be mandated.
- Pinned frontier reference snapshot ([ADR-0002](docs/adr/0002-pin-a-fixed-frontier-reference-snapshot.md)): OpenAI layered guardrails + static risk table, Anthropic cascade, OWASP taxonomy. Reconstruct from public descriptions; do not clone.
- The "spec as executable code" thesis applies: the deterministic conformance corpus is the living spec (it fails CI on desync); the judge is measured, not pinned; prose specs are avoided as drift-prone.
- Roadmap order to keep captured: (1) hash-chained audit log, (2) live LangGraph PEP + real enforcement, (3) stateful lane / Rete working memory for trajectory clauses, (4) second provider adapter for the divergence lesson, (5) alternate core distribution shapes (uniffi/WASM/sidecar) for comparison.
