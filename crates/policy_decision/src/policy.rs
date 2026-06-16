//! The org policy (PAP): a Cedar schema, the policies, and the entity store they evaluate
//! against. The three are authored together by the central plane and validated as a unit.
//!
//! ADR-0017: the hand-built `Matcher`/`Rule` engine is gone. A rule is now a Cedar
//! policy carrying annotations the host reads to reconstruct our richer verdict:
//! `@id(...)`, `@owasp(...)`, and on permits `@outcome(...)` + `@lane(...)`. A Cedar
//! `forbid` *is* a hard deny. The precedence (deny-overrides, default-deny) is Cedar's;
//! the cascade interpretation lives in `evaluate`.
//!
//! ADR-0018: the schema is the contract. [`Policy::from_sources`] is the one construction
//! path: it validates the policy set against the schema under `Strict` mode and parses the
//! entity store against the schema, so a typo'd attribute or a policy referencing an
//! undeclared field fails loud at load instead of silently becoming a non-match.

use std::fmt;

use cedar_policy::{Entities, PolicySet, Schema, ValidationMode, Validator};

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

/// Why constructing a [`Policy`] from its sources failed. A custom boundary error so the
/// loader/binding can tell *which* artifact was malformed; the variant payload carries the
/// underlying Cedar/validation message for the operator.
#[derive(Debug)]
pub enum PolicyLoadError {
    /// The Cedar schema source did not parse.
    Schema(String),
    /// The Cedar policy source did not parse.
    Policies(String),
    /// The policies parsed but do not validate against the schema (e.g. a reference to an
    /// undeclared entity type or attribute).
    Validation(String),
    /// The entity store did not parse, or does not conform to the schema.
    Entities(String),
}

impl fmt::Display for PolicyLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyLoadError::Schema(e) => write!(f, "parsing Cedar schema: {e}"),
            PolicyLoadError::Policies(e) => write!(f, "parsing Cedar policy: {e}"),
            PolicyLoadError::Validation(e) => {
                write!(f, "policy does not validate against schema: {e}")
            }
            PolicyLoadError::Entities(e) => write!(f, "parsing entities against schema: {e}"),
        }
    }
}

impl std::error::Error for PolicyLoadError {}

/// The authored org policy: the Cedar schema, the policy set, and the entity store (data-
/// scope attributes; v1 is a flat principal, no org-graph inheritance — ADR-0025). All three are
/// supplied by the central plane (PAP); the loader/binding parses them at the edge via
/// [`Policy::from_sources`], the one validated construction path.
pub struct Policy {
    schema: Schema,
    policies: PolicySet,
    entities: Entities,
}

impl Policy {
    /// Parse and validate the three PAP artifacts into a `Policy`, failing loud on any
    /// drift: the policy set is validated against the schema under `Strict` mode, and the
    /// entity store is parsed against the schema. This is the only constructor, so an
    /// in-memory `Policy` is always schema-consistent.
    pub fn from_sources(
        schema_src: &str,
        policy_src: &str,
        entities_json: &str,
    ) -> Result<Policy, PolicyLoadError> {
        let schema: Schema = schema_src
            .parse()
            .map_err(|e| PolicyLoadError::Schema(format!("{e}")))?;
        let policies: PolicySet = policy_src
            .parse()
            .map_err(|e| PolicyLoadError::Policies(format!("{e}")))?;

        let result = Validator::new(schema.clone()).validate(&policies, ValidationMode::Strict);
        if !result.validation_passed() {
            let errors = result
                .validation_errors()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(PolicyLoadError::Validation(errors));
        }

        let entities = Entities::from_json_str(entities_json, Some(&schema))
            .map_err(|e| PolicyLoadError::Entities(format!("{e}")))?;

        Ok(Policy {
            schema,
            policies,
            entities,
        })
    }

    #[must_use]
    pub fn schema(&self) -> &Schema {
        &self.schema
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

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal valid trio: one scope attribute, one permit that reads it. Proves the
    // happy path, then each reject-test perturbs exactly one artifact.
    const SCHEMA: &str = r#"
        entity Agent;
        entity DataScope = { sensitivity: String };
        action read appliesTo { principal: [Agent], resource: [DataScope] };
    "#;
    const POLICY: &str = r#"
        @id("allow-public-read")
        permit(principal, action == Action::"read", resource)
        when { resource.sensitivity == "public" };
    "#;
    const ENTITIES: &str = r#"[
        { "uid": { "type": "Agent", "id": "a1" }, "attrs": {}, "parents": [] },
        { "uid": { "type": "DataScope", "id": "docs" }, "attrs": { "sensitivity": "public" }, "parents": [] }
    ]"#;

    #[test]
    fn valid_sources_load() {
        assert!(Policy::from_sources(SCHEMA, POLICY, ENTITIES).is_ok());
    }

    #[test]
    fn policy_referencing_undeclared_attribute_fails_validation() {
        // `resource.classification` is not in the schema: Strict validation must reject it
        // rather than let it silently never match.
        let bad_policy = r#"
            @id("typo")
            permit(principal, action == Action::"read", resource)
            when { resource.classification == "public" };
        "#;
        assert!(matches!(
            Policy::from_sources(SCHEMA, bad_policy, ENTITIES),
            Err(PolicyLoadError::Validation(_))
        ));
    }

    #[test]
    fn entity_with_misspelled_attribute_fails_schema_check() {
        // `sensitvity` (typo) is not the schema's `sensitivity`: the schema-checked entity
        // load must reject it instead of producing a scope that matches nothing.
        let bad_entities = r#"[
            { "uid": { "type": "DataScope", "id": "docs" }, "attrs": { "sensitvity": "public" }, "parents": [] }
        ]"#;
        assert!(matches!(
            Policy::from_sources(SCHEMA, POLICY, bad_entities),
            Err(PolicyLoadError::Entities(_))
        ));
    }

    #[test]
    fn malformed_schema_fails() {
        assert!(matches!(
            Policy::from_sources("entity Agent = { not valid", POLICY, ENTITIES),
            Err(PolicyLoadError::Schema(_))
        ));
    }
}
