# ADR-0008: Repo agent alignment via hybrid nudge hooks (nudge, not gate)

Date: 2026-06-06
Status: Accepted (design; implementation deferred until the repo has code)

## Context
This is **dev-experience tooling for this repo**, distinct from the product: keeping
coding agents (Claude/Codex) working in this repo aligned to AGENTS.md and the project
skills. It dogfoods the product's pattern (a deterministic + judge cascade over hooks)
but is a separate concern.

Frontier findings (June 2026) that shape it:
- Static config guidance gets **~25-40% adherence**; the same rules enforced as
  **runtime hooks reach ~95%**. The lever is runtime, not the document.
- A factorial study of 1,650 Claude Code sessions found config-file **structure has no
  detectable effect** on adherence ([2605.10039](https://arxiv.org/abs/2605.10039)). So
  do not over-invest in how skills are written; invest in runtime nudging.
- **Emphasis markers** (IMPORTANT, YOU MUST, NEVER) measurably increase adherence.
- Skills decay over long turns and compaction (long-horizon degradation,
  [SlopCodeBench](https://arxiv.org/pdf/2603.24755)). The cure for decay is
  **re-presentation per turn**, not one-time loading.

## Decision
Add a hybrid **nudge** layer. It is always **non-blocking**: it only injects text
(`additionalContext`/`systemMessage`). The hook is dumb (inserts a reminder); the agent
is smart (acts on it). Two touch points:

- **Pre-edit, per turn (PreToolUse):** on the first edit to a code file, inject the
  targeted skill reminder. PreToolUse knows the file from `tool_input`, so it is always
  specific (never blind). Debounced **once per (turn, skill-area)**, so a mixed Rust+Python
  turn nudges each once and repeated edits do not spam. Phrased with emphasis markers.
- **End-of-turn review (Stop):** only if the turn touched `*.rs`/`*.py`, inject "review
  your changes against AGENTS.md and the skills you used; fix drift, or if the rules look
  stale versus the new architecture, surface that to the user." Doc-only and chat turns
  stay quiet.

Supporting decisions:
- **Reminder text is data, not code** (a filetype->skill map + snippets), so updating a
  nudge is editing text.
- **Bidirectional divergence handling** (operationalizes AGENTS.md's own meta-rules):
  code violates a valid rule -> align the code (prefactor/refactor); code reflects new
  architecture the docs have not caught up to -> surface to the user and update docs,
  do not force the code to match a stale rule; genuine conflict -> ask the user.
- **Drop the blind pre-turn (UserPromptSubmit) anchor**: PreToolUse already knows the
  target before the edit, so guessing from the prompt is unnecessary.

## Consequences
- The mechanism is cheap (text only); the hard part is **restraint** (debounce,
  conditional firing, high-confidence only) so nudges do not train agents to skim, which
  would also dull their attention to real injected context (banner blindness).
- Advisory and bypassable (consistent with [ADR-0003](0003-govern-at-framework-layer-defer-kernel.md));
  admin-managed hooks can pin it ([ADR-0007](0007-pep-via-provider-pretooluse-hooks.md)).
- **Every nudge must have a falsifiable target metric or it is not shipped** (see
  [ADR-0009](0009-repo-agent-adherence-eval.md)); an unmeasurable nudge is speculative cost.
- Implementation is deferred until the repo has code. The end-of-turn doc review is
  useful even now (em-dash, conventional commits, naming).
