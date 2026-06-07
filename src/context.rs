//! Context (PIP): the per-decision facts outside the policy, chiefly scoped approvals.
//!
//! A scoped approval lifts an *implicit* deny (`RequiresApproval`) only; it can never
//! override a `HardDeny` (org supremacy, enforced in `evaluate`). The trajectory window
//! for the future stateful lane will also live here.

use crate::canonical_action::{CanonicalAction, Operation};

/// Who granted an approval.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(pub String);

/// How widely an approval reaches. v0 carries the two scopes the corpus exercises;
/// `SessionWindow` lands with the stateful lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalScope {
    /// This one action only.
    ThisAction,
    /// Any ShellExec whose command contains this pattern.
    CommandClass(String),
}

/// A scoped, time-bounded human approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approval {
    pub scope: ApprovalScope,
    pub granted_by: UserId,
    pub expires: crate::canonical_action::Timestamp,
}

impl Approval {
    /// In-scope for the action *and* not expired as of the action's own timestamp.
    pub fn covers(&self, action: &CanonicalAction) -> bool {
        if action.at > self.expires {
            return false;
        }
        match &self.scope {
            ApprovalScope::ThisAction => true,
            ApprovalScope::CommandClass(pattern) => matches!(
                &action.operation,
                Operation::ShellExec { command, .. } if command.contains(pattern.as_str())
            ),
        }
    }
}

/// Everything the PDP knows beyond the action and the policy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Context {
    pub approvals: Vec<Approval>,
}

impl Context {
    /// Is there a valid, in-scope approval for this action? Only ever consulted to
    /// resolve a `RequiresApproval`, never a `HardDeny`.
    pub fn has_approval_for(&self, action: &CanonicalAction) -> bool {
        self.approvals.iter().any(|a| a.covers(action))
    }
}
