# ADR-0011: Workspace layout — a pure core crate and a separate PyO3 binding crate

Date: 2026-06-06
Status: Accepted

## Context
The host needs to call the Rust PDP from Python, which means a PyO3 boundary. SPEC.md
also requires the core to stay **polyglot-embeddable** (a C ABI / WASM / sidecar shape
is an explicit later option), so the core must not be welded to Python. The question is
how to lay out Rust and Python in one repo so the FFI lives at a single, contained edge.

Survey of the mature Rust-core/Python projects (June 2026):

- **HF tokenizers:** pure core crate `tokenizers/` + binding crate
  `bindings/python/` (`tokenizers-python`, `crate-type=["cdylib"]`, depends on the core
  via `path=../../tokenizers`, `pyo3` pinned). Python source + `.pyi` under the binding.
- **polars:** workspace of core `polars*` crates + a separate binding crate **`py-polars`**
  (deliberately named apart from core `polars`), path-dep into the workspace; Python
  package in `py-polars/src/polars/`; built with maturin + `python-source`.
- **pydantic-core:** a *single* crate that **is** the binding — because its only consumer
  is Python; there is no separate "core to embed elsewhere."

The split correlates with intent: split when the core is meant to outlive the binding
(tokenizers, polars); single-crate when it is not (pydantic-core). Our SPEC puts us in
the first camp. maturin officially supports this via `python-source` and a private
compiled submodule (`module-name = "pkg._core"`), the `_pydantic_core` / `polars.polars`
pattern; the one rough edge ([maturin#1372]) is that it cannot auto-locate a binding
crate in a workspace, so the build is pointed at it explicitly (`manifest-path`).

## Decision
Adopt a cargo **workspace** with a pure core crate and a separate binding crate, plus a
maturin `python-source` package:

```
crates/policy_decision/      # pure PDP core: zero FFI, zero runtime deps
crates/policy_decision_py/   # the PyO3 binding (cdylib); the ONLY place pyo3 exists
python/agent_action_classifier/   # the host package; wraps the private compiled module
corpus/                      # shared executable spec at the workspace root
pyproject.toml               # [tool.maturin] python-source + manifest-path + module-name
```

- The core crate **never depends on pyo3**; the binding crate depends on the core by
  `path`. This is the dependency rule (AGENTS.md) made physical: infrastructure (FFI) at
  the edge, pure logic at the center.
- The binding crate is named distinctly from the core (`policy_decision_py` vs
  `policy_decision`), per the polars `py-polars` convention, so the two are never
  confused.
- The compiled module is a **private submodule** (`agent_action_classifier._core`); the
  public Python package re-exports from it, so the user-facing API is Python, not the raw
  extension.
- The corpus stays at the **workspace root** as the shared spec (loader walks up to it),
  not buried in the core crate.

This is staged: **this ADR + commit introduce the workspace and move the core only** (no
pyo3 yet); the binding crate, maturin wiring, and FFI measurement land in the next slice.

## Consequences
- The single-crate v0 (and the `src/` collision where Rust `lib.rs` and the Python
  package shared one directory) is replaced; the source-organization skill named exactly
  this moment ("introduce the workspace when the binding crate is created").
- `just check` runs the Rust recipes with `--workspace`.
- maturin will be pointed at the binding crate via `manifest-path` ([maturin#1372]); the
  binding pins its `pyo3` version, as the surveyed projects do.
- When a third JSON consumer appears (the binding's wire mapping after the test harness),
  the serde DTO mapping becomes a candidate to extract into its own crate; deferred until
  then (Rule of Three).

[maturin#1372]: https://github.com/PyO3/maturin/issues/1372
