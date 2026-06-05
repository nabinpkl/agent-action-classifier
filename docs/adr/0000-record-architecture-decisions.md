# ADR-0000: Record architecture decisions

Date: 2026-06-05
Status: Accepted

## Context
Load-bearing architectural, design, and organizational decisions need a durable
home where future contributors — human and agent — find the *reasoning*, not just
the outcome. The repo separates concerns across docs: AGENTS.md is how-to-operate
guidance, PRD.md holds product intent, and SPEC.md holds technical shape.
Decisions need their own append-only record so the "why" survives across sessions
and isn't reconstructed from a diff.

## Decision
Record each significant decision as an ADR in `docs/adr/`, numbered sequentially
(`NNNN-title.md`). Each ADR states **Context**, the **Decision**, and its
**Consequences**, is dated, and is immutable once Accepted — supersede it with a
new ADR rather than editing history. Log decisions made *outside explicit
direction*, with standards-backed reasoning independent of any one prompt.

## Consequences
- The reasoning behind the architecture is greppable and survives contributors.
- AGENTS.md's "Architecture Decisions (ADRs)" section points here instead of
  carrying decision content.
- Small or locally-obvious choices do not need an ADR; reserve it for
  load-bearing ones, to avoid record sprawl.
