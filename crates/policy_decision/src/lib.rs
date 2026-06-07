//! `policy_decision`: the pure agent-action governance PDP (Policy Decision Point).
//!
//! One public entry point, [`decide`], maps a [`canonical_action::CanonicalAction`]
//! plus an org [`policy::Policy`] plus a [`context::Context`] to a
//! [`decision::Decision`]. It is pure: no I/O, no async, deterministic. The escalate
//! lane is *returned as a verdict* for the host to run, never executed here, which is
//! what keeps the core conformance-testable and polyglot-embeddable.
//!
//! Contracts: see `SPEC.md`. Precedence authority: see `docs/adr/0005`.

pub mod canonical_action;
pub mod context;
pub mod decision;
pub mod evaluate;
pub mod policy;

pub use evaluate::decide;
