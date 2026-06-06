---
name: provider-hooks
description: How Claude Code and Codex pre-execution hooks work and how the PEP maps onto them (PreToolUse, deny/ask, HTTP-hook PDP wire, repo-wide + managed config). Use when building or reasoning about the enforcement adapter (PEP) for either CLI.
---

# Provider hooks (the PEP surface)

The PEP is realized as a provider **pre-execution tool hook** (see
[ADR-0007](../../../docs/adr/0007-pep-via-provider-pretooluse-hooks.md)). Claude Code and
Codex have converged on nearly the same model, so the adapter is thin. Remember
[ADR-0003](../../../docs/adr/0003-govern-at-framework-layer-defer-kernel.md): this layer
is advisory and bypassable; managed hooks tighten it but do not harden it.

Sources: [Claude Code hooks](https://code.claude.com/docs/en/hooks),
[Codex hooks](https://developers.openai.com/codex/hooks).

## The shared shape

- **`PreToolUse` fires before the tool runs and can block.** Input JSON on stdin
  includes `tool_name`, `tool_input` (the full `command`/args), `cwd`, `session_id`.
  The hook sees the complete command before execution.
- **Block via** `permissionDecision: "deny"` (with a reason) in JSON output, or exit
  code `2` (stderr shown). Exit `0` with no output = no decision (normal flow).
- **Rewrite** the call via `modifiedInput` (Claude) / `updatedInput` (Codex).
- **`PostToolUse` fires after and cannot block**, so it is the home for observe/flag.

## Verdict -> hook output

| verdict | output | gate (EU AI Act Art 12) |
|---|---|---|
| allow | `permissionDecision: "allow"` | - |
| deny | `permissionDecision: "deny"` + `permissionDecisionReason` (rationale) | hard |
| escalate | `permissionDecision: "ask"` (native human dialog) | soft |
| flag | PostToolUse log entry | soft |

Feed the rationale back to the agent via `additionalContext`.

## Claude Code specifics

- Config: `.claude/settings.json` (**commit for team/org-wide**), with
  `.claude/settings.local.json` (gitignored) and **managed policy settings** (admin,
  org-wide) above it.
- `matcher` filters by tool (`"Bash"`, `"Bash|Edit|Write"`, regex, `mcp__server__.*`);
  an `if` field uses permission-rule syntax (`Bash(rm *)`) that strips leading env
  assignments, checks each `&&` subcommand, and looks inside `$(...)`.
- Hook types: `command` (shell, JSON on stdin), **`http`** (POST JSON to a URL, get a
  decision, the clean PDP wire), `mcp_tool`, `prompt`, `agent`.

## Codex specifics

- Config: `~/.codex/config.toml` `[hooks]` or `~/.codex/hooks.json` (user), and
  `<repo>/.codex/config.toml` or `<repo>/.codex/hooks.json` (**project-local, resolved
  from the git root**). Codex merges all matching layers.
- Stdin shares `session_id`, `cwd`, `hook_event_name`, `model`, plus `tool_name`/
  `tool_input` (and `tool_response` post-exec). `PreToolUse` runs before `Bash`,
  `apply_patch`, and MCP calls.
- Admin enforcement: `requirements.toml` `[features].hooks = false` disables hooks;
  `allow_managed_hooks_only = true` runs only enterprise-managed hooks and bypasses
  user/project/plugin hooks (the org-supremacy teeth).

## The PDP wire

Prefer the **HTTP hook to a local PDP service**: one endpoint receives the canonical
action and returns `deny`/`ask`/`allow`, instead of N per-tool shell scripts. This is
the sidecar shape. For LangGraph, call the PDP in-process; use a `command` hook only
where HTTP is unavailable.
