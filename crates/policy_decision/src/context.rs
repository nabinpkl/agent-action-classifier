//! Context (PIP): the per-decision facts outside the policy, chiefly scoped approvals.
//!
//! A scoped approval lifts an *implicit* deny (a `RequiresApproval` permit) only; it can
//! never override a Cedar `forbid` (org supremacy, enforced in `evaluate`: the approval
//! is consulted only on an Allow path). Approval lives host-side, not in the Cedar
//! policy, so a one-time consent cannot be replayed as blanket consent. The trajectory
//! window for the future stateful lane will also live here.

use crate::canonical_action::{CanonicalAction, ResourceId};

/// Who granted an approval.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(pub String);

/// How widely an approval reaches. v0 carries the two scopes the corpus exercises;
/// `SessionWindow` lands with the stateful lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalScope {
    /// This one action only.
    ThisAction,
    /// Any action on this data scope.
    ResourceClass(ResourceId),
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
    #[must_use]
    pub fn covers(&self, action: &CanonicalAction) -> bool {
        if action.at > self.expires {
            return false;
        }
        match &self.scope {
            ApprovalScope::ThisAction => true,
            ApprovalScope::ResourceClass(resource) => &action.resource == resource,
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
    /// resolve a `RequiresApproval`, never a `forbid`.
    #[must_use]
    pub fn has_approval_for(&self, action: &CanonicalAction) -> bool {
        self.approvals.iter().any(|a| a.covers(action))
    }
}
