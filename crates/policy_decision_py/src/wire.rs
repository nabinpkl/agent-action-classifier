//! The wire between the Python host and the PDP: parse a canonical action, a Cedar policy,
//! a Cedar entity store, and a context in; serialize a decision out. serde lives here at
//! the edge, never in the core. ADR-0017: the policy is now Cedar source text and the
//! entities are Cedar's entity JSON, both parsed by Cedar itself, so the bespoke policy
//! DTOs are gone; only the action and context need mapping.
//!
//! Errors use anyhow: the binding surfaces them to Python as ValueError, nobody branches
//! on variants.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use policy_decision::canonical_action::{
    ActionKind, AgentId, CanonicalAction, Provenance, ResourceId, SessionId, Timestamp,
};
use policy_decision::context::{Approval, ApprovalScope, Context, UserId};
use policy_decision::decide;
use policy_decision::decision::{Decision, GateType, Verdict};
use policy_decision::policy::{Lane, Policy};

use cedar_policy::{Entities, PolicySet};

/// Parse the inputs, run `decide`, and serialize the decision. `policy_cedar` is Cedar
/// policy source; `entities_json` is Cedar's entity JSON (the org model / PAP).
pub fn decide_json(
    action_json: &str,
    policy_cedar: &str,
    entities_json: &str,
    context_json: &str,
) -> Result<String> {
    let action: CanonicalAction = serde_json::from_str::<ActionDto>(action_json)?.try_into()?;
    let context: Context = serde_json::from_str::<ContextDto>(context_json)?.into();

    let policies: PolicySet = policy_cedar
        .parse()
        .map_err(|e| anyhow!("parsing Cedar policy: {e}"))?;
    let entities = Entities::from_json_str(entities_json, None)
        .map_err(|e| anyhow!("parsing entities: {e}"))?;
    let policy = Policy::new(policies, entities);

    let decision = decide(&action, &policy, &context);
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
