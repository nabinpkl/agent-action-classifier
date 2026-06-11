//! ASI05 (Unsafe Code Execution) conformance: replay the external `corpus/asi05/` spec
//! through `decide` and assert the four conformance keys (`verdict` / `gate_type` /
//! `owasp` / `policy_id`) at exact-match, per SPEC.md. The corpus is the drift-proof
//! executable spec; the runner (`corpus::check`) is black-box, only ever calling the
//! public `decide`. It reports every failing case at once and fails loud on an empty corpus.

mod corpus;

use corpus::{check, load_corpus};

#[test]
fn asi05_conformance_corpus() {
    let corpus = load_corpus("asi05").expect("load the ASI05 corpus");
    let failures = check(&corpus);

    assert!(
        failures.is_empty(),
        "{} of {} ASI05 cases failed:\n{}",
        failures.len(),
        corpus.cases.len(),
        failures.join("\n"),
    );
    eprintln!("ASI05 conformance: {} cases passed", corpus.cases.len());
}
