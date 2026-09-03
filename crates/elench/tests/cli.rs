use std::process::Command;

fn elench(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_elench"))
        .args(args)
        .output()
        .expect("failed to run elench");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

#[test]
fn scenario_cli_no_args_exits_1() {
    let (_, _, code) = elench(&[]);
    assert_eq!(code, 1);
}

#[test]
fn scenario_cli_help_exits_0() {
    let (stdout, _, code) = elench(&["help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("USAGE"));
    assert!(stdout.contains("COMMANDS"));
}

#[test]
fn scenario_cli_version_exits_0() {
    let (stdout, _, code) = elench(&["version"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("elench"));
}

#[test]
fn scenario_cli_unknown_command_exits_1() {
    let (_, stderr, code) = elench(&["nonexistent"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("unknown command"));
}

#[test]
fn scenario_cli_status_valid_claim_id() {
    let (stdout, _, code) = elench(&[
        "status",
        "cl_0000000000000000000000000000000000000000000000000000000000000001",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Unevaluated"));
}

#[test]
fn scenario_cli_status_invalid_claim_id_exits_1() {
    let (_, stderr, code) = elench(&["status", "invalid"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("invalid claim ID"));
}

#[test]
fn scenario_cli_gate_empty_log_passes() {
    let (stdout, _, code) = elench(&["gate", "abc123def456"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Pass"));
}

#[test]
fn scenario_cli_blast_empty_log() {
    let (stdout, _, code) = elench(&[
        "blast",
        "cl_0000000000000000000000000000000000000000000000000000000000000001",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("blast radius: 0 claims"));
}

#[test]
fn scenario_cli_store_blob_correct_oid() {
    let (stdout, _, code) = elench(&[
        "store",
        "blob",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"),
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("blob: "));
    assert!(stdout.contains("SHA-256"));
    let oid_line = stdout.lines().find(|l| l.starts_with("blob: ")).unwrap();
    let oid = oid_line.strip_prefix("blob: ").unwrap();
    assert_eq!(oid.len(), 64);
    assert!(oid.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn scenario_cli_store_blob_missing_file_exits_1() {
    let (_, stderr, code) = elench(&["store", "blob", "/tmp/opencode/elench_nonexistent.txt"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("failed to read"));
}

#[test]
fn scenario_cli_store_tree_multiple_files() {
    let (stdout, _, code) = elench(&[
        "store",
        "tree",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../Makefile"),
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("tree: "));
    assert!(stdout.contains("entries: 2"));
    assert!(stdout.contains("SHA-256"));
}

#[test]
fn scenario_cli_store_unknown_subcommand_exits_1() {
    let (_, stderr, code) = elench(&["store", "unknown"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("unknown subcommand"));
}

#[test]
fn scenario_cli_git_empty_log() {
    let (stdout, _, code) = elench(&["git", "/tmp/opencode/empty-claims.json"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("nothing to project") || stdout.contains("empty"));
}
