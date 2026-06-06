---
name: pydantic-models
description: Pydantic v2 conventions for the Python host, validation at boundaries, strict/frozen models, field vs model validators, settings. Use when defining the canonical-action mirror, config, judge request/response schemas, or any data crossing into the Python host.
---

# Pydantic v2 (Python host)

Pydantic v2 has a Rust validation core (5-50x faster than v1) and is the right tool
for the host's boundaries: validating actions, config, and judge I/O. It pairs
naturally with this project (its core is Rust, like the PDP). Follow
[AGENTS.md](../../../AGENTS.md) first: **fail loud**, no silent coercion or defaults.

Sources: [Pydantic docs](https://docs.pydantic.dev/latest/),
[Performance](https://docs.pydantic.dev/latest/concepts/performance/),
[Validators](https://pydantic.dev/docs/validation/latest/concepts/validators/).

## Rules

- **Config via `model_config = ConfigDict(...)`**, not the v1 inner `class Config`.
- **Strict mode for the governance domain.** Set `ConfigDict(strict=True)`: governance
  data must not be silently coerced (`"1"` is not `1`). Loud `ValidationError` over a
  quiet wrong type matches the fail-loud rule. Let `ValidationError` propagate; never
  catch-and-default it.
- **Freeze value objects.** The canonical action and policy records are immutable;
  `ConfigDict(frozen=True)` makes that a guarantee and makes them hashable.
- **`@field_validator` for one field, `@model_validator` for cross-field logic.** Use
  `mode="before"` to preprocess raw input, `"after"` (default) for validated values.
  **Avoid `mode="wrap"`** unless necessary, it is the slow path.
- **Parse with `Model.model_validate_json(bytes)`**, not `json.loads` then validate, it
  validates during parse and is faster, which matters for the conformance corpus.
- **`Field()` for constraints** (bounds, patterns), so invalid data fails at the edge,
  not deep in logic.
- **Config via `pydantic-settings`** (`BaseSettings`) for env/file configuration; do not
  scatter `os.environ` reads.

## This project

The Python host validates each `CanonicalAction` at the PEP boundary before it crosses
into the Rust PDP, and validates judge request/response schemas. Keep the Pydantic model
a **thin validating mirror** of the [SPEC.md](../../../SPEC.md) contract: the Rust PDP is
the source of truth for *semantics*; Pydantic guards the *shape* at the door. Do not
duplicate decision logic in the model's validators, validators check well-formedness,
not policy.
