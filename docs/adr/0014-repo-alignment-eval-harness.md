# ADR-0014: Repo-alignment eval harness — paired A/B, Codex subject, Claude judge

Date: 2026-06-07
Status: Accepted

## Context
[ADR-0009](0009-repo-agent-adherence-eval.md) decided the repo-alignment eval
(deterministic graders + LLM judge, A/B nudges off vs on, eval allowed to kill a
nudge) but deferred implementation until the hooks existed. The hooks now exist
([ADR-0013](0013-repo-alignment-nudge-stateless-tail-scan.md)), so this ADR records
the v1 harness.

June 2026 research framed the design:
- The closest prior art ([arxiv 2601.20404](https://arxiv.org/abs/2601.20404)) is a
  **paired A/B** of AGENTS.md presence — 10 repos, 124 PRs, each task run twice,
  one variable toggled, isolated containers — but it measures **efficiency only**
  (28.6% median faster, 16.6% fewer output tokens) and explicitly defers
  **adherence**. That deferred half is exactly this eval.
- Settled prompt-regression practice ([futureagi 2026](https://futureagi.com/blog/prompt-regression-testing-2026/)):
  a **classifier cascade** (deterministic first, frontier judge only where needed)
  scored by a **paired-delta bootstrap 95% CI**, gated like CI.
- **Agent-as-a-Judge** ([arxiv 2508.02994](https://arxiv.org/abs/2508.02994)): judge
  the trajectory, not just the end state.

## Decision
Run the eval as **separate, falsifiable experiments**, each toggling one variable
over a shared case bank, scored by a paired-delta CI that can ship or kill a nudge.

1. **Case bank as external data** — `repo_alignment/eval/cases/cases.json`: ~12 small,
   real tasks (mirroring the prior art's ≤100 LoC controlled changes), each authored
   around one known adherence pitfall in this repo. Reused across experiments
   (mirrors `corpus/asi05/` externalization). Each case declares `lane`:
   `deterministic` (machine-checkable on the diff) or `judge` (judgment-level).
2. **Subject = Codex, judge = Claude.** The graded agent (Codex `codex exec`,
   headless, one isolated `git worktree` per run) is never its own judge; the judge
   is a **pinned** `claude-opus-4-8` at temp 0 (`eval/judge_prompt.md`). This removes
   the self-evaluation bias ADR-0009 warns about and live-verifies the `.codex`
   wiring ADR-0013 left untested.
3. **Paired A/B, one variable.** Per experiment, every case runs twice with only the
   tested variable flipped (E1: nudge off vs on, via presence of `.codex/hooks.json`
   in the worktree). Everything else is held constant, so the paired delta isolates
   the effect (the prior art's design, retargeted to adherence).
4. **Deterministic floor + judge cascade.** `eval/case_grader.sh` scores
   deterministic cases to a `{violated:0|1}` bit (the non-drifting floor);
   judge-lane cases bundle diff + transcript for the pinned judge. POSIX `sh`
   graders (consistent with the hooks); the analyzer is Python on the existing host.
5. **Paired-delta bootstrap CI gate** — `eval/paired_ci.py`, **pure stdlib** (no
   numpy dependency at v0), seeded for reproducibility. Verdict: CI entirely > 0 →
   **SHIP**; straddles 0 → **SELF-RETIRE** (ADR-0009 kills a nudge that doesn't move
   its metric); entirely < 0 → **HARMFUL**. We measure the **artifact**, not the
   acknowledgment (ADR-0009).
6. **Experiments, run first, each falsifiable:** E1 pre-edit nudge reduces target
   violations; E2 wording — factual-pointer (ADR-0013) vs imperative+emphasis (the
   2026 AGENTS.md guides) — **empirically resolving the ADR-0013-vs-literature
   contradiction** rather than asserting a side; E3 the Stop review changes the
   artifact, not just the text; E4 the review's cost vs benefit. E1 is the vertical
   slice built first.

## Consequences
- `repo_alignment/` separates by concern into two vertical slices: `hooks/` (the
  runtime nudge system — pre-edit, stop-review, lib, skill-map, bench) and `eval/`
  (offline measurement — cases, grader, analyzer, runner, judge, the deterministic
  audit). The two do not import each other (loose coupling); the wiring in
  `.claude/settings.json` / `.codex/hooks.json` points at `hooks/`.
- The harness is build-once, configure-per-experiment; adding E2/E3 is new configs
  over the same runner/grader/analyzer, not new machinery.
- The deterministic spine (cases, grader, analyzer) is testable with synthetic diffs
  and was proven before any live run; the only machine-coupled, costed step is the
  Codex invocation.
- The judge can drift; ADR-0009's discipline holds — pinned+versioned judge, temp 0,
  the deterministic floor as the non-drifting anchor, never moving judge and expected
  verdict together.
- Cost is real: each experiment is `cases × 2` Codex runs. The case set is kept
  small (~12) at v0; broadening it is a deliberate, logged step, not a silent default.
- Wiring `codex exec` couples the harness to Codex CLI flags (`-C`,
  `--sandbox workspace-write`, `--dangerously-bypass-hook-trust`); on a CLI change the
  runner fails loud rather than silently mis-running.
