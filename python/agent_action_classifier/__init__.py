"""Python host for the agent-action governance classifier.

Holds the impure edges around the pure Rust PDP core: the enforcement adapter (PEP),
the context/approval source (PIP), the LLM judge, and the audit sink. Packaged by
concept as those land. For now it exposes one thing: `decide`, a Pythonic wrapper over
the compiled core (`_core`) that crosses the FFI boundary over a JSON wire. See SPEC.md.
"""

import json
from typing import Any

from ._core import decide_json as _decide_json

__all__ = ["decide"]


def decide(
    action: dict[str, Any],
    policy: dict[str, Any],
    context: dict[str, Any],
) -> dict[str, Any]:
    """Decide a verdict for one canonical action under a policy and context.

    `action`, `policy`, and `context` are plain dicts shaped like the JSON wire in
    SPEC.md; the return is the decision dict (verdict, gate_type, owasp, rule_id, lane,
    rationale). The pure decision happens in Rust; this only marshals across the edge.
    """
    result = _decide_json(json.dumps(action), json.dumps(policy), json.dumps(context))
    return json.loads(result)
