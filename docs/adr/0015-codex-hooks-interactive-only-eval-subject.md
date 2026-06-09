# ADR-0015: Codex hooks fire only interactively — eval subject driven via tmux

Date: 2026-06-09
Status: Accepted

Supersedes the subject-runner mechanism of
[ADR-0014](0014-repo-alignment-eval-harness.md) (its §2 "Codex `codex exec`,
headless" and the closing `codex exec` consequence). The rest of ADR-0014
(paired A/B, subject=Codex / judge=Claude, deterministic floor + judge cascade,
paired-delta CI gate) stands unchanged.

## Context

ADR-0014 built the eval on a `codex exec` subject: run a case headless in a
worktree, toggle the nudge by the presence of `.codex/hooks.json`. Building the
live runner exposed that this cannot work: **Codex 0.137 executes no hooks under
`codex exec` or `app-server`.** Probed directly —

- `codex exec` on an apply_patch edit: zero hook events fired (three sentinel
  probes: PreToolUse/apply_patch, UserPromptSubmit, Stop — all silent).
- `codex app-server` (stdio JSON-RPC): `hooks/list` *registers* our hook
  (`preToolUse`, matcher `apply_patch`), but on a real edit turn `hook/started`
  = 0; the hook never ran. No `initialize` capability opts into it.
- **Interactive `codex` (tmux): the hook DOES fire and run to completion** —
  `PreToolUse hook (completed)` with the injected reminder as `hook context`,
  and `Stop` fires too.

So the nudge under test reaches Codex **only** in interactive mode. A `codex
exec` subject made the nudge-ON arm a silent no-op; the A/B would have measured
nothing. (This matches a third-party report that apply_patch is not covered by
PreToolUse in headless Codex, and contradicts the official hooks docs, which are
ahead of the shipped binary.)

Two wiring bugs masked the finding at first and are fixed alongside:
`${CODEX_PROJECT_DIR}` is unset in 0.137 (the hook command must be repo-relative,
which resolves because Codex spawns hooks with cwd = the launch dir); and the
real apply_patch payload carries the patch body in `.tool_input.command`, not the
field the parser assumed.

## Decision

Drive the eval subject as **interactive Codex inside tmux**, unattended, for both
A/B arms (only the nudge differs). A small session library
(`eval/codex_session.sh`) brings up a fresh Codex per case in an isolated
worktree, walks the start-up prompts (pin the version at the update prompt; trust
hooks), dispatches the case brief, monitors the turn to completion, and tears the
session down. `eval/run_codex_case.sh` wraps one (case, arm): worktree up, nudge
toggle, drive, capture (diff vs the base SHA + untracked files + the session
rollout transcript), grade before teardown. `eval/run_eval.sh` runs the bank
paired (OFF then ON per case) and feeds `paired_ci.py`; judge-lane cases go
through `eval/run_judge.sh` (pinned `claude-opus-4-8`).

The version is **pinned**: the bring-up answers the Codex update prompt with
"Skip until next version" so a mid-eval auto-update cannot confound results. An
**unrecognized blocking prompt fails loud** rather than auto-answering a trust
gate.

## Consequences

- The subject is no longer scriptable in one shot; it is a driven tmux session
  per (case, arm) — `cases × 2` interactive sessions per experiment, sequential
  (one at a time), each minutes long. Slower and more fragile than `exec`, but it
  is the only mode where the treatment exists. A reaper clears stale
  sessions/worktrees from a crashed run.
- The harness downstream of the subject is unchanged: `case_grader.sh`,
  `paired_ci.py`, `judge_prompt.md`, and `cases.json` all consume the same diff +
  transcript and did not move. Only the subject-invocation layer changed.
- The capture is robust to the subject committing (diff against the base SHA, not
  the working tree) and to untracked additions (enumerated separately, since the
  banned-name check targets new files a plain diff hides). The nudge toggle's own
  `.codex` move is excluded from the captured diff — it is eval infrastructure,
  not the subject's change.
- Codex non-determinism makes a single run per arm noisy; v0 runs n=1 and pairs
  per case (shared drift cancels in the delta), widening only if the CI cannot
  verdict. A deviated run (subject escalated a gated non-edit command) is
  cancelled, flagged, and excluded — never graded as if it completed.
- Revisit when Codex ships PreToolUse-on-apply_patch in a headless mode; the
  faster `exec` subject becomes available again and this ADR can be superseded.
