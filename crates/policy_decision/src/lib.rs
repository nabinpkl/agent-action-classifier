//! `policy_decision`: the agent-action governance PDP (Policy Decision Point) over Cedar.
//!
//! One public entry point, [`decide`], maps a [`canonical_action::CanonicalAction`]
//! plus an org [`policy::Policy`] plus a [`context::Context`] to a
//! [`decision::Decision`]. Cedar is the deterministic decision engine (ADR-0017); this
//! crate is the thin orchestrator that builds the Cedar request and interprets Cedar's
//! result into the four-verdict cascade. No I/O, no async, deterministic. The escalate
//! lane is *returned as a verdict* for the host to run, never executed here, which is
//! what keeps the core conformance-testable and polyglot-embeddable.
//!
//! Contracts: see `SPEC.md`. Precedence authority: see `docs/adr/0005`, `docs/adr/0017`.

pub mod canonical_action;
pub mod context;
pub mod decision;
pub mod evaluate;
pub mod policy;

pub use evaluate::decide;
