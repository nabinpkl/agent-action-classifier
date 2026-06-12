# ADR-0023: Host derives attributes, Cedar decides on them (never parse raw strings in policy)

Date: 2026-06-12
Status: Accepted

## Context
Two open v0 questions drove a survey of how the industry governs agent actions with Cedar:
how to govern a **Bash/shell action** (the headline "an agent silently installs a package"
case, deferred by [ADR-0021](0021-pep-as-rust-command-hook-binary.md) as command-line parsing),
and how the **central plane** distributes and propagates rules. The survey found the
architecture this project arrived at independently is now the converging industry pattern
(pre-tool-call deterministic authorization, Cedar, deny-by-default, a reference monitor outside
the agent loop), and it answers both questions the same way.

Surveyed, June 2026:
- **AWS Bedrock AgentCore Policy + Cedar** ([why-Cedar](https://aws.amazon.com/blogs/security/why-policy-in-amazon-bedrock-agentcore-chose-cedar-for-securing-agentic-workflows/)):
  agent tool calls map to Cedar `principal/action/resource/context`; **LLM-generated tool
  arguments go into `context.input.*`** and trusted identity into `context` from JWT claims;
  policies gate on those attributes, "ensuring that even if the LLM produces unexpected
  arguments, the policy enforcement layer rejects them deterministically." Cedar was chosen for
  analyzability (formal verification of the policy set), readability ("structured natural
  language", auditable), and bounded deterministic eval.
- **Sondera, "Hooking Coding Agents with Cedar"** ([blog](https://blog.sondera.ai/p/hooking-coding-agents-with-the-cedar)):
  near-identical to this project (hook adapters at each agent's boundary, normalize over stdio,
  monitor outside the loop). Action set `ShellCommand`/`FileWrite`/`WebFetch`. Crucially, a
  **Guardrails Layer preprocesses the command and tags it with attributes** (Yara signatures for
  dangerous patterns) **before Cedar**; Cedar evaluates the *attributes*, not the raw string.
  Multi-turn ("read confidential then exfil") is served by an **Entity and Trajectory Store**
  that accumulates context across hook invocations and feeds Cedar's context.
- **OPAL** ([permitio/opal](https://github.com/permitio/opal)): the reference central-plane
  design. Server holds policies and publishes change events over a websocket pub/sub channel;
  each client pulls the update and **hot-reloads it into its local engine**; supports Cedar via
  `cedar-agent`. >100k changes/day in production.
- **arXiv, "Before the Tool Call"** ([pdf](https://arxiv.org/pdf/2603.20953)) and Microsoft's
  **Agent Governance Toolkit** corroborate: single point of control before execution,
  deny-by-default, central plane, single-action plus trajectory awareness.

## Decision
1. **Govern rich actions by host-derived attributes, never by parsing raw strings inside a
   policy.** The host (the PEP / a guardrails step) parses the messy input (a Bash command line,
   a tool-argument blob) and emits a **clean, typed attribute set**; the Cedar rule decides on
   those attributes. So a package-install gate is `forbid(principal, action ==
   Action::"ShellCommand", resource) when { context.command.kind == "package_install" }`, with
   the brittle command parsing living in Rust where it belongs, not as a regex in the rulebook.
   This is the canonical form of the "host computes facts, the rulebook decides on facts" line:
   **parsing/derivation is code; the decision is data.** It keeps policy declarative, centrally
   editable, and analyzable, and it matches AWS (args -> `context.input`) and Sondera (commands
   -> tagged attributes) directly.
2. **Untrusted, LLM-generated inputs enter only as `context` attributes**, never as the trusted
   principal/resource identity. This is the structural defense: a policy reads
   `context.command.*` / `context.input.*` knowing they are model output, while identity comes
   from the host. It also fixes the [ADR-0021](0021-pep-as-rust-command-hook-binary.md) seam:
   the request context is `CedarContext::empty()` today; the first non-mutation action is what
   populates it.
3. **Record the enforcement-point fork.** This project enforces at a **per-agent hook**
   ([ADR-0021](0021-pep-as-rust-command-hook-binary.md)); AWS AgentCore enforces at a **gateway**
   that proxies all tool calls (non-bypassable, but requires routing every tool through it).
   The hook is correct for v0 (works with existing agents, no proxy to run); the gateway is the
   non-bypassability upgrade path when tools route through MCP, and is the concrete shape behind
   [ADR-0003](0003-govern-at-framework-layer-defer-kernel.md)'s deferred "true enforcement".
4. **Confirm OPAL + `cedar-agent` as the central-plane distribution path.** The current
   per-call file read of `policy.cedar` is the correct single-machine degenerate case (an edit is
   live on the next call). OPAL's pub/sub-push + client-pull + hot-reload is the multi-machine
   evolution, which is exactly the already-deferred "policy distribution for multi-machine fleets"
   in TASKS.md — now with a named reference design, not an invention.

## Consequences
- **The `install_package` slice has a settled shape**: a Bash-command parser/tagger host-side, a
  small `ShellCommand` action + `context.command.*` attributes, and a declarative Cedar rule. The
  parser is the only new hard part, and its brittleness is contained to one host-side module,
  never leaking into policy text.
- **`context` becomes load-bearing.** Populating the Cedar request context (today empty) is the
  unlock for both non-mutation actions and, later, the trajectory lane. The discipline holds:
  trajectory enters as derived attributes too, fed by a store (the decision log is the candidate),
  matching Sondera's Entity and Trajectory Store.
- **Two near-term, validated additions become available** (not adopted here, recorded as
  directions): Cedar **policy analysis** in CI (detect contradictory/vacuous/over-permissive
  rules, Cedar's formal-verification superpower) and **neuro-symbolic authoring** (natural language
  -> Cedar draft -> analyzer verifies), which fits the "visualize and edit rules" goal.
- **Independent external validation** of the core architecture lowers the risk of the remaining
  design choices; the dominant risk shifts from "is the shape right" to "ship the concrete slice".

## Deliberately deferred (each with a revisit trigger)
- **The Bash-command parser/tagger itself.** This ADR fixes the *pattern*; the parser is the
  implementation slice. Trigger: the `ShellCommand` task (next).
- **Trajectory store + multi-turn rules.** The attribute pattern extends to trajectory, but the
  store and stateful clauses stay deferred. Trigger: the stateful lane (ASI03/06/08).
- **Gateway enforcement / MCP routing.** Trigger: a non-bypassable choke point is required (a real
  adversary, or tools already routed through MCP).
- **OPAL-based distribution.** Trigger: agents span machines, or must run while the plane is down.
- **Cedar policy analysis in CI and neuro-symbolic authoring.** Trigger: the policy set grows
  enough that manual review misses conflicts, or rule authoring by hand becomes the bottleneck.
