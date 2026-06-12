//! The wire between the Python host and the PDP: parse a canonical action and a context in;
//! serialize a decision out, against an already-compiled `Policy`. serde lives here at the
//! edge, never in the core. ADR-0019: the org policy is compiled once into a `Policy`
//! (parse-once handle) and reused for many decisions, so only the per-decision action and
//! context cross this wire; the schema/policy/entity parsing happens at compile time in the
//! binding's `CompiledPolicy::new`, not here on the hot path.
//!
//! Errors use anyhow: the binding surfaces them to Python as ValueError, nobody branches
//! on variants.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use policy_decision::canonical_action::{
    ActionKind, AgentId, CanonicalAction, Provenance, ResourceId, SessionId, Timestamp,
};
use policy_decision::context::{Approval, ApprovalScope, Context, UserId};
use policy_decision::decide as decide_action;
use policy_decision::decision::{Decision, GateType, Verdict};
use policy_decision::policy::{Lane, Policy};

/// Parse the per-decision inputs, run `decide` against the compiled `policy`, and serialize
/// the decision. The hot path: no schema/policy/entity parsing (that is paid once at
/// compile time). A malformed action or context surfaces to Python as `ValueError`.
pub fn decide(policy: &Policy, action_json: &str, context_json: &str) -> Result<String> {
    let action: CanonicalAction = serde_json::from_str::<ActionDto>(action_json)?.try_into()?;
    let context: Context = serde_json::from_str::<ContextDto>(context_json)?.into();

    let decision = decide_action(&action, policy, &context);
    Ok(serde_json::to_string(&DecisionDto::from(&decision))?)
}

// --- input DTOs: the full canonical action + context ------------------------------

#[derive(Deserialize)]
struct ActionDto {
    principal: String,
    action: String,
    resource: String,
    session_id: String,
    seq: u64,
    at: i64,
    source: ProvenanceDto,
}

#[derive(Deserialize)]
struct ProvenanceDto {
    provider: String,
    raw_payload_id: String,
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
    ResourceClass(String),
}

// --- output DTO + leaf enums ------------------------------------------------------

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

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum LaneDto {
    Deterministic,
    Semantic,
}

#[derive(Serialize)]
struct DecisionDto {
    verdict: VerdictDto,
    gate_type: GateTypeDto,
    owasp: Option<String>,
    policy_id: Option<String>,
    lane: LaneDto,
    rationale: String,
}

// --- mappings (the edge) ----------------------------------------------------------

impl TryFrom<ActionDto> for CanonicalAction {
    type Error = anyhow::Error;

    fn try_from(dto: ActionDto) -> Result<Self> {
        Ok(CanonicalAction {
            principal: AgentId(dto.principal),
            action: ActionKind::parse(&dto.action)
                .ok_or_else(|| anyhow!("unknown action kind: {}", dto.action))?,
            resource: ResourceId(dto.resource),
            session_id: SessionId(dto.session_id),
            seq: dto.seq,
            at: Timestamp(dto.at),
            source: Provenance {
                provider: dto.source.provider,
                raw_payload_id: dto.source.raw_payload_id,
            },
            // The FFI path carries no shell-command facts; that is the PEP's edge (ADR-0023).
            command: None,
        })
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
            scope: match dto.scope {
                ApprovalScopeDto::ThisAction => ApprovalScope::ThisAction,
                ApprovalScopeDto::ResourceClass(r) => ApprovalScope::ResourceClass(ResourceId(r)),
            },
            granted_by: UserId(dto.granted_by),
            expires: Timestamp(dto.expires),
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
            policy_id: decision.policy_id.as_ref().map(|id| id.0.clone()),
            lane: match decision.lane {
                Lane::Deterministic => LaneDto::Deterministic,
                Lane::Semantic => LaneDto::Semantic,
            },
            rationale: decision.rationale.clone(),
        }
    }
}
