# ADR-0004: Treat the OWASP taxonomy as settled, the enforcement architecture as open

Date: 2026-06-06
Status: Accepted

## Context
The agent-governance space has split into two layers at different stages of
maturity. The *taxonomy* of risks is converging: OWASP's Top 10 for Agentic
Applications 2026 (ASI01-ASI10), published December 2025, is the first formal one
and is now the common reference. The *enforcement architecture* is wide open:
Anthropic uses white-box activation probes, OpenAI uses black-box layered classifiers
plus a static risk table, Microsoft uses kernel-inspired interception; they disagree
on white-box vs black-box, framework vs kernel, block vs observe, and the policy
language (Rego vs Cedar vs YAML). Frontier disagreement on fundamentals is the proof
the pattern is unsettled.

## Decision
Separate the settled from the open and spend effort accordingly:
- **Settled target (adopt, do not invent):** the OWASP Agentic Top 10 is the risk
  taxonomy the classifier classifies against.
- **Open design space (genuinely ours to explore):** the enforcement architecture,
  how and where to evaluate policy, what the engine and the canonical action schema
  look like, observer vs blocker.

## Consequences
- No effort wasted inventing a taxonomy; classification has a stable, standard target.
- Architecture exploration is informative and not at risk of being made obsolete by a
  canonical pattern, because none exists yet.
- A clear rule for new decisions: adopt on the taxonomy axis, explore on the
  architecture axis.
