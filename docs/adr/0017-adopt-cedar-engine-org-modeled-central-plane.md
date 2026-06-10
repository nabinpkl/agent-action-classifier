# ADR-0017: Adopt Cedar as the policy engine; model governance org-first via a central plane

Date: 2026-06-10
Status: Accepted

## Context
The project began as a hand-built deterministic rule engine over a closed `Operation`
enum (`ShellExec` / `FileWrite` / `NetworkFetch`), with building the engine by hand as
the learning goal ([ADR-0001](0001-build-as-a-learning-project.md)) and Cedar named only
as a *reference model* ([ADR-0005](0005-organization-policy-supremacy-and-authz-architecture.md),
SPEC). A June 2026 research-and-decision arc revisited both the engine choice and the
actual domain, and both shifted.

**The engine landscape, re-examined.** XACML is a standard with slow XML implementations
("XACML is dead", Forrester). OPA/Rego is powerful but its own community flags
non-determinism and runtime exceptions as failure modes (a direct conflict with this
project's exact-match conformance requirement), and its maintainers' future went uncertain
after Apple's August 2025 acqui-hire. Cedar is a Rust, formally-verified (Lean), deterministic,
microsecond-latency engine that the agent world is converging on: AWS Bedrock AgentCore
Policy chose Cedar for agent-tool governance (GA March 2026), and the Cedar team ships
`cedar-for-agents` tooling (MCP-schema generation + analysis, May 2026). Inventing a bespoke
policy language is a known trap, and growing our JSON matcher schema into one is the same
trap; the honest options are to adopt a standard engine or hand-build an evaluator for a
standard language. The optimization shifted to "use the industry standard where one exists;
the outcome is a standard policy engine enforcing agents."

**The domain, clarified.** The real need is not a classifier over OS operations. It is a
**centralized policy plane** where a policy authored once propagates to N agents whose
PreToolUse hooks approve or deny tool calls, **modeled org-first**: principals (org, team,
role, user, agent) sit in an inheritance/relationship graph, and an agent's effective scope
is resolved by its position in that graph. The personal "5 agents" case is the degenerate
single-org instance, not a separate special case. The `Operation` enum was placeholder
scaffolding written by an agent, not the domain.

## Decision
1. **Adopt Cedar as the policy engine and language.** Embed the `cedar-policy` Rust crate
   as the PDP's evaluator. **Gate:** a latency spike must confirm embedded evaluation meets
   the stage-1 budget (p99 `< 100µs`; ref Microsoft `<0.1ms`) before the hand-rolled core is
   removed ([ADR-0006](0006-reference-or-frontier-measurement.md), reference-or-frontier).
2. **Model governance org-first.** Principals are Cedar entities in a hierarchy; inheritance
   is Cedar's entity-parent / `in`; per-principal grants are Cedar policy templates; data
   scopes are resources with attributes (ABAC). A policy attached at a graph node cascades to
   everything beneath it — **propagation is inheritance**. Single org for now.
3. **Centralized plane + per-agent hook enforcement.** One central plane authors and holds the
   org model plus the Cedar policies. Each agent's **PreToolUse hook is the PEP** that resolves
   the agent's effective policy (via its place in the graph) and approves/denies the tool call.
   The existing `repo_alignment` PreToolUse hook (exit-2-to-deny) is the proven enforcement
   primitive this builds on.
4. **The 5-agent personal setup is the degenerate instance:** one shallow org tree, all agents
   under a single node, so a policy authored there reaches all of them.

## Consequences
- **Supersedes [ADR-0001](0001-build-as-a-learning-project.md)'s "rebuild the rule engine by
  hand is the point."** The engine is adopted, not built. Comprehension is still a goal, but it
  moves up a layer — to modeling agent governance (the org graph, the canonical action, the
  cascade, the hooks) over a standard engine — and "use the industry standard where one exists"
  now wins over "rebuild for the lesson."
- **Replaces hand-built core code** (pending the speed gate): `crates/policy_decision`'s
  `Matcher` (`policy.rs`) and the precedence engine (`evaluate.rs`) are replaced by Cedar; the
  placeholder `Operation` enum (`canonical_action.rs`) is replaced by the
  principal-with-inheritance × action × data-scope-resource × context model. These are now
  scaffolding to remove, not a spec to preserve.
- **Reinforces, not supersedes, [ADR-0005](0005-organization-policy-supremacy-and-authz-architecture.md).**
  Cedar's `forbid`-overrides-`permit` and default-deny *are* the org-supremacy authority model;
  scoped approval maps onto Cedar policies + context/entities. Reinforces
  [ADR-0004](0004-owasp-taxonomy-settled-enforcement-open.md): Cedar is the settling standard on
  the engine axis, where the rule is "adopt."
- **The deterministic + semantic cascade composes around Cedar.** Cedar is the deterministic
  decision; escalate-to-judge stays host orchestration; OWASP-clause tagging rides on Cedar
  policy annotations.
- **Crosses the dependency bar deliberately.** `cedar-policy` becomes a real runtime dependency
  of a previously zero-dependency core. Justified: it is the emerging standard, production-proven
  at AWS, and formally verified.
- **Deferred, each with a revisit trigger:** multi-tenant many-orgs (trigger: the plane hosts
  more than one org); ReBAC / OpenFGA (trigger: relationships outgrow what Cedar's entity
  hierarchy expresses cleanly); policy distribution / OPAL (trigger: agents span multiple
  machines or must run while the plane is down).
- **PRD.md and SPEC.md are reframed** from the classifier-over-`Operation`s framing to the
  org-modeled central-plane framing. This ADR is the decision; those docs reflect it.
