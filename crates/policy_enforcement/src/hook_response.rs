//! Map a PDP [`Decision`] to a provider PreToolUse hook response: an exit code plus optional
//! stdout/stderr (ADR-0007 verdict table, ADR-0021 realization). Kept as a data value
//! ([`HookResponse`]) so the mapping is unit-testable without spawning a process; `main` does
//! the single real I/O at the end.
//!
//! Block semantics are the cross-provider mechanism verified this session: **exit 2 + reason
//! on stderr** is honored by both Claude and Codex. `Escalate` asks via a `permissionDecision`
//! JSON on stdout (Claude's native dialog). `Allow`/`Flag` proceed (exit 0). An internal
//! enforcement error maps to a fail-closed deny — the security-correct default, available
//! because the binary owns its exit code (Claude `type:http` could not).

use std::process::ExitCode;

use policy_decision::decision::{Decision, Verdict};

/// What to print and whether to block. `blocked = true` => exit 2 (deny); else exit 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookResponse {
    pub stdout: String,
    pub stderr: String,
    pub blocked: bool,
}

impl HookResponse {
    #[must_use]
    pub fn exit_code(&self) -> ExitCode {
        if self.blocked {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        }
    }

    /// Print to the real streams and return the exit code. The one I/O point.
    #[must_use]
    pub fn emit(&self) -> ExitCode {
        if !self.stdout.is_empty() {
            println!("{}", self.stdout);
        }
        if !self.stderr.is_empty() {
            eprintln!("{}", self.stderr);
        }
        self.exit_code()
    }
}

/// Map a decision to the hook response.
#[must_use]
pub fn from_decision(decision: &Decision) -> HookResponse {
    match decision.verdict {
        Verdict::Deny => blocked_deny(&deny_reason(decision)),
        Verdict::Escalate => HookResponse {
            stdout: ask_json(&decision.rationale),
            stderr: String::new(),
            blocked: false,
        },
        // Allow is a terminal pass; Flag (observe) is non-blocking and not produced in v0.
        Verdict::Allow | Verdict::Flag => proceed(),
    }
}

/// Proceed: the call is allowed or ungoverned, so let it run with no output.
#[must_use]
pub fn proceed() -> HookResponse {
    HookResponse {
        stdout: String::new(),
        stderr: String::new(),
        blocked: false,
    }
}

/// A fail-closed deny for an internal enforcement error (bad plane/payload/config): block with
/// a loud reason so the operator sees and fixes it, rather than silently failing open.
#[must_use]
pub fn fail_closed(reason: &str) -> HookResponse {
    blocked_deny(&format!("enforcement error (failing closed): {reason}"))
}

fn blocked_deny(reason: &str) -> HookResponse {
    HookResponse {
        stdout: String::new(),
        stderr: reason.to_string(),
        blocked: true,
    }
}

/// The audit-bearing deny reason: the OWASP clause, the policy id, and the rationale.
fn deny_reason(decision: &Decision) -> String {
    let clause = decision.owasp.as_ref().map_or("-", |c| c.0.as_str());
    let id = decision.policy_id.as_ref().map_or("-", |p| p.0.as_str());
    format!("DENY [{clause}/{id}]: {}", decision.rationale)
}

fn ask_json(rationale: &str) -> String {
    serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "ask",
            "permissionDecisionReason": rationale,
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use policy_decision::decision::{GateType, Verdict};
    use policy_decision::policy::{Lane, OwaspClause, PolicyId};

    fn decision(verdict: Verdict) -> Decision {
        Decision {
            verdict,
            gate_type: GateType::Hard,
            owasp: Some(OwaspClause("ASI05".into())),
            policy_id: Some(PolicyId("deny-secret-write".into())),
            lane: Lane::Deterministic,
            rationale: "explicit org forbid matched".into(),
        }
    }

    #[test]
    fn deny_blocks_with_clause_and_id_on_stderr() {
        let r = from_decision(&decision(Verdict::Deny));
        assert!(r.blocked);
        assert!(r.stderr.contains("ASI05"));
        assert!(r.stderr.contains("deny-secret-write"));
        assert!(r.stdout.is_empty());
    }

    #[test]
    fn escalate_asks_via_stdout_json_without_blocking() {
        let r = from_decision(&decision(Verdict::Escalate));
        assert!(!r.blocked);
        assert!(r.stdout.contains("\"ask\""));
        assert!(r.stdout.contains("permissionDecision"));
    }

    #[test]
    fn allow_proceeds_silently() {
        let r = from_decision(&decision(Verdict::Allow));
        assert_eq!(r, proceed());
    }

    #[test]
    fn internal_error_fails_closed() {
        let r = fail_closed("reading schema: not found");
        assert!(r.blocked);
        assert!(r.stderr.contains("failing closed"));
    }
}
