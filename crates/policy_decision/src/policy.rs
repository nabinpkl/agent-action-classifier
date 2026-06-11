//! The org policy (PAP): Cedar policies plus the entity store they evaluate against.
//!
//! ADR-0017: the hand-built `Matcher`/`Rule` engine is gone. A rule is now a Cedar
//! policy carrying annotations the host reads to reconstruct our richer verdict:
//! `@id(...)`, `@owasp(...)`, and on permits `@outcome(...)` + `@lane(...)`. A Cedar
//! `forbid` *is* a hard deny. The precedence (deny-overrides, default-deny) is Cedar's;
//! the cascade interpretation lives in `evaluate`.

use cedar_policy::{Entities, PolicySet};

/// OWASP Agentic clause, e.g. `ASI05` (read from a policy's `@owasp` annotation). The
/// organizing/audit layer, not the decision logic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwaspClause(pub String);

/// Stable policy id (read from a policy's `@id` annotation); appears in the decision
/// record for audit. Distinct from Cedar's internal auto-assigned `PolicyId` handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolicyId(pub String);

/// Which evaluation lane resolves a rule (read from a permit's `@lane` annotation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    /// Structured predicate, resolved deterministically by Cedar.
    Deterministic,
    /// Routed to the host's LLM judge (the core only returns `Escalate`).
    Semantic,
}

impl Lane {
    #[must_use]
    pub fn parse(s: &str) -> Option<Lane> {
        match s {
            "deterministic" => Some(Lane::Deterministic),
            "semantic" => Some(Lane::Semantic),
            _ => None,
        }
    }
}

/// The authored org policy: the Cedar policy set plus the entity store (data-scope
/// attributes now; the org graph with inheritance lands in the next slice). Both are
/// supplied by the central plane (PAP); the loader/binding parses them at the edge.
pub struct Policy {
    policies: PolicySet,
    entities: Entities,
}

impl Policy {
    #[must_use]
    pub fn new(policies: PolicySet, entities: Entities) -> Self {
        Self { policies, entities }
    }

    #[must_use]
    pub fn policies(&self) -> &PolicySet {
        &self.policies
    }

    #[must_use]
    pub fn entities(&self) -> &Entities {
        &self.entities
    }
}
