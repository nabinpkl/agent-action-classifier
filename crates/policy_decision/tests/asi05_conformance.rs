//! ASI05 (Unsafe Code Execution) conformance: replay the external corpus through
//! `decide` and assert the four conformance keys (`verdict` / `gate_type` / `owasp` /
//! `policy_id`) at exact-match, per SPEC.md. The corpus under `corpus/asi05/` is the
//! drift-proof executable spec; this runner is black-box, only ever calling the public
//! `decide`. It reports every failing case at once (not just the first) and fails loud
//! if the corpus is empty.

mod corpus;

use policy_decision::decide;
use policy_decision::decision::Decision;

use corpus::{Asi05Corpus, Case, load_asi05};

#[test]
fn asi05_conformance_corpus() {
    let Asi05Corpus { policy, cases } = load_asi05().expect("load the ASI05 corpus");

    let failures: Vec<String> = cases
        .iter()
        .filter_map(|case| {
            let got = decide(&case.action, &policy, &case.context);
            mismatch(case, &got).map(|why| format!("[{}] {why}", case.name))
        })
        .collect();

    assert!(
        failures.is_empty(),
        "{} of {} ASI05 cases failed:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n"),
    );
    eprintln!("ASI05 conformance: {} cases passed", cases.len());
}

/// `None` if every conformance key matches; otherwise the joined differences.
fn mismatch(case: &Case, got: &Decision) -> Option<String> {
    let want = &case.expect;
    let got_owasp = got.owasp.as_ref().map(|clause| clause.0.clone());
    let got_policy_id = got.policy_id.as_ref().map(|id| id.0.clone());

    let mut diffs = Vec::new();
    if got.verdict != want.verdict {
        diffs.push(format!(
            "verdict: got {:?}, want {:?}",
            got.verdict, want.verdict
        ));
    }
    if got.gate_type != want.gate_type {
        diffs.push(format!(
            "gate_type: got {:?}, want {:?}",
            got.gate_type, want.gate_type
        ));
    }
    if got_owasp != want.owasp {
        diffs.push(format!("owasp: got {got_owasp:?}, want {:?}", want.owasp));
    }
    if got_policy_id != want.policy_id {
        diffs.push(format!(
            "policy_id: got {got_policy_id:?}, want {:?}",
            want.policy_id
        ));
    }

    if diffs.is_empty() {
        None
    } else {
        Some(diffs.join("; "))
    }
}
