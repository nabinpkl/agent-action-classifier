# agent-action-classifier workflows. One named surface (AGENTS.md): don't hand-type
# the raw cargo/uv commands. `just check` is the full local gate and mirrors CI.

# list recipes
default:
    @just --list

# full local gate: build + test + lint + fmt for everything present
check: check-rust check-python

# Rust PDP core: format check, lint (deny warnings), test (the conformance corpus)
check-rust:
    cargo fmt --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test

# Python host: sync deps, format check, lint, type-check (Astral toolchain)
check-python:
    uv sync
    uv run ruff format --check
    uv run ruff check
    uv run ty check

# auto-fix formatting across both languages
fmt:
    cargo fmt
    uv run ruff format
    uv run ruff check --fix

# stage-1 latency benchmark (reference-or-frontier); lands with benches/
bench:
    cargo bench
