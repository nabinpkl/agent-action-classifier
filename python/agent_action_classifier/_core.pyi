"""Type stub for the compiled Rust extension `_core` (built by maturin from
crates/policy_decision_py). Hand-written so the type checker (ty) can resolve the
binding; keep it in sync with crates/policy_decision_py/src/lib.rs."""

def decide_json(
    action_json: str,
    schema_cedar: str,
    policy_cedar: str,
    entities_json: str,
    context_json: str,
) -> str:
    """Decide a verdict from a JSON action, the Cedar org policy (schema source, policy
    source, entity JSON), and a JSON context, returning a JSON decision.

    Raises ValueError if any input fails to parse or validate against the schema."""
    ...
