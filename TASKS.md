# TASKS.md

Current iteration task list for `agent-action-classifier`.

## Tasks

- [x] Define the initial PRD scope. (PRD.md, Accepted 2026-06-06)
- [x] Choose and document the technical stack in SPEC.md.
- [x] Scaffold the Rust PDP core + Python host skeleton + `just check` gate.
- [x] First vertical slice: pure `decide` + ASI05 deterministic conformance corpus (9 cases).
- [ ] Externalize the corpus to JSON (brings the first fallible boundary: serde + thiserror).
- [ ] Add the PyO3 binding crate (workspace) so the host calls `decide`; measure FFI overhead.
- [ ] Add the stage-1 latency benchmark (`benches/`, reference-or-frontier).
- [ ] Wire the semantic lane: host LLM judge for `Escalate`, graded eval.
