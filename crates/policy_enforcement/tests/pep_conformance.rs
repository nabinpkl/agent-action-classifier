//! PEP conformance: run the built `enforce` binary black-box over a fixture of provider
//! payloads and assert the verdict it returns (exit code + output). This is the host-layer
//! analogue of the PDP conformance corpora — it proves the whole wire (payload -> normalize ->
//! decide -> hook response) end to end against the real `corpus/asi05` plane, not just the
//! pure pieces the unit tests cover. Spawns the binary via `CARGO_BIN_EXE_enforce`; no extra
//! dependency beyond serde_json for the fixture.

use std::io::Write as _;
use std::process::{Command, Stdio};

/// Run the built binary with `payload` on stdin against the asi05 plane; return (exit, out, err).
fn run_enforce(payload: &str) -> (i32, String, String) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let plane = format!("{manifest}/../../corpus/asi05");
    let resource_map = format!("{manifest}/../../corpus/asi05/resource_map.json");
    let command_signatures = format!("{manifest}/../../corpus/asi05/command_signatures.json");

    let mut child = Command::new(env!("CARGO_BIN_EXE_enforce"))
        .args([
            "--plane",
            &plane,
            "--resource-map",
            &resource_map,
            "--command-signatures",
            &command_signatures,
            "--agent-id",
            "agent-1",
            "--provider",
            "claude",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn enforce");

    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(payload.as_bytes())
        .expect("write stdin");
    let output = child.wait_with_output().expect("wait enforce");

    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn pep_conformance_corpus() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let cases_json = std::fs::read_to_string(format!("{manifest}/tests/pep_cases.json"))
        .expect("read pep_cases.json");
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(&cases_json).expect("parse pep_cases.json");

    let mut failures = Vec::new();
    for case in &cases {
        let name = case["name"].as_str().unwrap_or("?");
        let payload = match case.get("payload_raw").and_then(|v| v.as_str()) {
            Some(raw) => raw.to_string(),
            None => serde_json::to_string(&case["payload"]).expect("serialize payload"),
        };
        let (code, stdout, stderr) = run_enforce(&payload);
        let expect = &case["expect"];

        if let Some(want) = expect["exit"].as_i64()
            && code as i64 != want
        {
            failures.push(format!(
                "{name}: exit {code} != expected {want} (stderr: {stderr})"
            ));
        }
        if let Some(sub) = expect["stderr_contains"].as_str()
            && !stderr.contains(sub)
        {
            failures.push(format!("{name}: stderr {stderr:?} missing {sub:?}"));
        }
        if let Some(sub) = expect["stdout_contains"].as_str()
            && !stdout.contains(sub)
        {
            failures.push(format!("{name}: stdout {stdout:?} missing {sub:?}"));
        }
        if expect["stdout_empty"].as_bool() == Some(true) && !stdout.trim().is_empty() {
            failures.push(format!("{name}: stdout expected empty, got {stdout:?}"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} PEP cases failed:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
    eprintln!("PEP conformance: {} cases passed", cases.len());
}
