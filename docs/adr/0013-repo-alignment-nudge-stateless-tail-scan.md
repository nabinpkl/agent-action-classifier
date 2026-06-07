# ADR-0013: Repo-alignment v1 — soft nudge, evidence-backed review, fail-loud

Date: 2026-06-07
Status: Accepted

## Context
[ADR-0008](0008-repo-agent-alignment-via-hybrid-nudge-hooks.md) chose a hybrid nudge layer
and [ADR-0009](0009-repo-agent-adherence-eval.md) the deterministic-graders-plus-judge eval,
both "implementation deferred until the repo has code." The repo now has code, so this ADR
records the v1 implementation and the decisions that hardened it during research.

Research this session confirmed: Claude Code and Codex hooks have **converged** (same
`PreToolUse`/`Stop` events, the same `hookSpecificOutput.additionalContext` injection, the
same `decision:block` and exit-code semantics). It also surfaced that the path-scoped
alternatives (`.claude/rules`, nested `AGENTS.md`) are **static** and not granular enough for
runtime re-presentation, so a hook is required.

## Decision
A tool-neutral `repo_alignment/` of POSIX-`sh` scripts (consistent with `.githooks/`; `jq`
the one tool-dep), wired repo-wide via `.claude/settings.json` and `.codex/hooks.json`.

1. **Soft nudge, never a gate on the agent.** The `PreToolUse` hook
   (`pre_edit_nudge.sh`) injects a terse, factual, per-skill-area reminder on the **first
   edit of an area per turn** and otherwise stays silent. It NEVER denies an edit (only
   `exit 0`). ADR-0008's nudge-not-gate is preserved. (An earlier idea to hard-block
   deterministic violations was rejected: enforcement belongs to the human, below.)
2. **Stateless debounce by a bounded tail-scan.** "Once per turn per area" is computed by
   scanning the transcript backward from newest, stopping at the first same-area edit (stay
   quiet) or the turn boundary (first edit, nudge), capped at `AA_MAX_SCAN=4000` lines. No
   marker file, no reset. The boundary discriminator is the real Claude transcript shape
   (`type=="user"`, no `toolUseResult`, `promptSource != null`), verified against a live
   transcript. Bench: ~40ms floor, flat ~60ms even at 50k-line transcripts (the cap holds).
3. **Evidence-backed final-message acknowledgment; the human is the gate.** The `Stop` hook
   (`end_of_turn_review.sh`), on a code-touching turn, returns `decision:block` **once**
   (`stop_hook_active`-guarded) to make the agent produce, in its final message: which skill
   nudges fired and HOW each shaped the code (cited decisions), what was changed to comply,
   what was deliberately not changed and why, and any stale-rule surfacing. This blocks only
   the agent's *stop*, never an edit or a user action. The human reading that auditable
   acknowledgment is the "hard block."
4. **Fail loud on our OWN failure (the only hard behavior).** jq absent, unparseable stdin
   (the control-char bug claude-code#53463), a missing/unreadable transcript, or an
   unexpected transcript shape (upstream schema drift) → a user-visible `systemMessage` and
   `exit 1` (non-blocking), NEVER a silent `exit 0`. This aligns the alignment tooling with
   AGENTS.md's own "fail loud / no silent fallbacks"; an earlier fail-open design violated it.
5. **Deterministic graders as a post-hoc audit** (`adherence_graders.sh`, `just adherence`),
   NOT an edit-time gate: banned dumping-ground names and conventional-commit subjects. The
   v1 floor is deliberately small; em-dashes are **not** graded (a chat-response rule, not a
   repo rule — AGENTS.md itself uses them). File-size (pre-commit) and dead-code (clippy) are
   reused, not duplicated.

Hardening idioms folded in (researched): output built with `jq -n --arg` (no escaping-hell /
JSON injection); reminders factual + skill-pointer, not imperative/`SYSTEM:` (which can trip
prompt-injection defenses) — this **supersedes ADR-0008's emphasis-marker guidance**; POSIX
`sh` only (no `<<<`/`[[`) so one script serves both runtimes; `${CLAUDE_PROJECT_DIR}` paths,
a 5s timeout, scripts locate their lib via `${0%/*}` (no `dirname` dependency).

## Consequences
- The hook never blocks real work; the cost of a missed nudge is bounded (graders + the
  acknowledgment catch drift, human as the gate). No over-blocking risk.
- The `Stop` review adds one bounded extra pass per code-touching turn — a real token/latency
  cost and the first thing to measure/tune (threshold by diff size) or self-retire per
  ADR-0009; disable-able in settings.
- The transcript-schema coupling is isolated to one `jq` filter and cap-protected; on drift
  the hook fails loud rather than guessing. The marker-file debounce and a Rust rewrite stay
  in reserve if the bench ever shows the scan is not flat.
- Codex specifics (the `${CODEX_PROJECT_DIR}` var, the `apply_patch` matcher token, whether
  Codex `Stop` honors `decision:block`) are wired but unverified from a Claude session; they
  are validated the first time the repo runs under Codex.
- Deferred (ADR-0009): the LLM judge, the A/B off-vs-on measurement against the literature
  reference, and the model-change regression baseline.
