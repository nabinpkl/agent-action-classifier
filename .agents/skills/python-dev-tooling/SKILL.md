---
name: python-dev-tooling
description: The 2026 Astral Python toolchain (uv, ruff, ty) plus maturin for the PyO3 wheel, all configured in pyproject.toml and wrapped by just. Use when setting up the Python host, adding deps, linting/formatting, type-checking, or building the Rust extension.
---

# Python dev tooling (2026: the Astral stack)

The host uses Astral's Rust-written toolchain (10x-1000x faster than the legacy tools),
configured in one `pyproject.toml` and wrapped by `just`. Astral (Ruff/uv/ty) joined
OpenAI's Codex team in 2026, so deeper agent integration is expected; the tools
themselves remain the standard.

Sources: [Astral](https://astral.sh/), [uv](https://docs.astral.sh/uv/),
[ruff](https://github.com/astral-sh/ruff), [ty](https://github.com/astral-sh/ty).

## The tools

- **uv** = package manager + venv + resolver (replaces pip / pip-tools / virtualenv).
  `uv add <pkg>`, `uv sync`, `uv run <cmd>`, `uv lock`. Commit `uv.lock` for reproducible
  resolutions. uv manages the project venv; do not hand-manage `pip`.
- **ruff** = linter + formatter in one (replaces black, isort, flake8 + plugins,
  pyupgrade, autoflake). `ruff format` and `ruff check --fix`. Configure under
  `[tool.ruff]`.
- **ty** = type checker + language server (replaces mypy/pyright, 10-100x faster).
  `ty check`. Configure under `[tool.ty]` in `pyproject.toml`. Type errors fail CI.
- **maturin** = builds the PyO3 Rust extension into the Python wheel. `maturin develop`
  for local iteration, `maturin build` for the wheel. This is how the Rust PDP becomes
  importable from the host.

## Workflow

Wrap everything in `just` (AGENTS.md: one named surface, don't hand-type raw commands):

- `just check` = `uv sync` + `ruff format --check` + `ruff check` + `ty check` +
  `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test`.
- `just dev` / `just bench` for run and benchmark.

Pin tool versions and run them in CI with errors fatal (the Python analog of
`clippy -D warnings`). Build, lint, type-check, and format locally before committing.
