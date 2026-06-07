# ADR-0010: Conformance corpus as external JSON, mapped at the edge

Date: 2026-06-06
Status: Accepted

## Context
The ASI05 conformance corpus began as Rust literals inside the test file. The corpus
is meant to be the **drift-proof executable spec** (and later the replay set for the
latency bench, and potentially a shape the host reuses), so it should be data, not
source. Externalizing it forces a parse/load boundary, which raised two coupled
questions: what format, and where the deserialization lives relative to the pure core.

The core (`policy_decision`) is deliberately pure and **zero runtime dependencies** so
it is testable with zero infrastructure and cheap to embed across languages
([ADR-0001](0001-build-as-a-learning-project.md), the dependency rule in AGENTS.md).
Deriving `serde` directly on the domain types would pull a serialization framework into
the pure modules, pointing a source dependency *outward* toward infrastructure.

## Decision
- The corpus is **external JSON** under `corpus/<clause>/`: one `policy.json` (the
  authored org policy) plus one `cases.json` (an array of `{name, action, context,
  expect}`). Cases author only the verdict-affecting fields; audit-only fields
  (`agent_id`, `seq`, `source`) are filled with fixed synthetic constants by the loader.
- The loader lives at the **edge**, as test/bench harness, **not** in the core crate.
  serde `Deserialize` **DTOs** mirror the JSON and map explicitly (`From`) into the
  domain types. The pure modules never import serde; the core crate carries serde only
  as a **dev-dependency**, so the shipped library stays dependency-free.
- Harness errors use **`anyhow`** (with `.context`), because nothing branches on a
  corpus-load failure, it only needs to read well. `thiserror` is reserved for the real
  typed boundary, the host loading an org policy, where callers *do* branch.
- The loader **fails loud**: a missing file, malformed JSON, an unknown variant tag, or
  a zero-case corpus all error rather than passing silently.

The rejected alternative, deriving serde on the domain types, is less code but couples
the pure core to serde and bakes a wire shape into the domain. The DTO + mapping
boilerplate is the accepted price of keeping the dependency direction correct.

## Consequences
- The core stays pure and zero-dep; conformance and (later) the bench share one loader.
- New action/matcher/scope variants require touching both the domain type and its DTO,
  a deliberate, compiler-enforced cost that matches the closed-set design.
- When the PyO3 binding lands and promotes the repo to a workspace
  ([source-organization skill]), the loader can move into its own corpus crate, fully
  removing serde from the core crate even as a dev-dependency.
- The host's eventual policy loader is a *separate* boundary; it will reuse the JSON
  shape but own a `thiserror` error type, not this harness code.
