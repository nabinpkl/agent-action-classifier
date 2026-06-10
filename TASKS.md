# TASKS.md

Current iteration task list for `agent-action-classifier`.

## Done

- [x] Define the initial PRD scope. (PRD.md, Accepted 2026-06-06)
- [x] Choose and document the technical stack in SPEC.md.
- [x] Scaffold the Rust PDP core + Python host skeleton + `just check` gate.
- [x] First vertical slice: pure `decide` + ASI05 deterministic conformance corpus (9 cases).

## Re-founding (ADR-0017: Cedar engine + org-first central plane)

The hand-built engine and the placeholder `Operation` enum are superseded. The corpus
externalization (ADR-0010) and the PyO3 host/binding split (ADR-0011) survive as the host
boundary; the matcher/precedence core is replaced by Cedar. Priority order:

1. [ ] **Cedar speed spike (the gate).** Embed `cedar-policy`; bench one mapped request
   against the stage-1 budget (p99 < 100µs; ref Microsoft <0.1ms, ADR-0006). Result decides
   whether the hand-rolled core is removed. *Next up; see the plan.*
2. [ ] **Swap the core to Cedar.** Replace `policy.rs` (`Matcher`) and `evaluate.rs`
   (precedence) with Cedar; re-point the conformance corpus at the Cedar integration.
3. [ ] **Org graph + inheritance resolution.** Cedar entities for org/team/role/user/agent;
   resolve an agent's effective policy by its position in the graph; policy-on-node cascade.
4. [ ] **Live per-agent hook PEP.** Wire each agent's PreToolUse hook to the plane (the
   `repo_alignment` exit-2-to-deny hook is the primitive).
5. [ ] **Decision-log record** (OPA/AAT-shaped JSON) at the host audit sink.
6. [ ] **Semantic judge lane:** host LLM judge for `Escalate`, graded eval (80-90%, never
   exact-match).

## Later (deferred, with triggers in PRD/SPEC)

- [ ] Multi-tenant orgs (trigger: plane hosts >1 org).
- [ ] Policy distribution for multi-machine fleets (OPAL-style).
- [ ] Stateful/trajectory lane (ASI03/06/08).
- [ ] Hash-chained tamper-evident audit log.
