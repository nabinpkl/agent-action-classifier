//! Org-graph conformance (ADR-0020): replay the `corpus/org_graph/` spec through `decide`
//! and assert the four conformance keys at exact-match. This corpus exercises the principal
//! hierarchy — cascade (a policy on the org reaches a deep agent), sub-node override (a team
//! forbid beats an org permit), RBAC (role membership grants access), team-scoped approval,
//! and an org-wide hard deny — proving Cedar resolves `in` so the engine needs no change.
//! Same black-box runner (`corpus::check`) as every corpus; only ever calls `decide`.

mod corpus;

use corpus::{check, load_corpus};

#[test]
fn org_graph_conformance_corpus() {
    let corpus = load_corpus("org_graph").expect("load the org_graph corpus");
    let failures = check(&corpus);

    assert!(
        failures.is_empty(),
        "{} of {} org_graph cases failed:\n{}",
        failures.len(),
        corpus.cases.len(),
        failures.join("\n"),
    );
    eprintln!("org_graph conformance: {} cases passed", corpus.cases.len());
}
