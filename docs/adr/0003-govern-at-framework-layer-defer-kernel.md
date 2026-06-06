# ADR-0003: Govern at the framework layer now; defer kernel-level to later

Date: 2026-06-06
Status: Accepted

## Context
Kernel-level governance (seccomp-bpf, eBPF, LSM hooks, ptrace, microVM sandboxing)
is the only *unbypassable* enforcement surface: the kernel sees every syscall
regardless of which framework produced it, so a hijacked or evasive agent cannot
route around it. That is a real point about production security. But the kernel
operates at the wrong semantic level for this project: it sees `openat(...)` and
`connect(...)`, not "the agent called the send_email tool with these args."
Framework hooks (LangChain/LangGraph callbacks, coding-agent hooks) hand over agent
*intent* directly; the kernel forces reconstructing intent across a wide semantic
gap, and the plumbing is platform-specific and heavy.

## Decision
Build governance at the **framework layer** for now, where intent is legible and the
rule-engine and policy lessons are learnable. Treat **kernel-level enforcement as a
deliberate deferral**, a later defense-in-depth lesson ("run", not "crawl"), to be
descended into only after the framework layer is well understood.

Hold one honesty constraint: framework-level governance is advisory and bypassable,
necessary but not sufficient for real security. Do not later oversell what the
framework-layer v0 would provide in production.

## Consequences
- The kernel argument is settled as a sequencing choice, not an open question.
- Effort goes to governance logic, not OS plumbing.
- A future ADR may re-open the kernel layer as an explicit defense-in-depth track.
