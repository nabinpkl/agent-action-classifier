# ADR-0001: Build agent-action-classifier as a learning project, not a product

Date: 2026-06-06
Status: Accepted

## Context
Early scoping reasoned in product terms: defensibility, moat, competition with
Microsoft's Agent Governance Toolkit, adoption, portability as market value. Those
constraints drove several choices (e.g. "observer-first to avoid Microsoft's
sub-0.1ms blocking niche"; "no-GC is premature optimization"). The actual intent
is comprehension: this repo is a vehicle to learn agent-action governance,
rule-engine internals, and polyglot systems design by building them by hand.

## Decision
Treat this as a learning project. The optimization function is depth of
understanding, not shippability or defensibility. Concretely:
- Competition, moat, adoption, and portability-as-product-value are out of scope as
  decision drivers.
- "Don't rebuild what a maintained package does," the dependency bar, AHA and YAGNI
  (from AGENTS.md) are relaxed where rebuilding is the lesson. Rebuilding the rule
  engine by hand is the point, not a liability.
- Choices are judged by "is it instructive," not "is it necessary." The original
  no-GC / Rust / hand-built Rete instinct is legitimate *as a systems lesson*, not
  because the latency budget requires it (it does not; see ADR on architecture).
- The syllabus is a blend of two tracks: systems/performance (Rust core, rule
  engine, zero-GC, interception internals) and domain/architecture (OWASP policy
  modeling, canonical action schema, provider adapters, conformance-as-spec).

## Consequences
- Product-mode reasoning in earlier notes is superseded. Observer-vs-blocker and
  language choice are now learning decisions (likely build and compare), not
  competitive ones.
- AGENTS.md is written product-style; some rules invert under a learning lens. Per
  its own meta-rule (rules are mutable working agreements), revisit it with a
  learning lens rather than silently working against it. Not done yet; deferred.
- Success is measured by what is understood and benchmarked, not by users.
