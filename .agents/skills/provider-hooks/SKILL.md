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

## Verified behavior (probed 2026-06-11: Claude Code 2.1.173, Codex 0.139.0)

- **Hook config resolves at the git root** for both CLIs. A nested git repo isolates hooks:
  `experiments/.git` makes `experiments/` its own root, so its `.claude`/`.codex` govern only
  sessions launched there (the safe live sandbox). `${CLAUDE_PROJECT_DIR}` = that git root;
  `CODEX_PROJECT_DIR` is unset (use a cwd-relative or absolute hook command path).
- **One `command` hook binary serves both providers:** `tool_name` + `tool_input` converged, and
  **exit 2 + reason on stderr blocks on both**. Codex `apply_patch` carries the patch text in
  `tool_input.command` (path in `*** (Add|Update|Delete) File:` headers); Claude `Edit`/`Write`/
  `MultiEdit` carry `tool_input.file_path`.
- **`codex exec` fires NO hooks** (re-probed; the v0.137 limit persists). Interactive Codex fires
  PreToolUse for `Bash` and `apply_patch` — the apply_patch interception bug (openai/codex#16732)
  is fixed at 0.139. Codex project hooks are trust-gated by hash (`[hooks.state]`); editing the
  hooks.json re-prompts. PreToolUse-deny does NOT accept `additionalContext` on Codex.
- **Claude `type:http` fails OPEN** (a down/slow/erroring sidecar lets the tool through, no setting
  to change it). This is *why the PEP is a `command` binary, not `type:http`*: a binary owns its
  exit code, so it can fail CLOSED (deny) on error.

## The PDP wire (realized: the `enforce` command-hook binary, ADR-0021)

The PEP is `enforce` (`crates/policy_enforcement`): one binary, invoked as a `command` hook by
both providers, that normalizes the payload and returns allow (exit 0) / deny (exit 2 + reason) /
ask (`permissionDecision` JSON). It fails closed on internal error. Wire it by absolute path with
the plane + resource map + agent id as flags:

```jsonc
// .claude/settings.json (Claude) — matcher for the governed mutation tools
{ "hooks": { "PreToolUse": [ { "matcher": "Write|Edit|MultiEdit", "hooks": [ { "type": "command",
  "command": "/abs/target/release/enforce --plane /abs/corpus/asi05 --resource-map /abs/corpus/asi05/resource_map.json --agent-id agent-eng --provider claude",
  "timeout": 10 } ] } ] } }
// .codex/hooks.json (Codex) — same binary, matcher "apply_patch", --provider codex
```

The **warm-handle HTTP sidecar** (this section's former recommendation) stays the *roadmap* for
when per-call rate or policy-set size makes the ~2ms spawn + ~0.3ms parse material; it would reuse
the same binary's normalization and the compiled `Policy`, changing only the transport. For
LangGraph, call the PDP in-process (no hook).
