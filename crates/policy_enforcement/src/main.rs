//! `enforce`: the agent-action governance PEP, realized as a per-call command-hook binary
//! (ADR-0021). Reads a provider PreToolUse payload on stdin, normalizes it to the canonical
//! action, decides it against the org policy (the compiled Cedar handle), and returns the
//! verdict to the provider: allow (exit 0) / deny (exit 2 + reason on stderr) / ask
//! (`permissionDecision` JSON on stdout). One binary serves both Claude and Codex.
//!
//! Fails CLOSED: any internal error (bad args/payload/plane) becomes a deny with a loud
//! reason, never a silent fail-open. Ungoverned tool calls (an ungoverned tool kind, or a
//! path that maps to no scope) short-circuit to a plain proceed before any policy is loaded,
//! so the live hook is cheap and low-noise on the common case.

mod command_classifier;
mod decision_record;
mod hook_response;
mod normalize;
mod tool_call;

use std::io::Read as _;
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use policy_decision::canonical_action::{ActionKind, CommandFacts, Timestamp};
use policy_decision::context::Context;
use policy_decision::decide;
use policy_decision::policy::Policy;

use crate::command_classifier::CommandClassifier;
use crate::decision_record::DecisionRecord;
use crate::hook_response::HookResponse;
use crate::normalize::ResourceMap;
use crate::tool_call::ToolCall;

fn main() -> ExitCode {
    match run() {
        Ok(response) => response.emit(),
        Err(err) => hook_response::fail_closed(&err).emit(),
    }
}

fn run() -> Result<HookResponse, String> {
    let args = Args::parse()?;

    let mut payload = String::new();
    std::io::stdin()
        .read_to_string(&mut payload)
        .map_err(|e| format!("reading stdin: {e}"))?;
    let call = ToolCall::parse(&payload).map_err(|e| format!("parsing payload: {e}"))?;

    // Ungoverned tool kind -> proceed without touching policy or config (the common path).
    let Some(action_kind) = normalize::action_for(&call.tool_name) else {
        return Ok(hook_response::proceed());
    };

    // Resolve what this call touches: a file mutation -> a DataScope by path; a shell command ->
    // the shell scope + its classified kind. A call that touches nothing governed proceeds.
    let Some((scope, command)) = resolve_target(action_kind, &call, &args)? else {
        return Ok(hook_response::proceed());
    };

    let action = normalize::canonical_action(
        action_kind,
        &scope,
        command,
        &call,
        &args.agent_id,
        &args.provider,
        now(),
    );
    let policy = load_plane(&args.plane)?;

    let started = Instant::now();
    let decision = decide(&action, &policy, &Context::default());
    let latency_ns = started.elapsed().as_nanos();

    // Audit the governed decision before returning the verdict. A write failure propagates and
    // fails closed: an unrecordable decision must not be silently allowed (decision_record).
    if let Some(path) = &args.audit_log {
        let record = DecisionRecord::build(&action, &decision, latency_ns, now().0);
        decision_record::append(path, &record)?;
    }
    Ok(hook_response::from_decision(&decision))
}

/// Resolve the governed `(resource scope, command facts)` for a tool call, or `None` if it
/// touches nothing the org governs (an unmapped path, an unclassified command). Fails CLOSED if
/// the call needs a config map this invocation was not given: a file mutation wired without a
/// resource map, or a shell command wired without signatures, is a misconfiguration, not a free
/// pass.
fn resolve_target(
    kind: ActionKind,
    call: &ToolCall,
    args: &Args,
) -> Result<Option<(String, Option<CommandFacts>)>, String> {
    match kind {
        ActionKind::Write => {
            let Some(path) = call.target_path.as_deref() else {
                return Ok(None);
            };
            let map_path = args
                .resource_map
                .as_deref()
                .ok_or("a file-mutation tool was governed but --resource-map was not configured")?;
            let resource_map = load_resource_map(map_path)?;
            Ok(resource_map.scope_for(path).map(|s| (s.to_string(), None)))
        }
        ActionKind::Execute => {
            let Some(command) = call.command.as_deref() else {
                return Ok(None);
            };
            let sigs_path = args.command_signatures.as_deref().ok_or(
                "a shell command was governed but --command-signatures was not configured",
            )?;
            let classifier = load_command_classifier(sigs_path)?;
            Ok(classifier.classify(command).map(|kind| {
                (
                    normalize::SHELL_SCOPE.to_string(),
                    Some(CommandFacts {
                        kind: kind.to_string(),
                    }),
                )
            }))
        }
        // action_for only yields Write or Execute in v0; other kinds are ungoverned.
        _ => Ok(None),
    }
}

fn load_resource_map(path: &str) -> Result<ResourceMap, String> {
    let json =
        std::fs::read_to_string(path).map_err(|e| format!("reading resource map {path}: {e}"))?;
    ResourceMap::from_json(&json)
}

fn load_command_classifier(path: &str) -> Result<CommandClassifier, String> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| format!("reading command signatures {path}: {e}"))?;
    CommandClassifier::from_json(&json)
}

/// Load and compile the plane (the three PAP artifacts) from a directory. v0 reads them per
/// call (the documented ~0.3ms parse); the warm-handle HTTP sidecar is the roadmap (ADR-0021).
fn load_plane(dir: &str) -> Result<Policy, String> {
    let schema = read(dir, "policy.cedarschema")?;
    let policy = read(dir, "policy.cedar")?;
    let entities = read(dir, "entities.json")?;
    Policy::from_sources(&schema, &policy, &entities).map_err(|e| format!("compiling plane: {e}"))
}

fn read(dir: &str, file: &str) -> Result<String, String> {
    std::fs::read_to_string(format!("{dir}/{file}")).map_err(|e| format!("reading {file}: {e}"))
}

fn now() -> Timestamp {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_nanos()).unwrap_or(i64::MAX));
    Timestamp(ns)
}

/// The hook command's flags: where the plane lives, who this agent is, and the maps for the
/// tool kinds this wiring governs. `--resource-map` (file mutations) and `--command-signatures`
/// (shell commands) are each optional; a governed call that needs a map it was not given fails
/// closed (see `resolve_target`). `--audit-log` is where to append the decision record.
struct Args {
    plane: String,
    resource_map: Option<String>,
    command_signatures: Option<String>,
    agent_id: String,
    provider: String,
    audit_log: Option<String>,
}

impl Args {
    fn parse() -> Result<Args, String> {
        let mut plane = None;
        let mut resource_map = None;
        let mut command_signatures = None;
        let mut agent_id = None;
        let mut provider = None;
        let mut audit_log = None;
        let mut it = std::env::args().skip(1);
        while let Some(flag) = it.next() {
            let value = match flag.as_str() {
                "--plane"
                | "--resource-map"
                | "--command-signatures"
                | "--agent-id"
                | "--provider"
                | "--audit-log" => it
                    .next()
                    .ok_or_else(|| format!("missing value for {flag}"))?,
                other => return Err(format!("unknown argument: {other}")),
            };
            match flag.as_str() {
                "--plane" => plane = Some(value),
                "--resource-map" => resource_map = Some(value),
                "--command-signatures" => command_signatures = Some(value),
                "--agent-id" => agent_id = Some(value),
                "--provider" => provider = Some(value),
                "--audit-log" => audit_log = Some(value),
                _ => unreachable!(),
            }
        }
        Ok(Args {
            plane: plane.ok_or("missing --plane")?,
            resource_map,
            command_signatures,
            agent_id: agent_id.ok_or("missing --agent-id")?,
            provider: provider.unwrap_or_else(|| "unknown".to_string()),
            audit_log,
        })
    }
}
