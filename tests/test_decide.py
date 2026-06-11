"""End-to-end test of the FFI boundary: compile a Cedar org policy once into a
`agent_action_classifier.Policy` handle, then decide actions against it (ADR-0019, the
parse-once lifecycle) and check the verdict. Also reports a rough per-decision latency, with
compilation excluded from the loop, so the number is the true hot-path cost. Stdlib
unittest, no extra deps."""

import time
import unittest

from agent_action_classifier import Policy

# Cedar schema (the contract / PAP): the policy and entities are validated against it at
# the edge (ADR-0018), so a typo'd attribute fails loud instead of silently not matching.
SCHEMA = """
entity Agent;
entity DataScope = { sensitivity: String, pii: Bool };
action read, write, share, delete, execute appliesTo {
    principal: [Agent],
    resource: [DataScope],
};
"""

# Cedar policy source: a hard deny on writing a secret-classified scope, with the audit
# annotations the host reads back (@id, @owasp).
POLICY = """
@id("deny-secret-write")
@owasp("ASI05")
forbid(principal, action == Action::"write", resource)
when { resource.sensitivity == "secret" };
"""

# Cedar entity JSON (the org model / PAP): data scopes with the schema's attributes.
ENTITIES = [
    {"uid": {"type": "Agent", "id": "agent-1"}, "attrs": {}, "parents": []},
    {
        "uid": {"type": "DataScope", "id": "secrets"},
        "attrs": {"sensitivity": "secret", "pii": False},
        "parents": [],
    },
    {
        "uid": {"type": "DataScope", "id": "telemetry"},
        "attrs": {"sensitivity": "internal", "pii": False},
        "parents": [],
    },
]


def write_action(resource: str) -> dict:
    return {
        "principal": "agent-1",
        "action": "write",
        "resource": resource,
        "session_id": "session-1",
        "seq": 0,
        "at": 1000,
        "source": {"provider": "langgraph", "raw_payload_id": "raw-0"},
    }


class DecideOverFfi(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Compile the org policy once (the parse-once handle); every test decides against it.
        cls.policy = Policy.compile(SCHEMA, POLICY, ENTITIES)

    def test_hard_deny_write_secret_scope(self):
        decision = self.policy.decide(write_action("secrets"), {"approvals": []})
        self.assertEqual(decision["verdict"], "deny")
        self.assertEqual(decision["gate_type"], "hard")
        self.assertEqual(decision["owasp"], "ASI05")
        self.assertEqual(decision["policy_id"], "deny-secret-write")

    def test_no_rule_defaults_to_escalate(self):
        decision = self.policy.decide(write_action("telemetry"), {"approvals": []})
        self.assertEqual(decision["verdict"], "escalate")
        self.assertIsNone(decision["owasp"])
        self.assertIsNone(decision["policy_id"])

    def test_bad_action_input_raises_value_error(self):
        # seq must be an integer; a string must surface as ValueError, not a silent pass.
        bad = write_action("secrets")
        bad["seq"] = "not-an-int"
        with self.assertRaises(ValueError):
            self.policy.decide(bad, {"approvals": []})

    def test_compile_rejects_schema_violating_policy(self):
        # `resource.classification` is not in the schema: compilation must fail loud at the
        # edge (ValueError), not defer a silent non-match to decide time.
        bad_policy = """
        @id("typo")
        permit(principal, action == Action::"read", resource)
        when { resource.classification == "public" };
        """
        with self.assertRaises(ValueError):
            Policy.compile(SCHEMA, bad_policy, ENTITIES)

    def test_report_per_decision_latency(self):
        action = write_action("secrets")
        ctx = {"approvals": []}
        iterations = 5000
        start = time.perf_counter()
        for _ in range(iterations):
            self.policy.decide(action, ctx)
        per_call_us = (time.perf_counter() - start) / iterations * 1_000_000
        # Compilation is excluded (paid once in setUpClass), so this is the true hot path.
        print(
            f"\n[ffi] decide hot path (marshal + eval, no policy parse): {per_call_us:.2f} us/call"
        )


if __name__ == "__main__":
    unittest.main()
