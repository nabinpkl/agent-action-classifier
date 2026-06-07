//! Conformance-corpus loader: parse the external JSON spec under `corpus/` into the
//! pure domain types, mapping at this edge so the core stays serde-free.
//!
//! This is test/bench harness, not library API: nobody branches on its failure modes,
//! they just need a readable message, so it uses `anyhow` (per the rust-coding skill,
//! anyhow for glue). The real typed boundary, the host loading an org policy, is where
//! `thiserror` will belong. The DTOs below mirror the JSON and map explicitly into the
//! domain types; that mapping is the price of keeping serde off the core.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use policy_decision::canonical_action::{
    AgentId, CanonicalAction, Operation, Provenance, SessionId, Timestamp,
};
use policy_decision::context::{Approval, ApprovalScope, Context as DecisionContext, UserId};
use policy_decision::decision::{GateType, Verdict};
use policy_decision::policy::{Lane, Matcher, Outcome, OwaspClause, Policy, Rule, RuleId};

// Audit-only fields that do not affect a verdict get fixed synthetic values, so the
// JSON cases only have to author the parts that matter (operation, timing, approvals).
const CORPUS_AGENT: &str = "corpus-agent";
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

/// The expected SPEC conformance keys (`verdict` / `gate_type` / `owasp` / `rule_id`).
pub struct Expectation {
    pub verdict: Verdict,
    pub gate_type: GateType,
    pub owasp: Option<String>,
    pub rule_id: Option<String>,
}

/// A loaded corpus: the one authored policy plus its cases.
pub struct Asi05Corpus {
    pub policy: Policy,
    pub cases: Vec<Case>,
}

/// Load `corpus/asi05/{policy,cases}.json` from the crate root. Fails loud: a missing
/// file, malformed JSON, or an empty case set all error rather than passing silently.
pub fn load_asi05() -> Result<Asi05Corpus> {
    let dir = corpus_root().join("asi05");
    let policy = load_policy(&dir.join("policy.json"))?;
    let cases = load_cases(&dir.join("cases.json"))?;
    if cases.is_empty() {
        bail!(
            "conformance corpus loaded zero cases from {}",
            dir.display()
        );
    }
    Ok(Asi05Corpus { policy, cases })
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus")
}

fn load_policy(path: &Path) -> Result<Policy> {
    let dto: PolicyDto = read_json(path)?;
    Ok(Policy {
        rules: dto.rules.into_iter().map(Rule::from).collect(),
    })
}

fn load_cases(path: &Path) -> Result<Vec<Case>> {
    let dtos: Vec<CaseDto> = read_json(path)?;
    Ok(dtos.into_iter().map(Case::from).collect())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes =
        std::fs::read(path).with_context(|| format!("reading corpus file {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing corpus file {}", path.display()))
}

// --- wire DTOs (mirror the JSON; serde lives only here) --------------------------

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
enum LaneDto {
    Deterministic,
    Semantic,
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
    #[serde(default)]
    at: i64,
    operation: OperationDto,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum OperationDto {
    ShellExec { command: String, cwd: String },
    FileWrite { path: String, byte_len: u64 },
    NetworkFetch { url: String },
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

#[derive(Deserialize)]
struct ExpectDto {
    verdict: VerdictDto,
    gate_type: GateTypeDto,
    #[serde(default)]
    owasp: Option<String>,
    #[serde(default)]
    rule_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum VerdictDto {
    Allow,
    Deny,
    Escalate,
    Flag,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum GateTypeDto {
    Hard,
    Soft,
}

// --- DTO -> domain mapping (the edge) --------------------------------------------

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

impl From<CaseDto> for Case {
    fn from(dto: CaseDto) -> Self {
        Case {
            name: dto.name,
            action: CanonicalAction {
                agent_id: AgentId(CORPUS_AGENT.to_string()),
                session_id: SessionId(CORPUS_SESSION.to_string()),
                seq: 0,
                at: Timestamp(dto.action.at),
                source: Provenance {
                    provider: CORPUS_PROVIDER.to_string(),
                    raw_payload_id: CORPUS_RAW.to_string(),
                },
                operation: dto.action.operation.into(),
            },
            context: DecisionContext {
                approvals: dto
                    .context
                    .approvals
                    .into_iter()
                    .map(Approval::from)
                    .collect(),
            },
            expect: Expectation {
                verdict: dto.expect.verdict.into(),
                gate_type: dto.expect.gate_type.into(),
                owasp: dto.expect.owasp,
                rule_id: dto.expect.rule_id,
            },
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

impl From<VerdictDto> for Verdict {
    fn from(dto: VerdictDto) -> Self {
        match dto {
            VerdictDto::Allow => Verdict::Allow,
            VerdictDto::Deny => Verdict::Deny,
            VerdictDto::Escalate => Verdict::Escalate,
            VerdictDto::Flag => Verdict::Flag,
        }
    }
}

impl From<GateTypeDto> for GateType {
    fn from(dto: GateTypeDto) -> Self {
        match dto {
            GateTypeDto::Hard => GateType::Hard,
            GateTypeDto::Soft => GateType::Soft,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_asi05_succeeds_and_is_nonempty() {
        let corpus = load_asi05().expect("the checked-in ASI05 corpus must load");
        assert!(!corpus.policy.rules.is_empty());
        assert!(!corpus.cases.is_empty());
    }

    #[test]
    fn malformed_json_is_an_error_not_a_silent_default() {
        let parsed: Result<PolicyDto, _> = serde_json::from_slice(b"{ not json");
        assert!(parsed.is_err(), "malformed JSON must fail loud");
    }

    #[test]
    fn unknown_matcher_kind_is_rejected() {
        // A typo'd matcher tag must error, not silently match nothing.
        let json = r#"{"id":"x","owasp":"ASI05","lane":"deterministic",
                       "match":{"bogus_matcher":[]},"outcome":"hard_deny"}"#;
        let parsed: Result<RuleDto, _> = serde_json::from_str(json);
        assert!(parsed.is_err(), "unknown matcher kind must fail loud");
    }
}
