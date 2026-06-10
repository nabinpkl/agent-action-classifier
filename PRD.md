# PRD.md

Product intent for `agent-action-classifier`.

> Status: Accepted (v0). Personal/learning-oriented, but the optimization shifted in
> [ADR-0017](docs/adr/0017-adopt-cedar-engine-org-modeled-central-plane.md): adopt the
> industry-standard engine (Cedar) and build the parts that have no standard, rather than
> rebuild the engine by hand. Architecture decisions live in [docs/adr/](docs/adr/);
> technical shape in [SPEC.md](SPEC.md).

## Problem Statement

I run several coding agents. Each can run a hook that approves or denies a tool call
before it executes, but the policy behind that decision lives nowhere central: there is
no single place to say what my agents may do, and no way to author a rule once and have
it bind every agent. As of mid-2026 this is the general state of agent governance:
organizations cannot state their agents' permitted actions in one authoritative place,
cannot propagate a policy change to every agent at once, and cannot produce an
audit-defensible record of what was decided and why. Provider-level model guardrails are
not auditable and do not encode *my* (or an organization's) policy, and a local user
clicking "approve" does not mean the policy permits the action.

The decision must be **modeled org-first** even when the deployment is just my own agents,
because the same model has to generalize to an organization with structure: principals
(org, team, role, user, agent) with **inheritance and relationships**, where an agent's
permitted scope is resolved by its position in that graph. Building the flat,
single-machine special case first would hardcode the degenerate shape and fight every
later org feature.

The enforcement architecture for agent governance is still unsettled (the OWASP taxonomy
is converging, the *how* is not, see
[ADR-0004](docs/adr/0004-owasp-taxonomy-settled-enforcement-open.md)). The engine,
however, no longer has to be invented: Cedar is the emerging standard
([ADR-0017](docs/adr/0017-adopt-cedar-engine-org-modeled-central-plane.md)). The work, and
the learning, is in the parts that have no standard: the org model, the canonical action
schema, the deterministic-plus-semantic cascade, and the per-agent enforcement hook.

## Solution

A **centralized, org-modeled policy plane that governs agent tool calls**. Policies are
authored once on the plane and propagate to every agent beneath the relevant node in the
org graph; each agent's **PreToolUse hook is the enforcement point** that resolves its
effective policy and decides `allow / deny / escalate / flag` on each tool call, records
*which OWASP clause fired* and *why*, and emits an audit-defensible decision log.

It is built as the standard authorization architecture (XACML's P\*Ps):

- **PDP (decide):** the policy decision point, evaluating with **Cedar** (the embedded
  `cedar-policy` engine, [ADR-0017](docs/adr/0017-adopt-cedar-engine-org-modeled-central-plane.md)).
  Cedar's `forbid`-overrides-`permit` and default-deny are the org-supremacy authority model
  ([ADR-0005](docs/adr/0005-organization-policy-supremacy-and-authz-architecture.md)).
- **PEP (enforce):** each agent's PreToolUse hook, intercepting the tool call and applying the
  verdict (the `repo_alignment` exit-2-to-deny hook is the proven primitive).
- **PAP (author):** the central plane holding the org graph and the Cedar policies; a policy on a
  node cascades to everything beneath it (propagation = Cedar entity inheritance).
- **PIP (context):** scoped approvals and, later, trajectory.

The plane runs a **layered cascade**: Cedar settles the clear cases deterministically at
sub-millisecond latency, and an LLM **judge** is consulted only for the ambiguous, semantic
ones. **Organization policy is supreme:** an explicit org deny can never be overridden by user
approval; scoped user approval only lifts an *implicit* deny the org policy delegates.

The 5-agent personal deployment is the **degenerate single-org instance**: one shallow tree,
every agent under one node, so one authored policy binds them all. The same model scales to a
modeled organization without re-founding.

## User Stories

1. As an **operator of several agents**, I want to author a policy once on a central plane and have it bind all my agents, so that I do not configure each agent's hook by hand or drift out of sync.
2. As a **policy author**, I want to attach a policy at a node in the org graph (org/team/role/user/agent) and have it cascade to everything beneath, so that authoring once at the right level reaches every agent under it.
3. As a **policy author**, I want an agent's effective scope resolved by its position in the org graph (inheritance + relationships), so that agents inherit (and at most attenuate) the scope of the principal they act for.
4. As a **policy author**, I want rules expressed in a standard policy language (Cedar), so that I am not learning or maintaining a bespoke language and can use its tooling and analysis.
5. As an **org policy author**, I want to say which agent tool calls are forbidden, allowed, or require approval, so that my policy actually binds the agents I run.
6. As an **org policy author**, I want each rule tagged with the OWASP Agentic clause (ASI01-ASI10) it addresses, so that audit reports speak in a standard risk vocabulary and I can see coverage gaps.
7. As an **org policy author**, I want an explicit deny to be unoverridable by any user approval, so that mandatory controls cannot be waived locally.
8. As an **org policy author**, I want to mark some tool calls as "requires approval", so that the organization can delegate specific discretionary decisions without opening everything.
9. As a **governed agent's operator**, I want to approve a specific tool call or class within a scope, so that legitimate work proceeds without pre-allowing everything.
10. As an **operator**, I want my approval scoped (this call / this class / this session window), so that a one-time "yes" cannot be replayed as blanket consent.
11. As a **governed agent**, I want my proposed tool calls evaluated by my hook before they execute, so that disallowed actions are denied rather than silently run.
12. As an **auditor**, I want every decision logged with the action, verdict, OWASP clause, rationale, and hard-vs-soft gate, so that I can satisfy EU AI Act Article 12's queryable-decision-record requirement.
13. As an **auditor**, I want the decision log model-independent and produced at the decision layer, so that it is audit-defensible rather than an opaque model output.
14. As an **auditor**, I want the record shaped so tamper-evidence (hash chaining) can be added later without changing the format, so that today's records remain valid evidence tomorrow.
15. As a **platform integrator**, I want the decision point to be a clean component over a standard engine, so that I can call it from the host now and other surfaces later without re-implementing policy logic.
16. As a **PDP**, I want to combine applicable policies with deny-overrides and default-deny (Cedar's native semantics), so that a single deny wins and the unmatched case fails closed.
17. As a **PDP**, I want to only terminally "allow" when no higher-layer (semantic/stateful) clause applies, so that an early allow is sound and not a premature pass.
18. As a **PDP**, I want unresolved or semantically ambiguous tool calls to default to escalate, then fail closed to deny if unresolved, so that the system is fail-safe.
19. As a **judge (semantic lane)**, I want the action plus context (recent trajectory, stated intent, scoped approval), so that I can reason about cases Cedar cannot settle deterministically.
20. As a **judge**, I want to reason *under* org supremacy (deciding whether a tool call violates org policy, approval as mitigating-not-overriding), so that my verdicts respect the authority model.
21. As a **developer-learner**, I want the deterministic Cedar stage measured against a reference (Microsoft's <0.1ms inline figure), so that I can prove enforcement is negligible against a slow LLM call rather than assume it.
22. As a **developer-learner**, I want any result with no published baseline marked `frontier`, so that I am honest about where I characterize a tradeoff versus hit a known target.
23. As a **developer-learner**, I want the judge measured by graded agreement (target 80-90%, ref ASSERT / human-to-human ~90%) rather than exact-match, so that nondeterminism is measured honestly and does not corrupt the deterministic spec.
24. As a **developer-learner**, I want the org graph and canonical action shaped for a later stateful (trajectory) lane and for multi-tenant orgs even though v0 is single-org and stateless, so that those are extensions, not rewrites.

## Implementation Decisions

- **Engine = Cedar, adopted not built** ([ADR-0017](docs/adr/0017-adopt-cedar-engine-org-modeled-central-plane.md)).
  The PDP embeds the `cedar-policy` Rust crate. Pending a latency spike that confirms embedded
  eval meets the stage-1 budget, the hand-built `Matcher`/precedence core is replaced by Cedar.
- **Model = org-first.** Principals (org, team, role, user, agent) are Cedar entities in a
  hierarchy; inheritance is Cedar's entity-parent / `in`; per-principal grants are Cedar policy
  templates; data scopes are resources with attributes (ABAC). Policy on a node cascades down.
  Single org in v0; multi-tenant deferred (trigger: the plane hosts more than one org).
- **Central plane + per-agent hook enforcement.** One plane authors and holds the org model and
  Cedar policies; each agent's PreToolUse hook is the PEP that resolves the effective policy and
  approves/denies. Distribution to many machines (OPAL-style) is deferred (trigger: agents span
  machines or must run while the plane is down).
- **Canonical action = principal × action × resource × context.** Principal = the agent with its
  inheritance chain; action = the tool call kind; resource = the data scope it touches (with
  attributes); context = scoped approval and, later, trajectory. This replaces the placeholder
  `Operation` enum. There is no industry standard for this schema yet (NIST is drafting), so it is
  genuinely ours to design; MCP tool descriptions are an input to it, not a replacement.
- **Architecture = XACML P\*Ps** with the dependency rule preserved: the decision point is a clean
  component, the impure edges (hooks/PEP, context/PIP, audit sink) implement what it needs.
- **Precedence = deny-overrides + default-fail-closed**, which is Cedar-native (`forbid` overrides
  `permit`, no match denies). Terminal-allow gated on no higher-layer clause applying. Default
  residue is **escalate, then deny if unresolved** (consent-based access control with a
  human/judge step), an intentional divergence from bare default-deny.
- **Organization-policy supremacy over user approval**
  ([ADR-0005](docs/adr/0005-organization-policy-supremacy-and-authz-architecture.md)), expressed in
  Cedar: an explicit `forbid` is supreme; scoped approval only resolves the requires-approval case.
- **OWASP tag on each policy.** The clause (ASI01-ASI10) rides on Cedar policy annotations; it is the
  organizing/audit layer, not the decision logic.
- **Judge present, context-aware.** The escalate lane calls an LLM judge with action + context. It
  composes *around* Cedar (Cedar decides allow/deny/escalate; the judge resolves the escalation),
  introducing nondeterminism handled by the two-eval-regime split.
- **Decision log, OPA/AAT-shaped.** JSON record = action + verdict + OWASP clause + policy id +
  rationale + lane + gate-type (hard/soft, EU AI Act Art 12) + latency, chain-ready (`prev_hash`
  slot; the SHA-256 chain itself is roadmap).
- **Measurement = reference-or-frontier** ([ADR-0006](docs/adr/0006-reference-or-frontier-measurement.md)).
  Every benchmark cites a baseline and reports a delta, or is tagged `frontier`.

## Testing Decisions

- **Good tests assert external behavior, not internals.** A test names a tool call, a principal,
  and a policy and asserts the verdict, OWASP clause, and gate-type. It never reaches into Cedar's
  or the host's internals, so the integration can change as long as behavior holds.
- **The conformance corpus is the spec for the deterministic lane.** A hand-authored set of tool
  calls + org model + policy to expected verdicts, asserted at **100% exact-match**, doubling as the
  latency benchmark. It now tests the Cedar integration (mapping + policy), not a bespoke engine.
- **The org model is tested** for correct inheritance resolution (an agent's effective scope from its
  position in the graph), policy cascade down a node, and deny-overrides precedence.
- **The decision-log shape is tested** for required fields, correct gate-type tagging, and
  chain-readiness (`prev_hash` slot present).
- **The judge is measured, not unit-tested.** A graded eval reports agreement with reference verdicts
  on the ambiguous cases (target 80-90%); kept out of the exact-match suite because it is
  nondeterministic.
- **Benchmarks are tests with references.** Cedar deterministic latency vs the <0.1ms reference; the
  FFI/host overhead vs published crossing costs; anything without a baseline reported as `frontier`.

## Out of Scope

- **Multi-tenant (many orgs in one plane).** v0 is single-org; the model is shaped for it but does
  not implement tenant isolation. Trigger to revisit: the plane hosts more than one org.
- **Rich ReBAC / relationship-graph authorization** beyond what Cedar's entity hierarchy expresses.
  Trigger: relationships outgrow Cedar's entity model (then OpenFGA/Zanzibar or a relationship store).
- **Policy distribution to many machines (OPAL-style).** v0 is one central plane. Trigger: agents
  span machines or must run while the plane is down.
- **Stateful / trajectory lanes.** ASI03/ASI06/ASI08 need working memory; the model is *shaped* for
  this lane but v0 does not implement it.
- **Tamper-evident audit (SHA-256 hash chain, append-only store) and the Action Provenance Graph.**
  The record is chain-ready; the chain and graph are roadmap.
- **Kernel-level enforcement.** Deferred ([ADR-0003](docs/adr/0003-govern-at-framework-layer-defer-kernel.md)).
- **Additional OWASP clauses beyond the v0 slice.**

## Further Notes

- "Why now": EU AI Act Article 12 reaches full enforcement 2026-08-02, requiring queryable decision
  records that distinguish hard from soft gates, exactly the verdict/gate-type model here.
- The agent world is converging on Cedar for tool-call authorization (AWS Bedrock AgentCore Policy,
  March 2026; the Cedar team's `cedar-for-agents` MCP tooling, May 2026), which is why adopting it
  rather than inventing a language is the standards-aligned move
  ([ADR-0017](docs/adr/0017-adopt-cedar-engine-org-modeled-central-plane.md)).
- The "spec as executable code" thesis still holds: the deterministic conformance corpus is the
  living spec (it fails CI on desync); the judge is measured, not pinned.
- Roadmap order to keep captured: (1) Cedar speed spike + core swap, (2) org-graph + inheritance
  resolution, (3) live per-agent hook PEP wired to the plane, (4) hash-chained audit log, (5)
  semantic judge lane, (6) multi-tenant orgs, (7) policy distribution for multi-machine fleets.
