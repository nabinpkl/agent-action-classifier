# ADR-0024: The escalate/ask hook response is provider-specific; Codex degrades to a block

Date: 2026-06-13
Status: Accepted

## Context
[ADR-0021](0021-pep-as-rust-command-hook-binary.md) realized the PEP as one `enforce` binary and
claimed it "serves both Claude and Codex" because their PreToolUse payloads converged and **exit 2
+ stderr blocks on both**. The shell-command lane ([ADR-0023](0023-host-derives-attributes-cedar-decides.md))
made escalate (`requires_approval`) the common verdict, and escalate is returned as Claude's
`hookSpecificOutput.permissionDecision:"ask"` JSON on stdout.

A live interactive Codex 0.139 session (the `experiments/` sandbox, governing Bash) refuted the
blanket claim **for the escalate path**: Codex fired the hook and the binary classified the command
correctly, but Codex **rejected the response** — `PreToolUse hook (failed) error: PreToolUse hook
returned unsupported permissionDecision:ask` — and blocked the command as a hook *error*. So the
three response paths split by provider:

- **deny** (exit 2 + stderr): cross-provider. Both honor it.
- **proceed** (exit 0): cross-provider.
- **`permissionDecision:"ask"`**: **Claude-only.** Codex rejects it; an unknown provider is unproven.

(Codex *does* accept `permissionDecision:"deny"`, so the incompatibility is specific to `"ask"`.)
Recorded in session memory as the verified environment fact.

## Decision
1. **`hook_response::from_decision` takes the provider** (a `Provider` enum: `Claude | Codex |
   Other`, parsed from `--provider`). Only the **escalate** arm branches; deny and proceed are
   unchanged (they are genuinely cross-provider).
2. **Escalate on a provider with no ask dialog degrades to a clean exit-2 block** with an
   `APPROVAL REQUIRED [clause/id]: <rationale>` stderr, distinct from a hard `DENY`. Claude keeps
   the `ask` dialog. `Other` (unrecognized provider) degrades to a block too — the conservative
   default: never emit an `ask` a provider might reject or ignore (a silent fail-open).
3. **This qualifies [ADR-0021](0021-pep-as-rust-command-hook-binary.md)'s "one binary serves both"**:
   the binary still serves both, but the *escalate response is provider-specific*. The single
   artifact and the deny/proceed paths stand.

## Consequences
- **No fail-open and no hook error on Codex escalations.** A `requires_approval` shell command
  (npm/pip/cargo install, npx, curl|sh) is *blocked* on Codex with a readable reason, rather than
  erroring or slipping through. The human gate is preserved; only its UX differs (block vs dialog).
- **Asymmetric UX by provider, by necessity.** On Claude the agent can proceed after the human
  approves the dialog; on Codex the human must act out-of-band (grant an approval in `context`, or
  the operator re-runs) since there is no in-hook approve. This is a provider limitation, not a
  policy choice — the verdict (`Escalate`) is identical; only its rendering differs.
- **`Provider` is the one place new providers are added.** A future provider that supports an ask
  dialog gets an arm + `supports_ask() == true`; one that does not inherits the safe block.
- Guarded by a unit test (escalate asks on Claude, blocks on Codex/Other) and a binary conformance
  case (`--provider codex` -> exit 2 + `APPROVAL REQUIRED`).

## Deliberately deferred (each with a revisit trigger)
- **A native Codex approve path.** If Codex gains an in-hook ask/approve mechanism, escalate on
  Codex should use it instead of degrading to a block. Trigger: Codex ships an ask-equivalent.
- **Out-of-band approval flow for blocked escalations.** v0 leaves the human to grant an approval
  in `context` (which lifts `requires_approval` to allow) or re-run. Trigger: blocked-then-approve
  becomes a frequent loop worth smoothing.
