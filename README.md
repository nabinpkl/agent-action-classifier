# agent-action-classifier

A policy-driven **governance classifier for AI agent actions**. Given an agent action
and an organization policy, it decides `allow / deny / escalate / flag`, records which
[OWASP Agentic](https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/)
clause fired and why, and emits an audit-defensible decision log.


## What it is

Built as the standard authorization architecture (XACML's P*Ps): a pure,
environment-independent **Policy Decision Point** in Rust, driven from a Python host
that holds the impure edges (enforcement adapter, context/approval source, audit sink).
The decision point runs a **layered cascade**: a cheap deterministic rule engine settles
the clear cases at sub-millisecond latency, and an LLM **judge** is consulted only for
the ambiguous, semantic ones. **Organization policy is supreme**: an explicit org deny
can never be overridden by a local user approval.

It is a [learning project](docs/adr/0001-build-as-a-learning-project.md): the goal is
to understand agent-action governance, rule-engine internals, and the audit trail by
building the load-bearing parts by hand and **measuring** them, not to ship a product.

## Status

v0 is scoped and accepted; implementation has not started. v0 proves one vertical slice
end to end, the OWASP **ASI05 (Unsafe Code Execution)** clause over shell and file-write
actions, and measures it.

## Documentation

- [PRD.md](PRD.md) — what we build and why (product intent).
- [SPEC.md](SPEC.md) — technical shape (in progress).
- [docs/adr/](docs/adr/) — architecture decisions, one per file.
- [AGENTS.md](AGENTS.md) — how to operate in this repo.

## License

No license yet. All rights reserved while the project is experimental.
