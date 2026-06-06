# ADR-0007: Realize the PEP as provider pre-execution tool hooks

Date: 2026-06-06
Status: Accepted

## Context
Framework-layer governance ([ADR-0003](0003-govern-at-framework-layer-defer-kernel.md))
needs a concrete enforcement point (the PEP). Research (June 2026) into Claude Code
and Codex hooks found the two CLIs have **converged on a nearly identical hook model**:
a `PreToolUse` event that fires before a tool runs, receives `tool_name` + `tool_input`
(the full command/args) on stdin, can **block** via `permissionDecision: "deny"` or exit
code `2`, can rewrite the call (`modifiedInput`/`updatedInput`), is configured
**repo-wide** in committed files resolved from the git root, and can be **force-enabled
by an admin** (Claude managed policy settings; Codex `allow_managed_hooks_only`). The
same conceptual point exists in LangGraph (a pre-tool callback/interrupt).

## Decision
- **The PEP is a provider pre-execution tool hook.** It receives the tool call before
  execution, normalizes it to the canonical action, calls the PDP, and returns a
  decision. This is one conceptual interception point realized per provider: a Claude/
  Codex `PreToolUse` hook, or a LangGraph pre-tool callback.
- **Integration wire = the HTTP hook to a local PDP service** where available (Claude
  `type: "http"`, a Codex endpoint), one endpoint instead of N per-tool shell scripts.
  This makes the sidecar distribution shape the natural PEP wire; an in-process call is
  used for LangGraph, a `command` hook is the fallback.
- **Verdict mapping** (also the EU AI Act hard/soft-gate split):
  | verdict | hook output | gate |
  |---|---|---|
  | allow | `permissionDecision: "allow"` | - |
  | deny | `permissionDecision: "deny"` + `permissionDecisionReason` (= rationale) | hard |
  | escalate | `permissionDecision: "ask"` (native human dialog) | soft |
  | flag/observe | PostToolUse log (cannot block post-exec) | soft |
  `additionalContext` carries the rationale back to the agent.
- **Org-supremacy teeth at the framework layer:** distribute the hook via committed
  `.claude/settings.json` / `<repo>/.codex/` and pin it with admin-managed policy so a
  local user cannot override it. (Bounded by the advisory/bypassable caveat below.)
- **The canonical action is modeled on the shared hook payload** (`tool_name`,
  `tool_input`/command, `cwd`), which fits LangGraph, Claude, and Codex alike. The
  convergence de-risks the cross-provider normalization thesis: a thin adapter suffices.
  This does not change v0's first build target; it confirms the schema is
  provider-neutral, not LangGraph-specific.

## Consequences
- The PEP adapter is thin; per-provider work is mapping a converged payload, not
  bespoke modeling.
- [ADR-0003](0003-govern-at-framework-layer-defer-kernel.md)'s caveat persists: hooks are
  advisory and bypassable (an action not routed through the tool layer, or a non-managed
  setup, escapes). Admin-managed hooks tighten this but do not make it a hardened
  boundary.
- Practical wiring details live in the `provider-hooks` skill, not here.
