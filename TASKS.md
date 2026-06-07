# TASKS.md

Current iteration task list for `agent-action-classifier`.

## Done

- [x] Define the initial PRD scope. (PRD.md, Accepted 2026-06-06)
- [x] Choose and document the technical stack in SPEC.md.
- [x] Scaffold the Rust PDP core + Python host skeleton + `just check` gate.
- [x] First vertical slice: pure `decide` + ASI05 deterministic conformance corpus (9 cases).

## Next (priority order)

1. [x] **Externalize the conformance corpus to JSON.** Done: `corpus/asi05/*.json`
   replayed through `decide`; serde DTOs + anyhow loader at the edge (test-side),
   core stays serde-free. See ADR-0010. (thiserror deferred to the host PAP boundary.)
2. [ ] **Add the PyO3 binding crate** (promotes the crate to a workspace; keeps the
   core free of FFI types) so the Python host actually calls `decide`. Measure the
   FFI-crossing overhead success bar (ref: pydantic-core / polars).
3. [ ] **Wire the semantic lane:** host LLM judge for `Escalate` verdicts (exercises
   R4 end-to-end) plus its graded eval (agreement target 80-90%, never exact-match).

## Later

- [ ] Stage-1 latency benchmark (`benches/`, reference-or-frontier; ref Microsoft <0.1ms).
- [ ] Decision-log record (OPA/AAT-shaped JSON) at the host audit sink.
