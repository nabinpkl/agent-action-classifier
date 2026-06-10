"""Python host for the agent-action governance classifier.

Holds the impure edges around the Cedar-backed PDP core: the enforcement adapter (PEP),
the context/approval source (PIP), the LLM judge, and the audit sink. Packaged by
concept as those land. For now it exposes one thing: `decide`, a Pythonic wrapper over
the compiled core (`_core`) that crosses the FFI boundary. See SPEC.md and ADR-0017.
"""

import json
from typing import Any

from ._core import decide_json as _decide_json

__all__ = ["decide"]


def decide(
    action: dict[str, Any],
    policy: str,
    entities: list[dict[str, Any]],
    context: dict[str, Any],
) -> dict[str, Any]:
    """Decide a verdict for one canonical action under an org policy and context.

    `action` and `context` are plain dicts shaped like the wire in SPEC.md. `policy` is
    Cedar policy source text and `entities` is Cedar's entity JSON (the org model / PAP);
    both are parsed by Cedar at the edge. The return is the decision dict (verdict,
    gate_type, owasp, policy_id, lane, rationale). The decision happens in Rust/Cedar;
    this only marshals across the boundary.
    """
    result = _decide_json(json.dumps(action), policy, json.dumps(entities), json.dumps(context))
    return json.loads(result)
