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
/// attributes (sensitivity, pii, later org-graph parents) live in the entity store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceId(pub String);

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
}
