---
name: source-code-organization
description: How to organize Python and Rust source in this repo, language mechanics layered under the repo's package-by-concept philosophy. Use when creating files, modules, crates, or packages; deciding where code or tests live; or structuring the Rust PDP core or the Python host.
---

# Source code organization (Python + Rust)

This is the **language-mechanics layer** that sits *under* the repo's organization
philosophy. The philosophy in [AGENTS.md](../../../AGENTS.md) wins on every conflict:
package by **concept** with **literal, greppable names**; vertical slices; no
`utils`/`helpers`/`core`/`manager`/`service` dumping grounds; the dependency rule
(pure logic inward, infrastructure at the edges); `~500` lines is a smell, split at a
natural seam. This skill only says *how to realize that idiomatically* in Cargo and in
Python packaging.

## The one override to remember

Generic Python/Rust guides tell you to package by **layer** (`core/`, `api/`,
`models/`, `services/`, `utils/`). **Do not.** That contradicts AGENTS.md. Name folders
and modules after the literal concept (`canonical_action`, `policy`, `decision`,
`judge`, `audit_log`), never after a framework role. The dependency direction lives in
the imports, not in folder names.

## Rust

Source: [Cargo Package Layout](https://doc.rust-lang.org/cargo/guide/project-layout.html),
[The Rust Book ch.7/14](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html).

- **Layout:** `src/lib.rs` for a library, `src/main.rs` for a binary. Integration tests
  in `tests/`, benchmarks in `benches/`, extra binaries in `src/bin/`, examples in
  `examples/`.
- **Modules = concepts.** Start flat (`policy.rs`, `decision.rs`). When a concept grows
  its own internals, promote it to a directory. **Use the modern form:** a sibling
  `policy.rs` *plus* a `policy/` directory for its children. **Avoid the legacy
  `policy/mod.rs`** style; `mod.rs` files make every module look the same to grep.
- **Tests:** unit tests colocated in the same file under `#[cfg(test)] mod tests`;
  cross-module/behavioral tests in top-level `tests/`. The conformance corpus is the
  primary test target and lives where it can run against `decide` as a black box.
- **Workspaces** (`[workspace]`, one shared `Cargo.lock`) only when a second crate
  actually exists. Don't pre-split. The natural seam here: the pure **PDP core** crate
  vs. the **PyO3 binding** crate, so the core stays free of FFI types and is testable
  with zero Python. Introduce the workspace when the binding crate is created, not before.
- **`pub` is the contract.** Keep the public surface small and literal; internals stay
  private. The deep module (the PDP) exposes one `decide`, hides the engine.
- Start simple, scale when needed; a single `lib.rs` with a few concept modules is the
  right v0, not a workspace.

## Python

Source: [Real Python project layout](https://realpython.com/ref/best-practices/project-layout/),
[Hitchhiker's Guide](https://docs.python-guide.org/writing/structure/).

- **Use the `src/` layout** (`src/<package>/...`). It's for installable/importable
  packages, and the host *is* a package (it ships a PyO3 wheel). The src-layout prevents
  accidental imports from the working dir and keeps source separate from config.
- **Root holds** `pyproject.toml`, `README.md`, configs, CI. One `pyproject.toml`.
- **Package by concept, not layer.** Sub-packages/modules named for what they are
  (`pep_adapter`, `corpus`, `judge`, `audit_log`, `context`), mirroring the same
  concepts as the Rust side where they pair. Ignore the common `core/api/models/utils`
  template, it violates AGENTS.md.
- **Tests** in a top-level `tests/` that loosely mirrors the package, so tests are easy
  to run and easy to exclude from the distribution. Keep behavioral tests (assert
  external behavior) separate from any unit tests.
- **`__init__.py` empty is fine and preferred** unless the package genuinely needs to
  share code; don't stuff re-exports in to look tidy.
- Split a module when it passes the repo's ~500-line smell *or* takes a second
  responsibility, whichever comes first.

## This project, concretely

```
agent-action-classifier/
├── pyproject.toml            # host package (src-layout) + maturin build
├── Cargo.toml                # PDP core crate (workspace later, when the binding crate lands)
├── src/                      # Rust PDP core: lib.rs + concept modules
│   ├── lib.rs                #   exposes `decide`
│   ├── canonical_action.rs
│   ├── policy.rs
│   ├── decision.rs
│   └── ...                   #   the layered evaluator, by concept
├── tests/                    # Rust behavioral/conformance tests against `decide`
├── benches/                  # stage-1 latency benchmark (reference-or-frontier)
├── src/<host_package>/       # Python host (src-layout), packaged by concept:
│   ├── pep_adapter/          #   provider event -> canonical action (synthetic corpus in v0)
│   ├── judge/                #   semantic lane (LLM)
│   ├── audit_log/            #   OPA/AAT-shaped decision records
│   └── context/              #   scoped approvals (PIP)
└── tests/                    # Python behavioral tests + judge graded eval
```

Names are illustrative; pick the literal concept when the code lands. The contracts
these realize are in [SPEC.md](../../../SPEC.md).
