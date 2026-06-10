//! Decision (PDP output): the verdict and the audit-bearing fields that explain it.
//!
//! Pure and deterministic: no `latency_ns` here. Timing is nondeterministic and would
//! break exact-match conformance, so it is measured by the caller (bench/host) and
//! attached at the decision-record layer, not produced by `decide`.

use crate::policy::{Lane, OwaspClause, PolicyId};

/// The four governance verdicts. `Flag` = observe-and-log (a soft, non-blocking
/// signal); reserved in v0, produced once the PostToolUse observe lane lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Deny,
    Escalate,
    Flag,
}

/// Hard = blocking gate; Soft = advisory/observe. The EU AI Act Art. 12 distinction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateType {
    Hard,
    Soft,
}

/// The PDP's verdict plus why. `owasp` and `policy_id` are both `None` for the
/// engine-default escalate, where no policy matched and so none can be cited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub verdict: Verdict,
    pub gate_type: GateType,
    pub owasp: Option<OwaspClause>,
    pub policy_id: Option<PolicyId>,
    pub lane: Lane,
    pub rationale: String,
}
