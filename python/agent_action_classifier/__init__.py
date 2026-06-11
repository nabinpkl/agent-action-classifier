"""Python host for the agent-action governance classifier.

Holds the impure edges around the Cedar-backed PDP core: the enforcement adapter (PEP),
the context/approval source (PIP), the LLM judge, and the audit sink. Packaged by
concept as those land. For now it exposes one thing: `decide`, a Pythonic wrapper over
the compiled core (`_core`) that crosses the FFI boundary. See SPEC.md and ADR-0017.
"""

import json
from typing import Any

from ._core import CompiledPolicy as _CompiledPolicy

__all__ = ["Policy"]


class Policy:
    """A compiled org policy: parse and validate the Cedar artifacts once, then decide many
    tool calls against the in-memory handle (ADR-0019, the parse-once lifecycle). The PEP
    compiles its effective policy on load (or on a plane push) and reuses it per tool call,
    so policy parsing never lands on the hot path. The decision happens in Rust/Cedar; this
    only marshals dicts across the boundary.
    """

    def __init__(self, handle: _CompiledPolicy) -> None:
        # Use Policy.compile(); the raw handle is built there.
        self._handle = handle

    @classmethod
    def compile(cls, schema: str, policy: str, entities: list[dict[str, Any]]) -> "Policy":
        """Compile the three Cedar artifacts authored by the central plane (PAP): `schema`
        (the contract source), `policy` (the rules source), and `entities` (Cedar's entity
        JSON). They are validated as a unit; drift raises ValueError.
        """
        return cls(_CompiledPolicy(schema, policy, json.dumps(entities)))

    def decide(self, action: dict[str, Any], context: dict[str, Any]) -> dict[str, Any]:
        """Decide a verdict for one canonical action under a context. `action` and `context`
        are plain dicts shaped like the wire in SPEC.md. The return is the decision dict
        (verdict, gate_type, owasp, policy_id, lane, rationale). A malformed action or
        context raises ValueError.
        """
        result = self._handle.decide(json.dumps(action), json.dumps(context))
        return json.loads(result)
