//! Org policy (PAP): the authored rules the PDP evaluates.
//!
//! A [`Matcher`] only decides *applicability* (does this rule concern this operation);
//! the verdict comes from the rule's [`Outcome`] and [`Lane`] under the precedence in
//! `evaluate`. Keeping applicability separate from outcome is what lets the same
//! structured predicate route an action to a HardDeny or to the semantic judge.

use crate::canonical_action::Operation;

/// Stable rule id; appears in the decision record for audit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuleId(pub String);

/// OWASP Agentic clause, e.g. `ASI05`. The organizing/audit layer, not the logic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwaspClause(pub String);

/// Which evaluation lane resolves a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    /// Structured predicate, resolved in the pure core.
    Deterministic,
    /// Routed to the host's LLM judge (the core only returns `Escalate`).
    Semantic,
}

/// What an applicable rule wants to happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Explicit deny: supreme, unoverridable by any approval.
    HardDeny,
    /// Explicit allow.
    HardAllow,
    /// An implicit deny the org delegates to scoped user approval.
    RequiresApproval,
}

/// Applicability predicate over a single operation. v0 carries only the variants the
/// ASI05 corpus exercises; matching on `self` first keeps it exhaustive, so a new
/// variant fails the build instead of silently never matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matcher {
    /// ShellExec whose command contains any of these substrings.
    ShellCommandContainsAny(Vec<String>),
    /// FileWrite whose path starts with this prefix.
    FileWritePathPrefix(String),
}

impl Matcher {
    pub fn applies_to(&self, operation: &Operation) -> bool {
        match self {
            Matcher::ShellCommandContainsAny(needles) => matches!(
                operation,
                Operation::ShellExec { command, .. }
                    if needles.iter().any(|n| command.contains(n.as_str()))
            ),
            Matcher::FileWritePathPrefix(prefix) => matches!(
                operation,
                Operation::FileWrite { path, .. } if path.starts_with(prefix.as_str())
            ),
        }
    }
}

/// One authored rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub id: RuleId,
    pub owasp_tag: OwaspClause,
    pub lane: Lane,
    pub matcher: Matcher,
    pub outcome: Outcome,
}

/// The org policy: an ordered set of rules. Precedence is by authority (see
/// `evaluate`), not by list position.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Policy {
    pub rules: Vec<Rule>,
}
