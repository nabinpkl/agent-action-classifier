//! The decision-log record (audit sink): the OPA/AAT-shaped JSON the host persists for every
//! *governed* decision — the queryable, model-independent record EU AI Act Art. 12 requires
//! (SPEC `DecisionRecord`). Written by the `enforce` binary itself: [ADR-0021](../../../docs/adr/0021-pep-as-rust-command-hook-binary.md)
//! put the PEP in Rust, so the sink follows it here. One JSON object per line, appended to the
//! audit log. Out-of-scope calls reach no decision and so leave no record (by design).
//!
//! `latency_ns` is the deterministic `decide()` cost, measured by the caller and attached here
//! (SPEC keeps it off the pure `Decision`). `prev_hash` is the chain-ready slot, null in v0;
//! the SHA-256 tamper-evident chain is roadmap.

use std::fs::OpenOptions;
use std::io::Write as _;

use policy_decision::canonical_action::CanonicalAction;
use policy_decision::decision::{Decision, GateType, Verdict};
use policy_decision::policy::Lane;
use serde::Serialize;

/// One audit record. Mirrors SPEC's `DecisionRecord`: the request, the verdict and its audit
/// fields, the measured latency, and the chain slot.
#[derive(Serialize)]
pub struct DecisionRecord {
    /// Wall-clock ns when the decision was recorded.
    at: i64,
    request: RequestRecord,
    verdict: &'static str,
    gate_type: &'static str,
    owasp: Option<String>,
    policy_id: Option<String>,
    lane: &'static str,
    rationale: String,
    latency_ns: u128,
    prev_hash: Option<String>,
}

/// The canonical action, flattened for the record (the provenance the audit needs).
#[derive(Serialize)]
struct RequestRecord {
    principal: String,
    action: String,
    resource: String,
    session_id: String,
    seq: u64,
    at: i64,
    provider: String,
    raw_payload_id: String,
}

impl DecisionRecord {
    #[must_use]
    pub fn build(
        action: &CanonicalAction,
        decision: &Decision,
        latency_ns: u128,
        now: i64,
    ) -> DecisionRecord {
        DecisionRecord {
            at: now,
            request: RequestRecord {
                principal: action.principal.0.clone(),
                action: action.action.as_cedar_id().to_string(),
                resource: action.resource.0.clone(),
                session_id: action.session_id.0.clone(),
                seq: action.seq,
                at: action.at.0,
                provider: action.source.provider.clone(),
                raw_payload_id: action.source.raw_payload_id.clone(),
            },
            verdict: verdict_str(decision.verdict),
            gate_type: gate_str(decision.gate_type),
            owasp: decision.owasp.as_ref().map(|c| c.0.clone()),
            policy_id: decision.policy_id.as_ref().map(|p| p.0.clone()),
            lane: lane_str(decision.lane),
            rationale: decision.rationale.clone(),
            latency_ns,
            prev_hash: None,
        }
    }
}

/// Append one record as a JSON line to the audit log, creating it if absent. Fails loud: a
/// write failure is *returned* so the caller fails CLOSED — an unrecordable decision must not
/// be silently allowed, since the record is the whole point (EU AI Act Art. 12).
pub fn append(path: &str, record: &DecisionRecord) -> Result<(), String> {
    let line = serde_json::to_string(record).map_err(|e| format!("serializing record: {e}"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("opening audit log {path}: {e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("appending to audit log {path}: {e}"))
}

fn verdict_str(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Allow => "allow",
        Verdict::Deny => "deny",
        Verdict::Escalate => "escalate",
        Verdict::Flag => "flag",
    }
}

fn gate_str(gate: GateType) -> &'static str {
    match gate {
        GateType::Hard => "hard",
        GateType::Soft => "soft",
    }
}

fn lane_str(lane: Lane) -> &'static str {
    match lane {
        Lane::Deterministic => "deterministic",
        Lane::Semantic => "semantic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use policy_decision::canonical_action::{
        ActionKind, AgentId, Provenance, ResourceId, SessionId, Timestamp,
    };
    use policy_decision::decision::{GateType, Verdict};
    use policy_decision::policy::{Lane, OwaspClause, PolicyId};

    fn sample_action() -> CanonicalAction {
        CanonicalAction {
            principal: AgentId("agent-1".into()),
            action: ActionKind::Write,
            resource: ResourceId("secrets".into()),
            session_id: SessionId("s1".into()),
            seq: 0,
            at: Timestamp(1000),
            source: Provenance {
                provider: "claude".into(),
                raw_payload_id: "s1".into(),
            },
        }
    }

    fn deny_decision() -> Decision {
        Decision {
            verdict: Verdict::Deny,
            gate_type: GateType::Hard,
            owasp: Some(OwaspClause("ASI05".into())),
            policy_id: Some(PolicyId("deny-secret-write".into())),
            lane: Lane::Deterministic,
            rationale: "explicit org forbid matched".into(),
        }
    }

    #[test]
    fn record_serializes_with_the_audit_fields() {
        let record = DecisionRecord::build(&sample_action(), &deny_decision(), 4200, 9999);
        let value: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();
        assert_eq!(value["verdict"], "deny");
        assert_eq!(value["gate_type"], "hard");
        assert_eq!(value["owasp"], "ASI05");
        assert_eq!(value["policy_id"], "deny-secret-write");
        assert_eq!(value["lane"], "deterministic");
        assert_eq!(value["latency_ns"], 4200);
        assert_eq!(value["request"]["resource"], "secrets");
        assert_eq!(value["request"]["action"], "write");
        assert!(value["prev_hash"].is_null());
    }

    #[test]
    fn default_escalate_records_null_clause_and_id() {
        let escalate = Decision {
            verdict: Verdict::Escalate,
            gate_type: GateType::Soft,
            owasp: None,
            policy_id: None,
            lane: Lane::Deterministic,
            rationale: "no applicable policy".into(),
        };
        let record = DecisionRecord::build(&sample_action(), &escalate, 1, 1);
        let value: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();
        assert_eq!(value["verdict"], "escalate");
        assert!(value["owasp"].is_null());
        assert!(value["policy_id"].is_null());
    }
}
