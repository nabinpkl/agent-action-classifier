//! The JSON wire between the Python host and the pure PDP: parse a full canonical
//! action + policy + context in, serialize a decision out. serde lives here at the
//! edge, never in the core (ADR-0010/0011); the binding owns this mapping because its
//! wire carries the *full* canonical action (real agent/session/provenance), unlike the
//! corpus loader's terse authoring form, so the two serializations differ by intent.
//!
//! This is the JSON PDP wire ADR-0007 describes (the shape a PEP HTTP-hook would send).
//! Errors use anyhow: the binding surfaces them to Python as ValueError, nobody branches
//! on variants.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use policy_decision::canonical_action::{
    AgentId, CanonicalAction, Operation, Provenance, SessionId, Timestamp,
};
use policy_decision::context::{Approval, ApprovalScope, Context, UserId};
use policy_decision::decide;
use policy_decision::decision::{Decision, GateType, Verdict};
use policy_decision::policy::{Lane, Matcher, Outcome, OwaspClause, Policy, Rule, RuleId};

/// Parse the three JSON inputs, run the pure `decide`, and serialize the decision.
pub fn decide_json(action_json: &str, policy_json: &str, context_json: &str) -> Result<String> {
    let action: CanonicalAction = serde_json::from_str::<ActionDto>(action_json)?.into();
    let policy: Policy = serde_json::from_str::<PolicyDto>(policy_json)?.into();
    let context: Context = serde_json::from_str::<ContextDto>(context_json)?.into();

    let decision = decide(&action, &policy, &context);
    Ok(serde_json::to_string(&DecisionDto::from(&decision))?)
}

// --- input DTOs: the full canonical action + policy + context --------------------

#[derive(Deserialize)]
struct ActionDto {
    agent_id: String,
    session_id: String,
    seq: u64,
    at: i64,
    source: ProvenanceDto,
    operation: OperationDto,
}

#[derive(Deserialize)]
struct ProvenanceDto {
    provider: String,
    raw_payload_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum OperationDto {
    ShellExec { command: String, cwd: String },
    FileWrite { path: String, byte_len: u64 },
    NetworkFetch { url: String },
}

#[derive(Deserialize)]
struct PolicyDto {
    rules: Vec<RuleDto>,
}

#[derive(Deserialize)]
struct RuleDto {
    id: String,
    owasp: String,
    lane: LaneDto,
    #[serde(rename = "match")]
    matcher: MatcherDto,
    outcome: OutcomeDto,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum OutcomeDto {
    HardDeny,
    HardAllow,
    RequiresApproval,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum MatcherDto {
    ShellCommandContainsAny(Vec<String>),
    FileWritePathPrefix(String),
}

#[derive(Deserialize, Default)]
struct ContextDto {
    #[serde(default)]
    approvals: Vec<ApprovalDto>,
}

#[derive(Deserialize)]
struct ApprovalDto {
    scope: ApprovalScopeDto,
    granted_by: String,
    expires: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ApprovalScopeDto {
    ThisAction,
    CommandClass(String),
}

// --- shared leaf enums (both directions) -----------------------------------------

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LaneDto {
    Deterministic,
    Semantic,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum VerdictDto {
    Allow,
    Deny,
    Escalate,
    Flag,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum GateTypeDto {
    Hard,
    Soft,
}

// --- output DTO ------------------------------------------------------------------

#[derive(Serialize)]
struct DecisionDto {
    verdict: VerdictDto,
    gate_type: GateTypeDto,
    owasp: Option<String>,
    rule_id: Option<String>,
    lane: LaneDto,
    rationale: String,
}

// --- mappings (the edge) ---------------------------------------------------------

impl From<ActionDto> for CanonicalAction {
    fn from(dto: ActionDto) -> Self {
        CanonicalAction {
            agent_id: AgentId(dto.agent_id),
            session_id: SessionId(dto.session_id),
            seq: dto.seq,
            at: Timestamp(dto.at),
            source: Provenance {
                provider: dto.source.provider,
                raw_payload_id: dto.source.raw_payload_id,
            },
            operation: dto.operation.into(),
        }
    }
}

impl From<OperationDto> for Operation {
    fn from(dto: OperationDto) -> Self {
        match dto {
            OperationDto::ShellExec { command, cwd } => Operation::ShellExec { command, cwd },
            OperationDto::FileWrite { path, byte_len } => Operation::FileWrite { path, byte_len },
            OperationDto::NetworkFetch { url } => Operation::NetworkFetch { url },
        }
    }
}

impl From<PolicyDto> for Policy {
    fn from(dto: PolicyDto) -> Self {
        Policy {
            rules: dto.rules.into_iter().map(Rule::from).collect(),
        }
    }
}

impl From<RuleDto> for Rule {
    fn from(dto: RuleDto) -> Self {
        Rule {
            id: RuleId(dto.id),
            owasp_tag: OwaspClause(dto.owasp),
            lane: dto.lane.into(),
            matcher: dto.matcher.into(),
            outcome: dto.outcome.into(),
        }
    }
}

impl From<LaneDto> for Lane {
    fn from(dto: LaneDto) -> Self {
        match dto {
            LaneDto::Deterministic => Lane::Deterministic,
            LaneDto::Semantic => Lane::Semantic,
        }
    }
}

impl From<OutcomeDto> for Outcome {
    fn from(dto: OutcomeDto) -> Self {
        match dto {
            OutcomeDto::HardDeny => Outcome::HardDeny,
            OutcomeDto::HardAllow => Outcome::HardAllow,
            OutcomeDto::RequiresApproval => Outcome::RequiresApproval,
        }
    }
}

impl From<MatcherDto> for Matcher {
    fn from(dto: MatcherDto) -> Self {
        match dto {
            MatcherDto::ShellCommandContainsAny(needles) => {
                Matcher::ShellCommandContainsAny(needles)
            }
            MatcherDto::FileWritePathPrefix(prefix) => Matcher::FileWritePathPrefix(prefix),
        }
    }
}

impl From<ContextDto> for Context {
    fn from(dto: ContextDto) -> Self {
        Context {
            approvals: dto.approvals.into_iter().map(Approval::from).collect(),
        }
    }
}

impl From<ApprovalDto> for Approval {
    fn from(dto: ApprovalDto) -> Self {
        Approval {
            scope: dto.scope.into(),
            granted_by: UserId(dto.granted_by),
            expires: Timestamp(dto.expires),
        }
    }
}

impl From<ApprovalScopeDto> for ApprovalScope {
    fn from(dto: ApprovalScopeDto) -> Self {
        match dto {
            ApprovalScopeDto::ThisAction => ApprovalScope::ThisAction,
            ApprovalScopeDto::CommandClass(pattern) => ApprovalScope::CommandClass(pattern),
        }
    }
}

impl From<&Decision> for DecisionDto {
    fn from(decision: &Decision) -> Self {
        DecisionDto {
            verdict: match decision.verdict {
                Verdict::Allow => VerdictDto::Allow,
                Verdict::Deny => VerdictDto::Deny,
                Verdict::Escalate => VerdictDto::Escalate,
                Verdict::Flag => VerdictDto::Flag,
            },
            gate_type: match decision.gate_type {
                GateType::Hard => GateTypeDto::Hard,
                GateType::Soft => GateTypeDto::Soft,
            },
            owasp: decision.owasp.as_ref().map(|clause| clause.0.clone()),
            rule_id: decision.rule_id.as_ref().map(|id| id.0.clone()),
            lane: match decision.lane {
                Lane::Deterministic => LaneDto::Deterministic,
                Lane::Semantic => LaneDto::Semantic,
            },
            rationale: decision.rationale.clone(),
        }
    }
}
