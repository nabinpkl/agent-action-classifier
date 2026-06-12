//! Conformance-corpus loader: parse an external spec under `corpus/<name>/` into the domain
//! types, mapping at this edge. The org policy is a Cedar schema + policy source + entity
//! store, validated as a unit by `Policy::from_sources` (ADR-0017, ADR-0018); only the cases
//! (actions + context + expectations) need bespoke DTOs. Each corpus is one named directory
//! (e.g. `asi05`, `org_graph`); `load_corpus` is generic over the name, and `check` replays a
//! loaded corpus through `decide` and returns the exact-match failures.
//!
//! This is test/bench harness, not library API: nobody branches on its failure modes,
//! so it uses `anyhow` (per the rust-coding skill, anyhow for glue). The DTOs mirror the
//! JSON and map explicitly into the domain types.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow, bail};
use serde::Deserialize;

use policy_decision::canonical_action::{
    ActionKind, AgentId, CanonicalAction, CommandFacts, Provenance, ResourceId, SessionId,
    Timestamp,
};
use policy_decision::context::{Approval, ApprovalScope, Context as DecisionContext, UserId};
use policy_decision::decide;
use policy_decision::decision::{Decision, GateType, Verdict};
use policy_decision::policy::Policy;

// Audit-only fields that do not affect a verdict get fixed synthetic values, so the
// JSON cases only author the parts that matter (principal, action, resource, timing,
// approvals).
const CORPUS_SESSION: &str = "corpus-session";
const CORPUS_PROVIDER: &str = "corpus";
const CORPUS_RAW: &str = "corpus-raw";

/// One loaded case: inputs plus the four expected conformance keys.
pub struct Case {
    pub name: String,
    pub action: CanonicalAction,
    pub context: DecisionContext,
    pub expect: Expectation,
}

/// The expected SPEC conformance keys (`verdict` / `gate_type` / `owasp` / `policy_id`).
pub struct Expectation {
    pub verdict: Verdict,
    pub gate_type: GateType,
    pub owasp: Option<String>,
    pub policy_id: Option<String>,
}

/// A loaded corpus: the one authored policy plus its cases.
pub struct Corpus {
    pub policy: Policy,
    pub cases: Vec<Case>,
}

/// Load `corpus/<name>/{policy.cedarschema,policy.cedar,entities.json,cases.json}` from the
/// workspace root. Fails loud: a missing file, malformed Cedar/JSON, or an empty case set
/// all error.
pub fn load_corpus(name: &str) -> Result<Corpus> {
    let dir = corpus_root().join(name);
    let policy = load_policy(
        &dir.join("policy.cedarschema"),
        &dir.join("policy.cedar"),
        &dir.join("entities.json"),
    )?;
    let cases = load_cases(&dir.join("cases.json"))?;
    if cases.is_empty() {
        bail!(
            "conformance corpus loaded zero cases from {}",
            dir.display()
        );
    }
    Ok(Corpus { policy, cases })
}

/// Replay every case through `decide` and return one `[name] diffs` line per case whose
/// decision does not exactly match its expectation. Empty result = full conformance. The
/// black-box runner shared by every corpus's conformance test (only ever calls `decide`).
#[must_use]
pub fn check(corpus: &Corpus) -> Vec<String> {
    corpus
        .cases
        .iter()
        .filter_map(|case| {
            let got = decide(&case.action, &corpus.policy, &case.context);
            mismatch(case, &got).map(|why| format!("[{}] {why}", case.name))
        })
        .collect()
}

/// `None` if every conformance key matches; otherwise the joined differences.
fn mismatch(case: &Case, got: &Decision) -> Option<String> {
    let want = &case.expect;
    let got_owasp = got.owasp.as_ref().map(|clause| clause.0.clone());
    let got_policy_id = got.policy_id.as_ref().map(|id| id.0.clone());

    let mut diffs = Vec::new();
    if got.verdict != want.verdict {
        diffs.push(format!(
            "verdict: got {:?}, want {:?}",
            got.verdict, want.verdict
        ));
    }
    if got.gate_type != want.gate_type {
        diffs.push(format!(
            "gate_type: got {:?}, want {:?}",
            got.gate_type, want.gate_type
        ));
    }
    if got_owasp != want.owasp {
        diffs.push(format!("owasp: got {got_owasp:?}, want {:?}", want.owasp));
    }
    if got_policy_id != want.policy_id {
        diffs.push(format!(
            "policy_id: got {got_policy_id:?}, want {:?}",
            want.policy_id
        ));
    }

    if diffs.is_empty() {
        None
    } else {
        Some(diffs.join("; "))
    }
}

fn corpus_root() -> PathBuf {
    // The corpus is the shared executable spec at the workspace root, not inside this
    // crate: walk up from crates/policy_decision/ to the repo root's corpus/.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
}

fn load_policy(schema_path: &Path, policy_path: &Path, entities_path: &Path) -> Result<Policy> {
    let schema_src = std::fs::read_to_string(schema_path)
        .with_context(|| format!("reading {}", schema_path.display()))?;
    let policy_src = std::fs::read_to_string(policy_path)
        .with_context(|| format!("reading {}", policy_path.display()))?;
    let entities_src = std::fs::read_to_string(entities_path)
        .with_context(|| format!("reading {}", entities_path.display()))?;

    // Cedar owns schema/policy/entity parsing and validation (ADR-0018); the loader just
    // feeds it the three sources and surfaces a validation failure loudly.
    Policy::from_sources(&schema_src, &policy_src, &entities_src)
        .with_context(|| format!("loading the org policy from {}", schema_path.display()))
}

fn load_cases(path: &Path) -> Result<Vec<Case>> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let dtos: Vec<CaseDto> =
        serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))?;
    dtos.into_iter().map(Case::try_from).collect()
}

// --- DTOs mirroring the JSON, mapped into the domain types at this edge ---

#[derive(Deserialize)]
struct CaseDto {
    name: String,
    action: ActionDto,
    #[serde(default)]
    context: ContextDto,
    expect: ExpectDto,
}

#[derive(Deserialize)]
struct ActionDto {
    principal: String,
    action: String,
    resource: String,
    #[serde(default)]
    at: i64,
    /// Host-derived command facts for an `execute` case; absent for file actions.
    #[serde(default)]
    command: Option<CommandDto>,
}

#[derive(Deserialize)]
struct CommandDto {
    kind: String,
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

#[derive(Deserialize)]
struct ExpectDto {
    verdict: String,
    gate_type: String,
    owasp: Option<String>,
    policy_id: Option<String>,
}

impl TryFrom<CaseDto> for Case {
    type Error = anyhow::Error;

    fn try_from(dto: CaseDto) -> Result<Self> {
        let action = CanonicalAction {
            principal: AgentId(dto.action.principal),
            action: ActionKind::parse(&dto.action.action).ok_or_else(|| {
                anyhow!("[{}] unknown action kind: {}", dto.name, dto.action.action)
            })?,
            resource: ResourceId(dto.action.resource),
            session_id: SessionId(CORPUS_SESSION.to_string()),
            seq: 0,
            at: Timestamp(dto.action.at),
            source: Provenance {
                provider: CORPUS_PROVIDER.to_string(),
                raw_payload_id: CORPUS_RAW.to_string(),
            },
            command: dto.action.command.map(|c| CommandFacts { kind: c.kind }),
        };
        let context = DecisionContext {
            approvals: dto
                .context
                .approvals
                .into_iter()
                .map(Approval::from)
                .collect(),
        };
        let expect = Expectation {
            verdict: parse_verdict(&dto.expect.verdict)
                .ok_or_else(|| anyhow!("[{}] unknown verdict: {}", dto.name, dto.expect.verdict))?,
            gate_type: parse_gate(&dto.expect.gate_type).ok_or_else(|| {
                anyhow!("[{}] unknown gate_type: {}", dto.name, dto.expect.gate_type)
            })?,
            owasp: dto.expect.owasp,
            policy_id: dto.expect.policy_id,
        };
        Ok(Case {
            name: dto.name,
            action,
            context,
            expect,
        })
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

fn parse_verdict(s: &str) -> Option<Verdict> {
    match s {
        "allow" => Some(Verdict::Allow),
        "deny" => Some(Verdict::Deny),
        "escalate" => Some(Verdict::Escalate),
        "flag" => Some(Verdict::Flag),
        _ => None,
    }
}

fn parse_gate(s: &str) -> Option<GateType> {
    match s {
        "hard" => Some(GateType::Hard),
        "soft" => Some(GateType::Soft),
        _ => None,
    }
}
