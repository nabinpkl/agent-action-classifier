"""Type stub for the compiled Rust extension `_core` (built by maturin from
crates/policy_decision_py). Hand-written so the type checker (ty) can resolve the
binding; keep it in sync with crates/policy_decision_py/src/lib.rs."""

class CompiledPolicy:
    """A compiled org policy handle: the Cedar artifacts parsed and validated once, reused
    for many decisions (ADR-0019)."""

    def __init__(self, schema_cedar: str, policy_cedar: str, entities_json: str) -> None:
        """Compile the schema source, policy source, and Cedar entity JSON into the handle.

        Raises ValueError if any input fails to parse or validate against the schema."""
        ...

    def decide(self, action_json: str, context_json: str) -> str:
        """Decide a verdict from a JSON action and JSON context, returning a JSON decision.

        Raises ValueError if the action or context fails to parse."""
        ...
