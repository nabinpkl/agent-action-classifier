//! The fixed canonical action schema.
//!
//! The single most load-bearing decision in the design: a **closed set of operation
//! variants**, not dynamic JSON. The fixed shape buys zero-allocation matching, a cheap
//! FFI boundary, and stable bindings at once. Adding a new action kind is a deliberate
//! code change, by design (the compiler then forces every match to handle it).

/// Which agent took the action.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

/// Trajectory id. Carried in v0 for the future stateful lane; not yet matched on.
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

/// The closed variant set. v0 covers the ASI05 (unsafe code execution) surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    ShellExec { command: String, cwd: String },
    FileWrite { path: String, byte_len: u64 },
    NetworkFetch { url: String },
}

/// One normalized agent action: the unit the PDP decides on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAction {
    pub agent_id: AgentId,
    pub session_id: SessionId,
    /// Monotonic index within the session.
    pub seq: u64,
    pub at: Timestamp,
    pub source: Provenance,
    pub operation: Operation,
}
