# ADR-0006: Every result carries a reference baseline or is tagged "frontier"

Date: 2026-06-06
Status: Accepted

## Context
This is a learning project whose success is *measured*, not asserted (ADR-0001), and
whose enforcement architecture sits in an unsettled space (ADR-0004). A benchmark with
no point of comparison is easy to misread as good or bad by inventing a target. The
discipline needs to make honesty about baselines explicit.

## Decision
**Every benchmark or result either cites a reference baseline and reports the delta, or
is tagged `frontier`.** A `frontier` tag means no published baseline exists, others are
still figuring it out too, and the deliverable is a *characterization of the tradeoff*,
not hitting a target. No silently-invented "good" numbers.

Examples of referenced results: stage-1 latency vs Microsoft's <0.1ms p99 inline
figure; PyO3 FFI overhead vs published crossing costs (pydantic-core, polars); judge
agreement vs ASSERT / human-to-human ~90%. Examples of genuinely `frontier` results:
escalation rate (fraction resolving deterministically vs escalating) for an org policy;
the latency/accuracy tradeoff of the layered early-exit shape; the effect of
org-supremacy-plus-scoped-approval on judge accuracy.

## Consequences
- Results are honest about where they stand relative to prior art.
- The `frontier` tag is the measurement-level expression of ADR-0004's "architecture is
  open": it marks the places this project is genuinely exploring, not replicating.
- Reviewers and the eventual writeup can trust a number's framing without re-deriving its
  context.
