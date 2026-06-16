//! The canonical action: one normalized agent tool call, the unit the PDP decides on.
//!
//! Modeled org-first (ADR-0017): a tool call is `principal × action × resource`, the
//! three axes Cedar evaluates. The principal is the agent; the action is a small closed
//! kind; the resource is a data scope (its attributes live in the entity store, the PAP,
//! not here). This is plain data with no Cedar types: the mapping to a Cedar request
//! lives at the engine edge (`evaluate`), so this stays a pure schema.

/// Which agent took the action (the Cedar principal: `Agent::"<id>"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

/// Trajectory id. Carried for the future stateful lane; not yet matched on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

/// Unix epoch nanoseconds. A plain ordered scalar so the pure core needs no clock;
/// "now" for an evaluation is the action's own `at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub i64);

/// Where the action came from, for audit: provider name plus an opaque id pointing
/// back at the raw provider payload the canonical action was normalized from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub provider: String,
    pub raw_payload_id: String,
}

/// The closed set of action kinds (the Cedar action: `Action::"<id>"`). Small and
/// closed by design; the resource/scope space is open. A new kind is a code change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Read,
    Write,
    Share,
    Delete,
    Execute,
}

impl ActionKind {
    /// The Cedar action entity id this kind maps to.
    #[must_use]
    pub fn as_cedar_id(self) -> &'static str {
        match self {
            ActionKind::Read => "read",
            ActionKind::Write => "write",
            ActionKind::Share => "share",
            ActionKind::Delete => "delete",
            ActionKind::Execute => "execute",
        }
    }

    /// Parse a wire/corpus action string. `None` on an unknown kind (the caller fails
    /// loud rather than guessing a default).
    #[must_use]
    pub fn parse(s: &str) -> Option<ActionKind> {
        match s {
            "read" => Some(ActionKind::Read),
            "write" => Some(ActionKind::Write),
            "share" => Some(ActionKind::Share),
            "delete" => Some(ActionKind::Delete),
            "execute" => Some(ActionKind::Execute),
            _ => None,
        }
    }
}

/// The data scope a tool call touches (the Cedar resource: `DataScope::"<id>"`). Its
/// attributes (sensitivity, pii) live in the entity store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceId(pub String);

/// Host-derived facts about a shell-command action, surfaced to policy as the Cedar request
/// `context` (ADR-0023). The host (the PEP) classifies the raw command line into a stable
/// `kind`; the rule decides on that classification, never on the raw string, so the brittle
/// parsing stays host-side and the policy stays declarative. `None` for actions that carry no
/// such facts (a file write). The raw command itself never reaches the policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFacts {
    /// The host's classification of the command (e.g. `package_install`, `ephemeral_exec`,
    /// `pipe_to_shell`). Exposed to policy as `context.command.kind`.
    pub kind: String,
}

/// One normalized agent action: the unit the PDP decides on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAction {
    pub principal: AgentId,
    pub action: ActionKind,
    pub resource: ResourceId,
    pub session_id: SessionId,
    /// Monotonic index within the session.
    pub seq: u64,
    pub at: Timestamp,
    pub source: Provenance,
    /// Host-derived command facts for the Cedar `context`, set when the action is a shell
    /// command the PEP classified; `None` otherwise (the request carries an empty context).
    pub command: Option<CommandFacts>,
}
