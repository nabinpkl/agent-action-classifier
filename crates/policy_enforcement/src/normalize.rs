//! Normalize a provider [`ToolCall`] into the canonical action the PDP decides on. This is the
//! genuinely novel part of the PEP (there is no industry standard for the tool-call -> action
//! schema yet): two closed resolvers, plus the resource map that binds a real file path to an
//! org `DataScope`. The orchestrator (`main`) calls these in order so an ungoverned tool or
//! path short-circuits before any policy is loaded.
//!
//! v0 governs mutation tools only (Edit/Write/MultiEdit/apply_patch -> `Write`). Read, Bash,
//! and the share/delete kinds are deferred (ADR-0021): a Bash command's action and resource
//! need command-line parsing, a separate rabbit hole.

use globset::{Glob, GlobSet, GlobSetBuilder};
use policy_decision::canonical_action::{
    ActionKind, AgentId, CanonicalAction, Provenance, ResourceId, SessionId, Timestamp,
};
use serde::Deserialize;

use crate::tool_call::ToolCall;

/// The action kind for a tool name, or `None` if the tool is ungoverned in v0. A new governed
/// tool is a one-line change here, not new architecture (the action set is closed by design).
#[must_use]
pub fn action_for(tool_name: &str) -> Option<ActionKind> {
    match tool_name {
        "Write" | "Edit" | "MultiEdit" | "apply_patch" => Some(ActionKind::Write),
        _ => None,
    }
}

/// The PAP's path -> scope binding: an ordered list of `(glob, DataScope id)`. First authored
/// glob wins, so order is precedence. This is config (loaded from JSON), not constants: which
/// real paths count as which scope is an operator decision.
pub struct ResourceMap {
    set: GlobSet,
    scopes: Vec<String>,
}

#[derive(Deserialize)]
struct ResourceMapEntry {
    glob: String,
    scope: String,
}

impl ResourceMap {
    /// Build from the JSON array of `{glob, scope}`. Fails loud on a malformed glob or JSON,
    /// like the policy artifacts — a broken map should not silently match nothing.
    pub fn from_json(json: &str) -> Result<ResourceMap, String> {
        let entries: Vec<ResourceMapEntry> =
            serde_json::from_str(json).map_err(|e| format!("parsing resource map: {e}"))?;
        let mut builder = GlobSetBuilder::new();
        let mut scopes = Vec::with_capacity(entries.len());
        for entry in entries {
            let glob =
                Glob::new(&entry.glob).map_err(|e| format!("bad glob {:?}: {e}", entry.glob))?;
            builder.add(glob);
            scopes.push(entry.scope);
        }
        let set = builder
            .build()
            .map_err(|e| format!("building glob set: {e}"))?;
        Ok(ResourceMap { set, scopes })
    }

    /// The `DataScope` id for the first authored glob that matches `path`, or `None` (an
    /// ungoverned path: the call touches nothing the org declared as a resource).
    #[must_use]
    pub fn scope_for(&self, path: &str) -> Option<&str> {
        self.set
            .matches(path)
            .into_iter()
            .min()
            .map(|i| self.scopes[i].as_str())
    }
}

/// Build the canonical action from the resolved kind + scope and the call's facts. `seq` is 0
/// (the stateful lane is deferred); `at` is the host clock; `raw_payload_id` is the session id
/// as a v0 placeholder (the audit log, TASKS #5, will carry a per-call id).
#[must_use]
pub fn canonical_action(
    action: ActionKind,
    scope: &str,
    call: &ToolCall,
    agent_id: &str,
    provider: &str,
    now: Timestamp,
) -> CanonicalAction {
    CanonicalAction {
        principal: AgentId(agent_id.to_string()),
        action,
        resource: ResourceId(scope.to_string()),
        session_id: SessionId(call.session_id.clone()),
        seq: 0,
        at: now,
        source: Provenance {
            provider: provider.to_string(),
            raw_payload_id: call.session_id.clone(),
        },
        // v0 governs mutation tools, which carry no command facts; the shell-command path
        // (ADR-0023) sets this when it lands.
        command: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = r#"[
        { "glob": "**/.env", "scope": "secrets" },
        { "glob": "**/secrets/**", "scope": "secrets" },
        { "glob": "**/*.config.*", "scope": "app-config" }
    ]"#;

    #[test]
    fn mutation_tools_map_to_write_others_are_ungoverned() {
        assert_eq!(action_for("Write"), Some(ActionKind::Write));
        assert_eq!(action_for("Edit"), Some(ActionKind::Write));
        assert_eq!(action_for("apply_patch"), Some(ActionKind::Write));
        assert_eq!(action_for("Read"), None);
        assert_eq!(action_for("Bash"), None);
    }

    #[test]
    fn resource_map_resolves_a_governed_path() {
        let map = ResourceMap::from_json(MAP).unwrap();
        assert_eq!(map.scope_for("/repo/experiments/.env"), Some("secrets"));
        assert_eq!(map.scope_for("/repo/secrets/key.txt"), Some("secrets"));
        assert_eq!(map.scope_for("/repo/app.config.json"), Some("app-config"));
    }

    #[test]
    fn unmapped_path_is_out_of_scope() {
        let map = ResourceMap::from_json(MAP).unwrap();
        assert_eq!(map.scope_for("/repo/src/main.rs"), None);
    }

    #[test]
    fn first_authored_glob_wins() {
        // Both globs match a path under secrets/; the earlier entry's scope is taken.
        let map = ResourceMap::from_json(
            r#"[{ "glob": "**/secrets/**", "scope": "secrets" }, { "glob": "**/*.config.*", "scope": "app-config" }]"#,
        )
        .unwrap();
        assert_eq!(
            map.scope_for("/repo/secrets/db.config.json"),
            Some("secrets")
        );
    }

    #[test]
    fn malformed_map_fails_loud() {
        assert!(ResourceMap::from_json("{not json").is_err());
    }
}
