//! Policy lint (ADR-0023, the "Cedar analysis in CI" direction): every authored corpus plane
//! must validate against its schema with ZERO warnings, not just zero errors.
//!
//! The production load path (`Policy::from_sources`) fails only on validation *errors* (a type
//! mismatch, a reference to an undeclared entity/action). This CI gate is deliberately stricter:
//! it also fails on validation *warnings*, which Cedar emits for policies that type-check but can
//! never fire (an impossible condition, a permit shadowed into vacuity) — an authoring bug that
//! would silently sit as a dead rule. Catching it at commit time is exactly the cheap slice of
//! "policy analysis" the crate gives us; full symbolic analysis (over-permissiveness, conflict)
//! needs the Lean toolchain and stays deferred (ADR-0023).
//!
//! Stricter in CI than at runtime is intentional: a benign warning must not brick a live plane
//! load, but it must not pass review either.

use cedar_policy::{PolicySet, Schema, ValidationMode, Validator};

#[test]
fn every_corpus_policy_validates_with_no_warnings() {
    let corpus_root = format!("{}/../../corpus", env!("CARGO_MANIFEST_DIR"));
    let mut planes = 0;

    for entry in std::fs::read_dir(&corpus_root).expect("read corpus dir") {
        let dir = entry.expect("corpus entry").path();
        let schema_path = dir.join("policy.cedarschema");
        let policy_path = dir.join("policy.cedar");
        // Only directories that declare a plane (schema + policy) are linted.
        if !schema_path.exists() || !policy_path.exists() {
            continue;
        }
        planes += 1;
        let name = dir
            .file_name()
            .expect("corpus dir has a name")
            .to_string_lossy()
            .into_owned();

        let schema: Schema = std::fs::read_to_string(&schema_path)
            .expect("read schema")
            .parse()
            .unwrap_or_else(|e| panic!("[{name}] schema parse failed: {e}"));
        let policies: PolicySet = std::fs::read_to_string(&policy_path)
            .expect("read policy")
            .parse()
            .unwrap_or_else(|e| panic!("[{name}] policy parse failed: {e}"));

        let result = Validator::new(schema).validate(&policies, ValidationMode::Strict);

        let errors: Vec<String> = result.validation_errors().map(|e| e.to_string()).collect();
        assert!(
            errors.is_empty(),
            "[{name}] validation errors: {}",
            errors.join("; ")
        );

        let warnings: Vec<String> = result
            .validation_warnings()
            .map(|w| w.to_string())
            .collect();
        assert!(
            warnings.is_empty(),
            "[{name}] validation warnings (a dead or impossible policy?): {}",
            warnings.join("; ")
        );
    }

    assert!(
        planes >= 2,
        "expected at least the asi05 + org_graph planes, found {planes}"
    );
}
