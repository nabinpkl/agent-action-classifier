# ADR-0009: Repo agent-adherence eval (with model-change regression guard)

Date: 2026-06-06
Status: Accepted (design; implementation deferred until the repo has code)

## Context
The nudge layer ([ADR-0008](0008-repo-agent-alignment-via-hybrid-nudge-hooks.md)) must be
able to prove it helps rather than harms. This needs an eval whose subject is **agents
operating in this repo** and whose measure is **adherence to this repo's AGENTS.md +
skills**, distinct from the product's classifier eval (though the same shape, the
dogfooding spine).

Frontier findings (June 2026):
- **Regression on model change is the canonical silent failure**: upstream provider
  updates change behavior with no code change. The named practice is **Continuous Prompt
  Regression** (run the eval suite whenever instructions, model weights, or context
  params change).
- Evals should be **layered**: deterministic graders for what code can confirm,
  LLM-as-judge for nuance, humans for calibration.
- **The judge itself drifts** ("evaluating the evaluator"): if the judge model silently
  updates, the baseline moves under you.

## Decision
Define a repo agent-adherence eval:

- **Metric:** adherence rate on countable rules via **deterministic graders** (banned
  names, thiserror at the lib boundary, em-dashes, ~500-line files, conventional
  commits, dead code). An **LLM judge** scores the semantic rules (does the abstraction
  earn its place). These are the two regimes: deterministic = exact-match conformance,
  judge = graded.
- **Method:** A/B, nudges **off** (baseline) vs **on** (treatment), reported as a delta
  against the literature reference (static ~25-40%, hook ~95%) per
  [ADR-0006](0006-reference-or-frontier-measurement.md); unmeasurable dimensions tagged
  `frontier`. **The eval is allowed to kill the nudge feature.**
- **Falsifiability per nudge:** each nudge carries the specific violation it must reduce;
  nudges that do not move their metric **self-retire**. Measure the **artifact**, never
  the acknowledgment; the primary harm signal is the **ignored-despite-nudged rate**
  (a nudge fired and the violation survived = pure cost).

### Model-change regression guard
- Baseline is **versioned per (model version, corpus version, judge version)**.
- **Offline** run on a deliberate model upgrade; **sampled online** re-runs to catch
  silent provider drift.
- The **deterministic backbone gates** (it cannot drift, so it is the trustworthy
  regression signal); the judge informs until calibrated.

### Judge discipline (the trap)
Pin and version the judge model, run it at temperature 0, calibrate against a frozen
human gold set before gating, and **never change the judge and the target at the same
time**.

### Regress grounding (who evaluates the evaluator)
Terminate the regress on **non-drifting anchors**, never another model:
1. **Deterministic graders** (inspectable code, self-grounding, no evaluator needed).
2. A **frozen human-labeled gold set** (definitional ground truth).
3. **Known-answer canaries** embedded in the judge eval (flagrant violation / flagrant
   compliance) that fail loudly if the judge drifts.
Rule: if tempted to add a model to check the model that checks the model, **stop and push
the check down to a fixed floor**. Maximize the deterministic floor; every rule made a
code-based grader is one fewer rung on the tower.

## Consequences
- Reuses the product's conformance-corpus pattern for repo-rule adherence, built once.
- **Coverage caps** what regressions are caught; refresh the gold set from real failures.
- Small N on a personal repo -> directional signal; deterministic regressions are crisp,
  judge-layer regressions are `frontier`.
- Implementation deferred; the reference numbers (25-40% static, 95% hook) are pinned now
  so the eval has a yardstick from day one.
