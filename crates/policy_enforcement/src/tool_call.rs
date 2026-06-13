//! Parse a provider PreToolUse payload into a typed [`ToolCall`]. Cross-provider by design
//! (ADR-0007): Claude `Edit`/`Write`/`MultiEdit` carry the target in `tool_input.file_path`;
//! Codex `apply_patch` carries the raw patch text in `tool_input.command`, with the file path
//! in its `*** (Add|Update|Delete) File:` headers. The header parsing mirrors the proven
//! shell logic in `repo_alignment/hooks/lib.sh:aa_edited_paths`.

use serde::Deserialize;

/// A provider tool call reduced to what the PEP needs: the tool kind, the file path it targets
/// (if any), and the session it belongs to. A well-formed payload for a non-edit tool yields
/// `target_path = None` (the call is simply ungoverned, not an error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub tool_name: String,
    pub target_path: Option<String>,
    /// The raw command line for a shell tool (Claude/Codex `Bash`); `None` for non-command
    /// tools. The classifier (ADR-0023) reduces it to a kind; the raw string never reaches policy.
    pub command: Option<String>,
    pub session_id: String,
}

#[derive(Deserialize)]
struct RawPayload {
    tool_name: String,
    #[serde(default)]
    tool_input: RawToolInput,
    #[serde(default)]
    session_id: String,
}

#[derive(Deserialize, Default)]
struct RawToolInput {
    #[serde(default)]
    file_path: Option<String>,
    #[serde(default)]
    command: Option<String>,
}

impl ToolCall {
    /// Parse the PreToolUse JSON. The only error is malformed JSON (the caller fails closed);
    /// the target path is taken from `file_path`, else from the first `apply_patch` header.
    pub fn parse(payload_json: &str) -> Result<ToolCall, serde_json::Error> {
        let raw: RawPayload = serde_json::from_str(payload_json)?;
        let target_path = raw.tool_input.file_path.clone().or_else(|| {
            raw.tool_input
                .command
                .as_deref()
                .and_then(first_patched_path)
        });
        Ok(ToolCall {
            tool_name: raw.tool_name,
            target_path,
            command: raw.tool_input.command,
            session_id: raw.session_id,
        })
    }
}

/// The first file path named in an apply_patch body's `*** (Add|Update|Delete) File:` header.
fn first_patched_path(patch: &str) -> Option<String> {
    const PREFIXES: [&str; 3] = ["*** Add File: ", "*** Update File: ", "*** Delete File: "];
    patch.lines().find_map(|line| {
        PREFIXES
            .iter()
            .find_map(|prefix| line.strip_prefix(prefix))
            .map(|rest| rest.trim().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_edit_carries_file_path() {
        let payload =
            r#"{"tool_name":"Write","tool_input":{"file_path":"/repo/.env"},"session_id":"s1"}"#;
        let call = ToolCall::parse(payload).unwrap();
        assert_eq!(call.tool_name, "Write");
        assert_eq!(call.target_path.as_deref(), Some("/repo/.env"));
        assert_eq!(call.session_id, "s1");
    }

    #[test]
    fn codex_apply_patch_path_comes_from_the_header() {
        let payload = r#"{"tool_name":"apply_patch","tool_input":{"command":"*** Begin Patch\n*** Update File: /repo/src/lib.rs\n+x\n*** End Patch"},"session_id":"s2"}"#;
        let call = ToolCall::parse(payload).unwrap();
        assert_eq!(call.tool_name, "apply_patch");
        assert_eq!(call.target_path.as_deref(), Some("/repo/src/lib.rs"));
    }

    #[test]
    fn bash_tool_exposes_the_command_and_no_path() {
        let payload = r#"{"tool_name":"Bash","tool_input":{"command":"npm install lodash"},"session_id":"s3"}"#;
        let call = ToolCall::parse(payload).unwrap();
        assert_eq!(call.target_path, None);
        assert_eq!(call.command.as_deref(), Some("npm install lodash"));
    }

    #[test]
    fn malformed_json_is_an_error() {
        assert!(ToolCall::parse("{not json").is_err());
    }
}
