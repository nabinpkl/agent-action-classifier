# agent-action-classifier workflows. One named surface (AGENTS.md): don't hand-type
# the raw cargo/uv commands. `just check` is the full local gate and mirrors CI.

# list recipes
default:
    @just --list

# full local gate: build + test + lint + fmt for everything present
check: check-rust check-python

# Rust workspace (PDP core + binding): format check, lint, test.
# The binding crate is excluded from `cargo test` (a pyo3 cdylib needs libpython to link
# a test bin, and it has no Rust tests anyway); it is covered by the Python e2e test.
check-rust:
    cargo fmt --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo test --workspace --exclude policy_decision_py

# Python host: build the extension + sync, format check, lint, type-check, e2e test
check-python:
    uv sync
    uv run ruff format --check
    uv run ruff check
    uv run ty check
    uv run python -m unittest discover -s tests -p "test_*.py" -v

# auto-fix formatting across both languages
fmt:
    cargo fmt
    uv run ruff format
    uv run ruff check --fix

# stage-1 latency benchmark (reference-or-frontier); lands with benches/
bench:
    cargo bench --workspace

# repo-alignment deterministic graders (post-hoc adherence audit; see docs/adr/0013)
adherence:
    repo_alignment/eval/adherence_graders.sh

# bench the repo-alignment hook's bounded transcript tail-scan
hook-bench:
    repo_alignment/hooks/hook_bench.sh
