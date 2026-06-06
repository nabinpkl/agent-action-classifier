# ADR-0002: Pin a fixed frontier reference snapshot

Date: 2026-06-06
Status: Accepted

## Context
There is no settled architecture for agent-action governance yet (see ADR-0004).
The frontier players disagree on fundamentals, which makes "evolve alongside the
frontier" attractive but dangerous: reflexively tracking every new blog post from
Anthropic, OpenAI, or Microsoft turns into constant re-architecture and finishing
nothing. A learning project needs a stable reference to reconstruct against.

## Decision
Pin a fixed snapshot of frontier designs as the reference to reconstruct, and treat
it as immutable for the duration of the build rather than a moving target:
- **OpenAI layered guardrails + static tool risk table** (low/medium/high by
  read-vs-write, reversibility, permissions, financial impact).
- **Anthropic cascade**: cheap deterministic first stage that screens everything and
  escalates only ambiguous cases to an expensive second stage.
- **OWASP Top 10 for Agentic Applications 2026 (ASI01-ASI10)** as the risk taxonomy.

Reconstruct the *architecture and ideas* from public descriptions and build a
minimal version. Do not clone an implementation. Evolution beyond the snapshot is
*deliberate*: adopt a new frontier idea only when the reason it is better is
understood, never reflexively because something shipped recently.

## Consequences
- A stable target to build against; exploration is not invalidated by frontier churn.
- Frontier monitoring is for understanding, not for triggering rewrites.
- When the snapshot is intentionally revised, supersede this ADR with a new one
  stating what changed and why.
