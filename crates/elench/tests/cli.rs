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
    // Hermetic: create a truly empty claim log in a temp file rather than
    // depending on a fixed path that may not exist across machines.
    let path = std::env::temp_dir().join(format!(
        "elench_empty_claims_{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, "[]").unwrap();
    let (stdout, _, code) = elench(&["git", path.to_str().unwrap()]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 0);
    assert!(stdout.contains("nothing to project") || stdout.contains("empty"));
}

// --- store-backend.feature: --store flag selection ---

#[test]
fn scenario_cli_store_default_reports_memory() {
    let (stdout, _, code) = elench(&[
        "store",
        "blob",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"),
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("store: memory"));
}

#[test]
fn scenario_cli_store_explicit_memory_before_command() {
    let (stdout, _, code) = elench(&[
        "--store",
        "memory",
        "store",
        "blob",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"),
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("store: memory"));
}

#[test]
fn scenario_cli_store_memory_after_command() {
    let (stdout, _, code) = elench(&[
        "store",
        "blob",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"),
        "--store",
        "memory",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("store: memory"));
}

#[test]
fn scenario_cli_store_missing_value_rejected() {
    let (_, stderr, code) = elench(&["--store"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a value"));
}

#[test]
fn scenario_cli_store_unknown_backend_rejected() {
    let (_, stderr, code) = elench(&["--store", "redis", "store", "blob", "x"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("unknown backend"));
    assert!(stderr.contains("redis"));
}

#[cfg(not(feature = "fjall-backend"))]
#[test]
fn scenario_cli_store_fjall_rejected_without_feature() {
    let (_, stderr, code) = elench(&[
        "--store",
        "fjall",
        "/tmp/opencode/elench_nofeature",
        "store",
        "blob",
        "x",
    ]);
    assert_eq!(code, 1);
    assert!(stderr.contains("fjall-backend feature"));
}

#[cfg(feature = "fjall-backend")]
#[test]
fn scenario_cli_store_fjall_persists_on_disk() {
    let dir = std::env::temp_dir().join(format!(
        "elench_fjall_cli_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let (stdout, _, code) = elench(&[
        "--store",
        "fjall",
        dir.to_str().unwrap(),
        "store",
        "blob",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"),
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("store: fjall"));
    assert!(dir.exists(), "fjall store dir should be materialized");
    let _ = std::fs::remove_dir_all(&dir);
}
