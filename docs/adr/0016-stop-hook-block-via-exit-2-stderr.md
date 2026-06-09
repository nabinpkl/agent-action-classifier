# ADR-0016: Stop-hook block via exit 2 + stderr (cross-runtime)

Date: 2026-06-09
Status: Accepted

Supersedes the **Stop-hook emit mechanism** of
[ADR-0013](0013-repo-alignment-nudge-stateless-tail-scan.md) §3 (the `decision:block`
JSON output). Everything else in ADR-0013 stands: the soft PreToolUse nudge, the
stateless tail-scan, the once-guard, the evidence-backed acknowledgment intent with
the human as the gate, and fail-loud-on-our-own-failure. Resolves the open item ADR-0013
flagged ("whether Codex `Stop` honors `decision:block` ... wired but unverified"), and
follows from [ADR-0015](0015-codex-hooks-interactive-only-eval-subject.md).

## Context

ADR-0013's Stop hook elicits the end-of-turn acknowledgment by returning, once,
`{decision:"block", reason, hookSpecificOutput:{hookEventName:"Stop"}}` with `exit 0`,
relying on ADR-0013's premise that Claude and Codex had **converged** on the same hook
schema. Verifying the hooks live on Codex 0.137 (the eval subject, ADR-0015) broke that
premise for the Stop output:

- The Stop hook first **silently no-opped** on Codex because its transcript tail-scan only
  parsed Claude's schema. Fixed separately (commit c213446): `aa_turn_tuples` now also
  parses the Codex rollout, so it detects the touched areas and emits the review.
- Once detection worked, Codex **rejected the output**: "Stop hook (failed): hook returned
  invalid stop hook JSON output." Codex 0.137 does not accept the `decision:block` JSON
  shape, even though the Codex hooks docs document it (the docs are ahead of the shipped
  binary; the binary validates against a different `HookOutputEntryKind` schema).

Both the Codex docs and Claude's hook contract document a **second** continuation
mechanism: **exit code 2 with the reason on stderr.** Unlike the JSON shape, this one the
0.137 binary honors, and it is also Claude's canonical block convention.

## Decision

The Stop hook blocks-to-continue by writing the review reason to **stderr** and exiting
**2** (`printf '%s' "$REASON" >&2; exit 2`), not by emitting JSON. One cross-runtime code
path, no per-runtime branch and no mode flag (so it stays inside AGENTS.md's "no toggle
flags that change semantics").

This refines the hook's exit-code policy without changing its posture:

- `exit 0` — proceed/quiet. PreToolUse may add an `additionalContext` nudge here (unchanged);
  Stop emits nothing.
- `exit 2` (**Stop only**) — block-to-continue: stderr is fed back as a new continuation
  prompt so the agent produces the acknowledgment. This is **not** a denial of an agent
  action (the agent continues), so ADR-0008/0013 nudge-not-gate is preserved. PreToolUse
  still never uses `exit 2` (it never denies an edit).
- `exit 1` — our own failure (jq absent, unparseable input, schema drift): a loud,
  non-blocking `systemMessage` (unchanged from ADR-0013).

## Consequences

- The acknowledgment elicitation now works on **both** runtimes from one path. Verified live
  on Codex 0.137: a rust-edit turn shows "Stop hook (blocked) / feedback: This turn changed:
  rust ..." and Codex produces the evidence-backed self-review (which nudges fired and how
  each shaped the code, what changed vs deliberately did not). This unblocks the E3
  experiment (does the Stop review change the artifact) on the Codex subject.
- **No infinite loop**, and on Codex the terminator is not `stop_hook_active`: the exit-2
  continuation prompt becomes a **new user-turn boundary** in the transcript, so the next
  Stop's bounded tail-scan finds no edit after that boundary and exits 0 quiet. On Claude the
  `stop_hook_active` guard still applies as a second line of defense.
- Loses the structured `hookSpecificOutput` channel on Stop (stderr is plain text), which is
  acceptable: the Stop hook only needs to inject one continuation reason, and plain stderr is
  the more stable contract across runtime versions.
- Couples the Stop block to the exit-2 convention; if a future runtime drops exit-2
  continuation the hook fails loud (the review just would not appear) rather than mis-firing.
  Revisit if Codex aligns its shipped Stop JSON schema with its docs, at which point a single
  JSON emit could serve both again.
