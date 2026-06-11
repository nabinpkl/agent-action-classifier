//! Audit sink (TASKS #5): with `--audit-log`, the built `enforce` binary appends one
//! OPA/AAT-shaped JSON record per governed decision. Runs the binary black-box against a temp
//! log and asserts the record fields, including the measured latency and the null chain slot.

use std::io::Write as _;
use std::process::{Command, Stdio};

fn run_enforce_with_audit(payload: &str, audit_log: &str) -> i32 {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let plane = format!("{manifest}/../../corpus/asi05");
    let resource_map = format!("{manifest}/../../corpus/asi05/resource_map.json");

    let mut child = Command::new(env!("CARGO_BIN_EXE_enforce"))
        .args([
            "--plane",
            &plane,
            "--resource-map",
            &resource_map,
            "--agent-id",
            "agent-1",
            "--provider",
            "claude",
            "--audit-log",
            audit_log,
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
    child
        .wait_with_output()
        .expect("wait enforce")
        .status
        .code()
        .unwrap_or(-1)
}

#[test]
fn governed_decisions_append_records_ungoverned_calls_do_not() {
    let log = std::env::temp_dir().join(format!("aac-audit-{}.jsonl", std::process::id()));
    let log_path = log.to_str().expect("temp path is utf-8");
    let _ = std::fs::remove_file(&log);

    // A governed deny, then an ungoverned (out-of-scope) call: only the first is recorded.
    let deny_exit = run_enforce_with_audit(
        r#"{"tool_name":"Write","tool_input":{"file_path":"/x/.env"},"session_id":"s9"}"#,
        log_path,
    );
    let proceed_exit = run_enforce_with_audit(
        r#"{"tool_name":"Write","tool_input":{"file_path":"/x/src/main.rs"},"session_id":"s9"}"#,
        log_path,
    );
    assert_eq!(deny_exit, 2, "deny still exits 2 with auditing on");
    assert_eq!(proceed_exit, 0, "out-of-scope still proceeds");

    let contents = std::fs::read_to_string(&log).expect("audit log written");
    let _ = std::fs::remove_file(&log);

    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "only the governed decision is recorded, got: {contents}"
    );

    let record: serde_json::Value = serde_json::from_str(lines[0]).expect("valid json record");
    assert_eq!(record["verdict"], "deny");
    assert_eq!(record["gate_type"], "hard");
    assert_eq!(record["owasp"], "ASI05");
    assert_eq!(record["policy_id"], "deny-secret-write");
    assert_eq!(record["lane"], "deterministic");
    assert_eq!(record["request"]["resource"], "secrets");
    assert_eq!(record["request"]["action"], "write");
    assert_eq!(record["request"]["principal"], "agent-1");
    assert!(record["latency_ns"].as_u64().is_some(), "latency recorded");
    assert!(record["prev_hash"].is_null(), "chain slot null in v0");
}
