# ADR-0025: v1 is flat — remove the org-graph hierarchy plane

Date: 2026-06-16
Status: Accepted

Supersedes the hierarchy realization of [ADR-0020](0020-principal-hierarchy-as-cedar-groups.md);
narrows the org-first scope of [ADR-0005](0005-organization-policy-supremacy-and-authz-architecture.md)
and [ADR-0017](0017-adopt-cedar-engine-org-modeled-central-plane.md) to a single flat principal for v1.

## Context
[ADR-0020](0020-principal-hierarchy-as-cedar-groups.md) built `corpus/org_graph/` — a
conformance plane with `Org <- Team <- User <- Agent` membership plus a cross-cutting `Role`,
node-attached + RBAC policies, 9 cases proving cascade, sub-node deny-override
(`sales` vs `eng`), team-scoped approval, and an org-wide hard deny. It passed at 100%
exact-match and proved Cedar evaluates the principal hierarchy natively.

But the plane was wired to **nothing**. Verified at the time of this decision:

- The live PEP (`enforce`) loads `corpus/asi05` — a flat plane whose only entity types are
  `Agent` and `DataScope`, and whose `agent-1` has `parents = []`. No hierarchy is reachable
  from any hook.
- `org_graph` was loaded only by its own CI conformance test. No binary, no host path, no agent
  ever evaluated against an `Org`/`Team`/`Role`/`User` entity.
- The sandbox wiring even passed `--agent-id agent-eng`, an id no loaded plane declared — a
  dangling principal reference, because the org identity had no real seam.

So `org_graph` tested a capability the product never exercised. The stated v1 (a handful of
personal agents sharing one flat rule list — see the v1 framing in [PRD.md](../../PRD.md)) does
not have teams, roles, or an org chart. Per the project's own AHA/YAGNI bar, hierarchy was
speculative generality living in a test fixture: validated against itself, not against a use case.
Re-reading the corpus repeatedly raised "why is `sales`/`eng` here when there is no model?" —
the correct signal that the layer did not fit.

## Decision
1. **Delete `corpus/org_graph/` and `org_graph_conformance.rs`.** v1 governs a single flat
   principal; the only conformance plane is `asi05` (four-verdict cascade on resource attributes
   + shell-command kinds). The `policy_lint` plane-count floor drops from 2 to 1.
2. **The principal stays a flat `Agent`.** No `parents`, no `in` resolution, no node-attached
   policies in v1. Rules apply to `principal` unconstrained (or by resource/context attributes),
   not by position in a graph.
3. **`load_corpus(name)` stays generic.** It is a one-line cost and is the seam a future plane
   (org or another OWASP clause) plugs into. Keeping it is not speculative — it is already used.
4. **The latency bench keeps its inline `Org`/`Team` entities.** `benches/cedar_decide.rs` builds
   a hierarchy purely to measure Cedar eval cost; it asserts no product behavior and loads no
   corpus, so it is unaffected by this decision.

## Consequences
- **One plane, one mental model.** The corpus now matches what the product does: flat agent,
  resource-attribute + command-kind rules, four verdicts. The recurring confusion is gone.
- **Org-supremacy is deferred, not denied.** The cascade still works in Cedar for free
  ([ADR-0020](0020-principal-hierarchy-as-cedar-groups.md)'s finding holds); v1 simply does not
  use it. The decision to model governance org-first ([ADR-0005](0005-organization-policy-supremacy-and-authz-architecture.md),
  [ADR-0017](0017-adopt-cedar-engine-org-modeled-central-plane.md)) is narrowed to flat-for-v1,
  not reversed.
- **No code-path lost.** Nothing live read `org_graph`, so no enforcement behavior changes; only
  CI shrinks by 9 cases and the dead fixture is gone.

## Deferred (with revisit trigger)
- **Re-introduce a hierarchy plane** when there is a real multi-agent org to govern: more than one
  team or role authoring distinct policy, or an agent whose effective scope must be attenuated by
  the principal it acts for. Trigger: a second authoring party, or a tenant boundary. At that
  point rebuild the plane against a live seam (the PEP loading the agent's parent chain), not as a
  standalone fixture — the mistake this ADR corrects.
