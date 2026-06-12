//! The PDP decision: map a canonical action to a Cedar request, let Cedar decide, then
//! interpret the result into our richer verdict. Deterministic, no I/O.
//!
//! Cedar owns matching and precedence (deny-overrides, default-deny). Cedar returns only
//! Allow/Deny plus the determining policy ids; the host reconstructs the four-verdict
//! cascade (SPEC / docs/adr/0005) from those policies' annotations, in precedence order:
//! a determining `forbid` is a supreme hard Deny (unoverridable by approval). Otherwise,
//! on an Allow, the determining permits are read in order: a `requires_approval` permit
//! resolves to Allow/Soft if a scoped approval covers the action else Escalate/Soft; a
//! `semantic` permit escalates (shadowing any co-applicable hard allow); any other permit
//! is Allow/Hard. A Deny with no determining policy is Cedar's default-deny, routed to a
//! default Escalate that fails closed to Deny if it does not resolve.

use std::str::FromStr;

use cedar_policy::{
    Authorizer, Context as CedarContext, Decision as CedarDecision, Effect, EntityId,
    EntityTypeName, EntityUid, PolicyId as CedarPolicyId, Request, Response, RestrictedExpression,
};

use crate::canonical_action::CanonicalAction;
use crate::context::Context;
use crate::decision::{Decision, GateType, Verdict};
use crate::policy::{Lane, OwaspClause, Policy, PolicyId};

// Cedar entity type names for the three request axes. Protocol constants: they are the
// shape the authored policies are written against (`Agent::"..."`, `Action::"..."`,
// `DataScope::"..."`), so they live inline with the mapping, not in config.
const AGENT_TYPE: &str = "Agent";
const ACTION_TYPE: &str = "Action";
const SCOPE_TYPE: &str = "DataScope";

// The `@outcome` annotation value that marks a permit as an implicit deny the org
// delegates to scoped approval. The only outcome the host discriminates on: a plain
// permit (no `@outcome`) is a terminal hard allow.
const REQUIRES_APPROVAL: &str = "requires_approval";

/// Decide a verdict for `action` under `policy` and `context`. Pure and total: the
/// escalate lane is returned, never run here.
#[must_use]
pub fn decide(action: &CanonicalAction, policy: &Policy, context: &Context) -> Decision {
    let request = build_request(action, policy);
    let response = Authorizer::new().is_authorized(&request, policy.policies(), policy.entities());
    interpret(action, policy, context, &response)
}

/// Build the Cedar request `<principal, action, resource, {}>` from the canonical action,
/// validated against the policy's schema. The entity ids come from validated strings, the
/// type names are constants, and the closed `ActionKind` set maps only to schema-declared
/// actions, so a build failure is a broken invariant (panic), not a recoverable case.
fn build_request(action: &CanonicalAction, policy: &Policy) -> Request {
    Request::new(
        euid(AGENT_TYPE, &action.principal.0),
        euid(ACTION_TYPE, action.action.as_cedar_id()),
        euid(SCOPE_TYPE, &action.resource.0),
        request_context(action),
        Some(policy.schema()),
    )
    .expect("cedar request from a canonical action is schema-valid by construction")
}

/// Build the Cedar request context from the action's host-derived command facts (ADR-0023):
/// a classified shell command becomes `{ command: { kind } }`, the seam the rules read as
/// `context.command.kind`; an action with no facts carries an empty context (the prior
/// behavior for every file action). The schema validates the shape in `build_request`.
fn request_context(action: &CanonicalAction) -> CedarContext {
    let Some(facts) = &action.command else {
        return CedarContext::empty();
    };
    let command = RestrictedExpression::new_record([(
        "kind".to_string(),
        RestrictedExpression::new_string(facts.kind.clone()),
    )])
    .expect("command record has one unique string field: well-formed by construction");
    CedarContext::from_pairs([("command".to_string(), command)])
        .expect("a single-key context record is valid by construction")
}

fn euid(type_name: &str, id: &str) -> EntityUid {
    let ty = EntityTypeName::from_str(type_name).expect("constant cedar entity type is valid");
    let eid = EntityId::from_str(id).expect("entity id from string is infallible");
    EntityUid::from_type_name_and_id(ty, eid)
}

/// Reconstruct our verdict from Cedar's decision plus the determining policies' annotations.
fn interpret(
    action: &CanonicalAction,
    policy: &Policy,
    context: &Context,
    response: &Response,
) -> Decision {
    let reason: Vec<&CedarPolicyId> = response.diagnostics().reason().collect();

    match response.decision() {
        CedarDecision::Deny => {
            // A determining `forbid` is the supreme hard deny; an empty reason is Cedar's
            // default-deny (nothing applied), which the host routes to escalate.
            match reason
                .iter()
                .find(|id| effect(policy, id) == Some(Effect::Forbid))
            {
                Some(id) => from_policy(
                    policy,
                    id,
                    Verdict::Deny,
                    GateType::Hard,
                    "explicit org forbid matched: supreme, unoverridable by approval",
                ),
                None => default_escalate(),
            }
        }
        CedarDecision::Allow => {
            // 2a. RequiresApproval: an implicit deny the org delegates to scoped approval.
            if let Some(id) = reason.iter().find(|id| requires_approval(policy, id)) {
                return if context.has_approval_for(action) {
                    from_policy(
                        policy,
                        id,
                        Verdict::Allow,
                        GateType::Soft,
                        "requires_approval satisfied by a valid in-scope approval",
                    )
                } else {
                    from_policy(
                        policy,
                        id,
                        Verdict::Escalate,
                        GateType::Soft,
                        "requires_approval with no in-scope approval: escalate to human",
                    )
                };
            }

            // 2b. A semantic permit shadows any co-applicable hard allow: escalate to judge.
            if let Some(id) = reason
                .iter()
                .find(|id| lane(policy, id) == Some(Lane::Semantic))
            {
                return from_policy(
                    policy,
                    id,
                    Verdict::Escalate,
                    GateType::Soft,
                    "semantic clause applies: escalate to the judge (the host runs it)",
                );
            }

            // 2c. A plain permit (annotated hard_allow, or any permit) terminally allows.
            match reason.first() {
                Some(id) => from_policy(
                    policy,
                    id,
                    Verdict::Allow,
                    GateType::Hard,
                    "explicit org permit matched, no semantic or approval clause applies",
                ),
                // Allow with no determining policy is not reachable from a parsed policy set;
                // treat defensively as the default escalate rather than a silent allow.
                None => default_escalate(),
            }
        }
    }
}

/// The engine default when no policy resolves the action: escalate, fail closed to Deny
/// downstream if unresolved. No policy matched, so none is cited.
fn default_escalate() -> Decision {
    Decision {
        verdict: Verdict::Escalate,
        gate_type: GateType::Soft,
        owasp: None,
        policy_id: None,
        lane: Lane::Deterministic,
        rationale: "no applicable policy: default escalate, fail closed to Deny if unresolved"
            .to_string(),
    }
}

/// Build a decision citing the determining Cedar policy: its `@id` and `@owasp` annotations
/// become the audit fields, and its `@lane` (default Deterministic) the resolving lane.
fn from_policy(
    policy: &Policy,
    id: &CedarPolicyId,
    verdict: Verdict,
    gate_type: GateType,
    rationale: &str,
) -> Decision {
    Decision {
        verdict,
        gate_type,
        owasp: annotation(policy, id, "owasp").map(|s| OwaspClause(s.to_string())),
        policy_id: annotation(policy, id, "id").map(|s| PolicyId(s.to_string())),
        lane: lane(policy, id).unwrap_or(Lane::Deterministic),
        rationale: rationale.to_string(),
    }
}

fn effect(policy: &Policy, id: &CedarPolicyId) -> Option<Effect> {
    policy.policies().policy(id).map(|p| p.effect())
}

fn requires_approval(policy: &Policy, id: &CedarPolicyId) -> bool {
    annotation(policy, id, "outcome") == Some(REQUIRES_APPROVAL)
}

fn lane(policy: &Policy, id: &CedarPolicyId) -> Option<Lane> {
    annotation(policy, id, "lane").and_then(Lane::parse)
}

fn annotation<'a>(policy: &'a Policy, id: &CedarPolicyId, key: &str) -> Option<&'a str> {
    policy.policies().policy(id).and_then(|p| p.annotation(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_action::{
        ActionKind, AgentId, CommandFacts, Provenance, ResourceId, SessionId, Timestamp,
    };

    // A minimal plane whose only rule gates on the request context: a classified shell command
    // requires approval. This is the seam ADR-0023 opens (build_request now carries
    // `context.command.kind`); these tests prove a host-derived fact reaches Cedar and drives
    // the verdict, in isolation from the PEP that will produce the fact.
    const SCHEMA: &str = r"
        entity Agent;
        entity DataScope = { sensitivity: String, pii: Bool };
        action execute appliesTo {
            principal: [Agent],
            resource: [DataScope],
            context: { command: { kind: String } },
        };
    ";
    const POLICY: &str = r#"
        @id("approve-package-install")
        @owasp("ASI05")
        @outcome("requires_approval")
        @lane("deterministic")
        permit(principal, action == Action::"execute", resource)
        when { context.command.kind == "package_install" };
    "#;
    const ENTITIES: &str = r#"[
        { "uid": { "type": "Agent", "id": "agent-1" }, "attrs": {}, "parents": [] },
        { "uid": { "type": "DataScope", "id": "shell" }, "attrs": { "sensitivity": "system", "pii": false }, "parents": [] }
    ]"#;

    fn execute_with(command: Option<CommandFacts>) -> CanonicalAction {
        CanonicalAction {
            principal: AgentId("agent-1".into()),
            action: ActionKind::Execute,
            resource: ResourceId("shell".into()),
            session_id: SessionId("s1".into()),
            seq: 0,
            at: Timestamp(0),
            source: Provenance {
                provider: "test".into(),
                raw_payload_id: "s1".into(),
            },
            command,
        }
    }

    #[test]
    fn classified_command_in_context_drives_the_verdict() {
        let policy = Policy::from_sources(SCHEMA, POLICY, ENTITIES).expect("valid inline plane");
        let decision = decide(
            &execute_with(Some(CommandFacts {
                kind: "package_install".into(),
            })),
            &policy,
            &Context::default(),
        );
        // The requires_approval permit applied (so the context value reached Cedar), and with no
        // approval it escalates.
        assert_eq!(decision.verdict, Verdict::Escalate);
        assert_eq!(decision.gate_type, GateType::Soft);
        assert_eq!(
            decision.policy_id.as_ref().map(|id| id.0.as_str()),
            Some("approve-package-install")
        );
    }

    #[test]
    fn an_unnamed_command_kind_matches_no_policy() {
        let policy = Policy::from_sources(SCHEMA, POLICY, ENTITIES).expect("valid inline plane");
        let decision = decide(
            &execute_with(Some(CommandFacts { kind: "ls".into() })),
            &policy,
            &Context::default(),
        );
        // A kind the policy does not name: no permit applies -> Cedar default-deny -> the host's
        // default escalate citing no policy. Proves the context value is actually read, not ignored.
        assert_eq!(decision.verdict, Verdict::Escalate);
        assert_eq!(decision.policy_id, None);
    }
}
