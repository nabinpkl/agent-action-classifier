# ADR-0026: pipe-to-shell graduates from ask to a hard forbid

Date: 2026-06-16
Status: Accepted

Builds on [ADR-0023](0023-host-derives-attributes-cedar-decides.md) (host classifies a command
into `context.command.kind`, Cedar decides on the kind) and
[ADR-0024](0024-escalate-is-provider-specific-codex-degrades-to-block.md) (escalate/ask is
provider-specific). ADR-0023 shipped all three command kinds as `requires_approval`; ADR-0024's
policy comment already named pipe-to-shell "the first candidate to graduate from ask to a hard
forbid." This ADR makes that graduation.

## Context
The three governed command kinds are not equally risky:

- `package_install` (npm/pip/cargo/brew install) and `ephemeral_exec` (npx/uvx/pipx run) execute
  code, but from a **named, registry-resolvable** artifact a human can recognize and approve
  ("yes, install lodash"). Approval is meaningful: the human is deciding about a *known* thing.
- `pipe_to_shell` (`curl ... | sh`, `wget ... | bash`, `sh -c "$(curl ...)"`) executes **whatever
  bytes the URL returns at fetch time** — no registry, no version pin, no provenance, no name to
  reason about. The content is unknown until it has already run, and can differ between the
  approval moment and the execution moment (server-side swap). Approving it is approving a blank
  cheque.

So an `ask` on pipe-to-shell is a false choice: the human has nothing concrete to evaluate. The
honest verdict is a hard deny, with the safe path being "fetch the script, read it, then run the
local file" (which classifies as nothing and passes).

## Decision
1. **`pipe_to_shell` becomes a Cedar `forbid`** (`@id("forbid-pipe-to-shell")`), replacing the
   `approve-pipe-to-shell` permit. Like `deny-secret-write` it carries no `@outcome`/`@lane`: a
   forbid is a terminal hard deny (`Deny`/`Hard`), unoverridable by approval (deny-overrides).
2. **The other two kinds stay `requires_approval`.** This decision is specific to the
   no-provenance kind, not a blanket hardening.
3. **The deny is cross-provider, which closes the Codex asymmetry for this kind.** Per ADR-0024
   an escalate degraded to a block on Codex (no ask dialog); a `forbid` needs no degradation
   because exit-2 + stderr deny is honored by Claude and Codex identically. So pipe-to-shell is
   now blocked the same way everywhere, with a `DENY [ASI05/forbid-pipe-to-shell]` reason rather
   than an `APPROVAL REQUIRED` one.

## Consequences
- **A `curl | sh` is blocked outright on every provider**, with no approve path. The remediation
  is to fetch-then-read-then-run, which the classifier does not flag.
- **Conformance proves it cannot be approved around**: a `pipe_to_shell` case with a valid in-scope
  approval still denies (`pipe_to_shell_is_unoverridable_by_approval`), mirroring the secret-write
  invariant. The PEP black-box corpus asserts the cross-provider deny on both Claude and Codex.
- **One fewer provider-specific path.** The escalate/ask provider branch (ADR-0024) still exists
  for `package_install`/`ephemeral_exec`; pipe-to-shell no longer exercises it.

## Deferred (with revisit trigger)
- **An allowlist of trusted install URLs** (e.g. `https://sh.rustup.rs`) that could downgrade a
  specific pipe-to-shell back to approval. Not in v0 — an allowlist is its own provenance system.
  Trigger: a recurring, genuinely-trusted bootstrap script makes the blanket forbid too coarse.
