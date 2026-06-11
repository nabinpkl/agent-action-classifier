# ADR-0020: Model the principal hierarchy as Cedar entity groups

Date: 2026-06-11
Status: Accepted

## Context
[ADR-0017](0017-adopt-cedar-engine-org-modeled-central-plane.md) committed to modeling
governance org-first: principals (org, team, role, user, agent) sit in an
inheritance/relationship graph, and an agent's effective scope is resolved by its position in
that graph. TASKS #3 is to make that real — a policy authored once at a node must cascade to
every agent beneath it.

The question was how much engine code this needs. The answer, from exploring Cedar, is **none**.
Cedar evaluates hierarchy natively: entities carry `parents`; the `in` operator walks the
transitive closure of membership; RBAC is just `principal in <Group>`; precedence down the tree
is Cedar's deny-overrides. The pure core (`policy_decision`) never inspects the principal beyond
mapping it to `Agent::"<id>"`, and `decide` already hands the full entity store to
`is_authorized`, which resolves `in` itself. So the org graph is **data** — schema + entities +
policies — not new logic.

## Decision
1. **Org/Team/Role/User/Agent are Cedar entity types with `in` membership.** The structural
   spine is `Org <- Team <- User <- Agent` (declared `entity Team in [Org];`,
   `entity User in [Team, Role];`, `entity Agent in [User, Role];`). `Role` is a cross-cutting
   RBAC group an Agent or User can also be `in`. Only Agents take actions, so the request
   principal type stays `[Agent]`; the other types are groups the agent is `in`.
2. **A policy attached at a node is one scoped `principal in <Node>`.** Cascade-down is the
   transitive `in`: `permit(principal in Org::"acme", ...)` reaches every agent under acme.
3. **Sub-node override is deny-overrides.** A `forbid` on a deeper node beats a `permit` on an
   ancestor, so the same action can resolve Allow for an agent under one team and Deny for an
   agent under another. Position in the graph changes the verdict.
4. **RBAC is role-group membership.** `permit(principal in Role::"data-reader", ...)` grants by
   role regardless of team. An agent holds a role by having it as a parent.
5. **The personal "5 agents" case is the degenerate single-node tree** — all agents under one
   node, so a policy there reaches all of them. No special case.

The `corpus/org_graph/` conformance corpus encodes all of this and passes at 100% exact-match
through the unchanged `decide`.

## Consequences
- **Zero pure-core change; hierarchy is data, evaluated natively by Cedar.** This is the direct
  payoff of [ADR-0017](0017-adopt-cedar-engine-org-modeled-central-plane.md): adopting a standard
  engine bought us the org graph for free, with nothing to revisit on the engine side.
- **Reinforces [ADR-0005](0005-organization-policy-supremacy-and-authz-architecture.md):** org
  supremacy *is* deny-overrides — an org/team `forbid` is unoverridable by a descendant permit or
  by scoped approval (approval is consulted only on an Allow path, host-side).
- **The entity store is the org model.** Authoring the graph = editing `entities.json` (parents)
  and node-scoped policies; no code deploy. The schema validates membership under `Strict`, so a
  malformed hierarchy fails loud at load (`PolicyLoadError`), same as any other drift.
- **Two focused corpora.** `asi05` proves the four-verdict cascade on a flat principal; `org_graph`
  proves the hierarchy. Each stays single-concept.

## Deliberately deferred (each with a revisit trigger)
These are real future capabilities, recorded here so they are not lost when the ephemeral plan is.

- **Host-side "resolve effective entities" helper** — hand an agent only its own ancestor slice
  instead of the org's full entity store. **Trigger:** the central plane serves agents over a
  boundary (an agent should not receive the whole org graph).
- **Per-agent policy slicing** (AVP-style: pre-filter to policies whose scope the agent satisfies
  before evaluating). **Trigger:** the policy set grows large enough that whole-set evaluation
  misses the stage-1 latency budget.
- **Multi-org tenancy** (many orgs in one plane). **Trigger:** the plane hosts more than one org
  (also [ADR-0017](0017-adopt-cedar-engine-org-modeled-central-plane.md)).
- **ReBAC / OpenFGA** for relationships beyond a strict hierarchy (cross-team sharing, ownership
  graphs). **Trigger:** relationships outgrow what Cedar's entity hierarchy expresses cleanly
  (also [ADR-0017](0017-adopt-cedar-engine-org-modeled-central-plane.md)).
- **Principal-side ABAC attributes** (e.g. `Team.region`, time-bounded membership). **Trigger:** a
  policy needs to branch on a principal/node attribute, not just membership.
