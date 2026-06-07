//! The PDP precedence engine: pure, deterministic, no I/O.
//!
//! Implements the authority order from `SPEC.md` / `docs/adr/0005`: deny-overrides
//! (explicit deny is supreme), org supremacy (approval lifts only an implicit deny),
//! a HardAllow shadowed when a semantic clause also applies, and a default
//! fail-closed via escalate when nothing resolves the action.

use crate::canonical_action::CanonicalAction;
use crate::context::Context;
use crate::decision::{Decision, GateType, Verdict};
use crate::policy::{Lane, Outcome, Policy, Rule};

/// Decide a verdict for `action` under `policy` and `context`. Pure and total: the
/// escalate lane is returned, never run here.
#[must_use]
pub fn decide(action: &CanonicalAction, policy: &Policy, context: &Context) -> Decision {
    let applicable: Vec<&Rule> = policy
        .rules
        .iter()
        .filter(|rule| rule.matcher.applies_to(&action.operation))
        .collect();

    // 1. Explicit deny is supreme (deny-overrides). No approval can lift it.
    if let Some(rule) = applicable.iter().copied().find(is_hard_deny) {
        return hard(
            Verdict::Deny,
            rule,
            "explicit org HardDeny matched: supreme, unoverridable",
        );
    }

    // 2. RequiresApproval is an implicit deny delegated to scoped approval.
    if let Some(rule) = applicable.iter().copied().find(is_requires_approval) {
        return if context.has_approval_for(action) {
            soft(
                Verdict::Allow,
                rule,
                "RequiresApproval satisfied by a valid in-scope approval",
            )
        } else {
            soft(
                Verdict::Escalate,
                rule,
                "RequiresApproval with no in-scope approval: escalate to human",
            )
        };
    }

    // 3. Explicit allow, but only if no higher (semantic) lane also applies.
    let semantic_applies = applicable.iter().any(|rule| rule.lane == Lane::Semantic);
    if !semantic_applies && let Some(rule) = applicable.iter().copied().find(is_hard_allow) {
        return hard(
            Verdict::Allow,
            rule,
            "explicit org HardAllow matched, no semantic clause applies",
        );
    }

    // 4. A semantic clause applies (or shadowed an allow): escalate to the judge.
    if let Some(rule) = applicable
        .iter()
        .copied()
        .find(|rule| rule.lane == Lane::Semantic)
    {
        return from_rule(
            Verdict::Escalate,
            GateType::Soft,
            rule,
            "semantic clause applies: escalate to the judge (the host runs it)",
        );
    }

    // 5. Nothing resolved it: default escalate; the host fails closed to Deny if the
    //    escalation does not resolve. No clause matched, so none is cited.
    Decision {
        verdict: Verdict::Escalate,
        gate_type: GateType::Soft,
        owasp: None,
        rule_id: None,
        lane: Lane::Deterministic,
        rationale: "no applicable rule: default escalate, fail closed to Deny if unresolved"
            .to_string(),
    }
}

fn is_hard_deny(rule: &&Rule) -> bool {
    rule.outcome == Outcome::HardDeny
}

fn is_requires_approval(rule: &&Rule) -> bool {
    rule.outcome == Outcome::RequiresApproval
}

fn is_hard_allow(rule: &&Rule) -> bool {
    rule.outcome == Outcome::HardAllow
}

fn hard(verdict: Verdict, rule: &Rule, why: &str) -> Decision {
    from_rule(verdict, GateType::Hard, rule, why)
}

fn soft(verdict: Verdict, rule: &Rule, why: &str) -> Decision {
    from_rule(verdict, GateType::Soft, rule, why)
}

fn from_rule(verdict: Verdict, gate_type: GateType, rule: &Rule, why: &str) -> Decision {
    Decision {
        verdict,
        gate_type,
        owasp: Some(rule.owasp_tag.clone()),
        rule_id: Some(rule.id.clone()),
        lane: rule.lane,
        rationale: why.to_string(),
    }
}
