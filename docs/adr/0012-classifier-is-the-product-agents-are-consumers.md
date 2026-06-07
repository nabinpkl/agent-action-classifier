# ADR-0012: The classifier is the product; agents are consumers, not peers

Date: 2026-06-07
Status: Accepted

## Context
Three kinds of code will live here: the pure Rust PDP core, the Rust↔Python binding
(the host wheel), and pure-Python agent code (LangGraph/LangChain) that drives the
classifier. The open question was whether the agent code is a **first-class peer
component** (which would justify re-founding the repo into a polyglot monorepo: a uv
workspace of multiple Python packages mirroring the cargo workspace, with the heavy
agent dependencies in their own member) or a **consumer** of the product.

Surveying the precedent ([ADR-0011](0011-workspace-layout-pure-core-and-binding-crate.md)):
co-rooting `Cargo.toml` and `pyproject.toml` (polars, pydantic-core) is the maturin-
ecosystem norm but reads as "a Rust project wearing a Python face." A neutral-root
`rust/` + `python/` split reads as "a system of equal components." The choice hinges on
whether an agent is load-bearing.

It is not. The project thesis ([project scope], ADR-0003/0007) is that the governance
engine works **beyond any one provider**: you define the rule engine once and it applies
to any agent with hooks. That only holds if no particular agent framework is structural.
The dependency chain is one-directional and inward: `agent -> host -> binding -> core`,
never the reverse.

## Decision
- **The classifier (Rust core + host wheel) is the product.** Agents are **consumers**
  that exercise it, sitting at the leaf of the dependency chain. No agent framework is
  load-bearing.
- **Retain the polars-style co-root** ([ADR-0011](0011-workspace-layout-pure-core-and-binding-crate.md)):
  one `Cargo.toml` workspace + one `pyproject.toml` at the root. **Do not** split into a
  uv workspace of peer Python packages; there is one Python product (the host), not many.
- **Agent exercises live as examples** under `examples/`, consuming the host package.
  Their heavy dependencies (langgraph, an LLM client) go in an **optional
  `[dependency-groups]`**, never in the host wheel's runtime `dependencies`. So the wheel
  stays lean and the "don't weld langchain into the wheel" concern is structural, not
  vigilance.
- An exercise may **generate actions that feed the conformance corpus**, but the corpus
  and classifier depend on no agent; the arrow stays inward.

This pins the project identity: a **governance engine**, not an agent framework.

## Consequences
- No restructure now; the current layout stands. The whole "three things collide"
  problem dissolves because the third thing was never a peer.
- The re-found-into-a-uv-workspace option is explicitly deferred to a real trigger: an
  exercise growing into a distributable, reusable harness that needs its own package.
  Until then, `examples/` + an optional dep-group is the home.
- Keeps faith with the provider-agnostic thesis: swapping LangGraph for a coding agent or
  a future framework is a new consumer wiring its PEP adapter into the same core, not a
  change to the architecture's center.
- The binding's JSON wire and PEP adapter remain the stable contract every consumer
  targets, reinforcing ADR-0007 rather than competing with it.

[project scope]: ../../PRD.md
