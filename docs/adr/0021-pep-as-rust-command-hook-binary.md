# ADR-0021: Realize the PEP as a Rust command-hook binary

Date: 2026-06-11
Status: Accepted

## Context
TASKS #4 is the live PEP: intercept a real agent tool call, decide it, and return the verdict
to the provider before the tool runs. The pieces existed — `decide`, the compiled `Policy`
handle ([ADR-0019](0019-policy-lifecycle-compiled-handle.md)), the four-verdict `Decision`, and
the provider hook contract ([ADR-0007](0007-pep-via-provider-pretooluse-hooks.md)). The open
question was the **integration shape**: how the held policy meets a per-tool-call hook.

Two shapes were considered and the latency was measured this session (not assumed):

- **Per-call binary** (a command hook spawning a process each call): pays a ~2.1ms process-spawn
  floor + ~0.3ms policy parse every call; end-to-end **~5ms measured** on the governed path
  (spawn + plane compile + decide), and cheaper on the common ungoverned path (no plane load).
- **Warm-handle HTTP sidecar** (a long-running service holding the compiled `Policy`): ~0.2-0.4ms
  per call (loopback + ~15µs decide), but it is a daemon to run and supervise, and Claude's
  `type:http` hook is **hard-wired fail-open** (a down/slow sidecar lets the tool through
  unguarded, with no setting to change it).

The decide itself is ~15µs and irrelevant to the choice; the cost is process spawn + policy
parse per call, which only the sidecar amortizes.

Probe findings that bound the design (verified on Claude Code 2.1.173, Codex 0.139.0):
- A **command hook is one binary that serves both providers** — Claude and Codex PreToolUse
  payloads converged on `tool_name` + `tool_input`, and **exit 2 + stderr blocks on both**.
- **`codex exec` fires no hooks at all** (interactive Codex fires for `Bash` and `apply_patch`),
  so autonomous `codex exec` agents are ungovernable by hooks — a constraint, not a bug here.
- A **nested git repo isolates hooks** for both providers, giving a safe live sandbox
  (`experiments/`) that does not touch the dev session.

## Decision
1. **The PEP is a Rust command-hook binary, `enforce` (`crates/policy_enforcement`), invoked per
   tool call.** It reads the PreToolUse payload on stdin, normalizes it to the canonical action,
   compiles the plane + `decide`s, and returns the verdict: allow = exit 0, deny = exit 2 +
   reason on stderr, escalate = `permissionDecision:"ask"` JSON on stdout. **One binary serves
   both Claude and Codex.** This is the impure edge (stdin JSON + exit codes) over the serde-free
   core, parallel to how `policy_decision_py` is the FFI edge.
2. **This re-founds SPEC's "the PEP lives in the Python host."** The enforcement path is Rust over
   the pure core — no Python interpreter startup per call, no FFI on the hot path. The Python host
   (`agent_action_classifier`) is retained for the programmatic/LangGraph FFI and the future LLM
   judge; it is simply not on the enforcement path. (Chosen over a Python command hook, which
   measured ~24ms/call from interpreter + import startup.)
3. **It fails CLOSED.** Any internal error (bad args/payload/plane/config) maps to an exit-2 deny
   with a loud reason, never a silent fail-open — the security-correct default, available
   *because the binary owns its exit code* (Claude `type:http` cannot). An **out-of-scope** call
   (an ungoverned tool kind, or a path that maps to no `DataScope`) proceeds (exit 0): it touches
   nothing the org declared as a resource, so there is nothing to govern; this short-circuits
   before any policy load, keeping the live hook cheap and low-noise.
4. **Normalization is the novel part** (no industry standard for the tool-call -> action schema):
   v0 governs **mutation tools only** (`Write`/`Edit`/`MultiEdit`/`apply_patch` -> `Write`); the
   target file path maps to a `DataScope` id via a `resource_map.json` (glob -> scope, first
   authored glob wins). Reuses the `corpus/asi05` plane, so the demo needs **zero new policy**:
   a secret-mapped path is denied, a restricted/config path asks, everything unmapped proceeds.
5. **The live sandbox is a nested git repo** (`experiments/`, probe-verified isolation); the
   durable wiring snippet lives in the `provider-hooks` skill, the live instance in the sandbox.

## Latency, and the HTTP-sidecar roadmap
| Shape | Spawn/call | Parse/call | End-to-end (measured) |
|---|---|---|---|
| Python command hook | ~24ms (interpreter+import) | ~0.3ms | ~24ms |
| **Rust binary (chosen)** | ~2.1ms | ~0.3ms | **~5ms** (governed path) |
| HTTP sidecar (warm, native http) | 0 | 0 | ~0.2-0.4ms |

~5ms is negligible against any real tool call (a 500ms-2s LLM call, or even a ~10ms local
action). **The warm-handle HTTP sidecar stays the roadmap** ([ADR-0007](0007-pep-via-provider-pretooluse-hooks.md)'s
preferred wire): switch when per-call **rate** makes the ~2ms spawn material, or the **policy set**
grows enough that the ~0.3ms per-call parse hurts. The sidecar would reuse the same compiled
`Policy` and the same normalization; only the transport changes.

## Consequences
- **One artifact, no daemon, fail-closed-capable** — simpler than the sidecar's service +
  per-provider client, and safer than `type:http` on the failure path.
- **`codex exec` agents are ungovernable** by this (or any) hook; governing them needs interactive
  mode, Codex *managed* hooks, or a non-hook interception point.
- **The framework-layer caveat persists** ([ADR-0003](0003-govern-at-framework-layer-defer-kernel.md)):
  a call not routed through the tool layer escapes. Managed/kernel enforcement is the only true
  fix and stays deferred.
- **Reinforces [ADR-0019](0019-policy-lifecycle-compiled-handle.md):** the per-call binary cannot
  hold a warm handle (each call is a fresh process), which is exactly why the sidecar — a process
  that *persists* — is the latency roadmap.

## Deliberately deferred (each with a revisit trigger)
- **The warm-handle HTTP sidecar.** Trigger: per-call rate or policy-set size makes spawn+parse
  material (see the table).
- **The LLM judge on `escalate`.** v0 returns `permissionDecision:"ask"` (native human dialog);
  the judge that resolves the semantic lane is TASKS #6. Trigger: that task.
- **The decision-log audit sink.** The binary has the `Decision` in hand but does not persist it
  yet. Trigger: TASKS #5.
- **Read/Bash/share/delete resolvers.** v0 governs mutation tools only; a Bash command's action
  and resource need command-line parsing (a separate rabbit hole). Trigger: a policy needs to
  govern a non-mutation tool.
- **Per-call provenance id.** `raw_payload_id` is the session id as a placeholder; the audit log
  (TASKS #5) will carry a true per-call id.
- **Managed/kernel enforcement** for true non-bypassability. Trigger: the framework-layer caveat
  becomes load-bearing (a real adversary, not a cooperating dev agent).
