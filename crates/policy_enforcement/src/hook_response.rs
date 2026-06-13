//! Map a PDP [`Decision`] to a provider PreToolUse hook response: an exit code plus optional
//! stdout/stderr (ADR-0007 verdict table, ADR-0021 realization). Kept as a data value
//! ([`HookResponse`]) so the mapping is unit-testable without spawning a process; `main` does
//! the single real I/O at the end.
//!
//! Block semantics are the cross-provider mechanism: **exit 2 + reason on stderr** is honored by
//! both Claude and Codex. `Escalate` asks via a `permissionDecision:"ask"` JSON on stdout, but
//! ONLY Claude speaks that schema — verified live that Codex *rejects* it ("unsupported
//! permissionDecision:ask") and fails the hook. So the escalate path is **provider-branched**:
//! Claude gets the ask dialog; Codex and an unknown provider degrade to an exit-2 block, so the
//! human still gates the action instead of seeing a hook error or a silent pass. `Allow`/`Flag`
//! proceed (exit 0). An internal enforcement error maps to a fail-closed deny — the
//! security-correct default, available because the binary owns its exit code.

use std::process::ExitCode;

use policy_decision::decision::{Decision, Verdict};

/// Which provider's hook protocol the response must speak. Only Claude honors the
/// `permissionDecision:"ask"` escalate dialog; Codex rejects it (verified live) and `Other` is an
/// unrecognized/misconfigured `--provider` we treat conservatively. A new provider is one arm here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Claude,
    Codex,
    Other,
}

impl Provider {
    #[must_use]
    pub fn parse(s: &str) -> Provider {
        match s {
            "claude" => Provider::Claude,
            "codex" => Provider::Codex,
            _ => Provider::Other,
        }
    }

    /// Does this provider honor a `permissionDecision:"ask"` PreToolUse response? Only Claude; an
    /// unknown provider is treated as not (degrade an escalate to a block, never a silent pass).
    #[must_use]
    fn supports_ask(self) -> bool {
        matches!(self, Provider::Claude)
    }
}

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

/// Map a decision to the hook response for `provider`. The escalate path is the only
/// provider-dependent one: Claude asks; everyone else blocks (they cannot ask via a hook).
#[must_use]
pub fn from_decision(decision: &Decision, provider: Provider) -> HookResponse {
    match decision.verdict {
        Verdict::Deny => blocked_deny(&deny_reason(decision)),
        Verdict::Escalate => {
            if provider.supports_ask() {
                HookResponse {
                    stdout: ask_json(&decision.rationale),
                    stderr: String::new(),
                    blocked: false,
                }
            } else {
                // Codex/unknown cannot ask a human via a hook, so degrade the escalate to a
                // clean exit-2 block: the action is gated (not errored, not silently allowed).
                blocked_deny(&approval_required_reason(decision))
            }
        }
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

/// The block reason for an escalate degraded to a deny (a provider with no ask dialog). Labeled
/// distinctly from a hard deny so the operator sees it was an approval gate, not a forbid.
fn approval_required_reason(decision: &Decision) -> String {
    let clause = decision.owasp.as_ref().map_or("-", |c| c.0.as_str());
    let id = decision.policy_id.as_ref().map_or("-", |p| p.0.as_str());
    format!(
        "APPROVAL REQUIRED [{clause}/{id}]: {} (no ask dialog on this provider: blocked pending human approval)",
        decision.rationale
    )
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
    fn deny_blocks_with_clause_and_id_on_stderr_on_any_provider() {
        // Deny is cross-provider (exit 2 + stderr), so the provider does not change it.
        for provider in [Provider::Claude, Provider::Codex, Provider::Other] {
            let r = from_decision(&decision(Verdict::Deny), provider);
            assert!(r.blocked);
            assert!(r.stderr.contains("ASI05"));
            assert!(r.stderr.contains("deny-secret-write"));
            assert!(r.stdout.is_empty());
        }
    }

    #[test]
    fn escalate_asks_on_claude_but_blocks_on_codex_and_unknown() {
        // Claude speaks the ask dialog.
        let claude = from_decision(&decision(Verdict::Escalate), Provider::Claude);
        assert!(!claude.blocked);
        assert!(claude.stdout.contains("\"ask\""));
        assert!(claude.stdout.contains("permissionDecision"));

        // Codex and an unknown provider cannot ask -> the escalate degrades to a clean block,
        // never a silent pass and never the ask JSON they reject.
        for provider in [Provider::Codex, Provider::Other] {
            let r = from_decision(&decision(Verdict::Escalate), provider);
            assert!(
                r.blocked,
                "{provider:?} must block an escalate it cannot ask"
            );
            assert!(r.stderr.contains("APPROVAL REQUIRED"));
            assert!(r.stdout.is_empty(), "no ask JSON for {provider:?}");
        }
    }

    #[test]
    fn allow_proceeds_silently_on_any_provider() {
        for provider in [Provider::Claude, Provider::Codex, Provider::Other] {
            assert_eq!(
                from_decision(&decision(Verdict::Allow), provider),
                proceed()
            );
        }
    }

    #[test]
    fn provider_parse_maps_known_names_else_other() {
        assert_eq!(Provider::parse("claude"), Provider::Claude);
        assert_eq!(Provider::parse("codex"), Provider::Codex);
        assert_eq!(Provider::parse("unknown"), Provider::Other);
    }

    #[test]
    fn internal_error_fails_closed() {
        let r = fail_closed("reading schema: not found");
        assert!(r.blocked);
        assert!(r.stderr.contains("failing closed"));
    }
}
