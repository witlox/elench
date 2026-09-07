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

// --- build-provenance.feature: --artifact flag + digest selection ---

/// Compute the SHA-256 hex digest of a byte slice, matching
/// `elench_store::Oid::from_blob_data`.
fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(data);
    format!("{hash:x}")
}

const TREE_OID: &str = "abc123def456789abc123def456789abc123def456789abc123def456789abcd";

#[test]
fn scenario_cli_build_digest_is_artifact_file_when_present() {
    let artifact = std::env::temp_dir().join(format!(
        "elench_build_art_{}.bin",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let content = b"this is the real build artifact, not stdout";
    std::fs::write(&artifact, content).unwrap();
    let expected = sha256_hex(content);

    let (stdout, _, code) = elench(&[
        "--harness",
        "build",
        TREE_OID,
        "--artifact",
        artifact.to_str().unwrap(),
        "--",
        "true",
    ]);
    let _ = std::fs::remove_file(&artifact);
    assert_eq!(code, 0, "stdout was: {stdout}");
    assert!(
        stdout.contains(&format!("digest:    {expected}")),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("(artifact") || stdout.contains("artifact:"),
        "stdout should name the artifact source: {stdout}"
    );
}

#[test]
fn scenario_cli_build_digest_falls_back_to_stdout() {
    // `echo hello` writes "hello\n" to stdout; SHA-256 of that is the digest.
    let (stdout, _, code) = elench(&["--harness", "build", TREE_OID, "--", "echo", "hello"]);
    assert_eq!(code, 0, "stdout was: {stdout}");
    let expected = sha256_hex(b"hello\n");
    assert!(
        stdout.contains(&format!("digest:    {expected}")),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains("source:    stdout"),
        "stdout should indicate the digest source: {stdout}"
    );
}

#[test]
fn scenario_cli_build_missing_artifact_rejected() {
    let missing = std::env::temp_dir().join(format!(
        "elench_build_missing_{}.bin",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(!missing.exists());
    let (_, stderr, code) = elench(&[
        "build",
        TREE_OID,
        "--artifact",
        missing.to_str().unwrap(),
        "--",
        "true",
    ]);
    assert_eq!(code, 1);
    assert!(stderr.contains("artifact"), "stderr: {stderr}");
    assert!(
        stderr.contains("not found") || stderr.contains("no such file") || stderr.contains("read"),
        "stderr should name the missing file: {stderr}"
    );
}

#[test]
fn scenario_cli_build_artifact_parsed_before_double_dash() {
    // `echo --artifact foo` must receive "--artifact foo" as its own args;
    // elench's --artifact is the one before the first --.
    let artifact = std::env::temp_dir().join(format!(
        "elench_build_dash_{}.bin",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let content = b"real artifact for dash test";
    std::fs::write(&artifact, content).unwrap();
    let expected = sha256_hex(content);

    let (stdout, _, code) = elench(&[
        "--harness",
        "build",
        TREE_OID,
        "--artifact",
        artifact.to_str().unwrap(),
        "--",
        "echo",
        "--artifact",
        "foo",
    ]);
    let _ = std::fs::remove_file(&artifact);
    assert_eq!(code, 0, "stdout was: {stdout}");
    assert!(
        stdout.contains(&format!("digest:    {expected}")),
        "digest should be the real artifact, not echo's output: {stdout}"
    );
}

// --- git init CLI ---

#[test]
fn scenario_cli_git_init_no_args_exits_1() {
    let (_, stderr, code) = elench(&["git", "init"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires <output_path> <claims.json>"));
}

#[test]
fn scenario_cli_git_init_missing_claims_file_exits_1() {
    let (_, stderr, code) = elench(&[
        "git",
        "init",
        "/tmp/opencode/elench-nonexistent-output",
        "/tmp/opencode/elench-nonexistent-claims.json",
    ]);
    assert_eq!(code, 1);
    assert!(stderr.contains("claims file not found"));
}

#[test]
fn scenario_cli_git_init_empty_claims_exits_1() {
    let path = std::env::temp_dir().join(format!(
        "elench_git_init_empty_{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, "[]").unwrap();
    let (_, stderr, code) = elench(&[
        "git",
        "init",
        "/tmp/opencode/elench-git-init-output",
        path.to_str().unwrap(),
    ]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 1);
    assert!(stderr.contains("empty claim log"));
}

// --- reconcile CLI ---

#[test]
fn scenario_cli_reconcile_no_args_exits_1() {
    let (_, stderr, code) = elench(&["reconcile"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a tree OID"));
}

#[test]
fn scenario_cli_reconcile_missing_claims_file_exits_1() {
    let (_, stderr, code) = elench(&[
        "reconcile",
        "abc123def456789abc123def456789abc123def456789abc123def456789abcd",
    ]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a claims file"));
}

#[test]
fn scenario_cli_reconcile_empty_log() {
    let path = std::env::temp_dir().join(format!(
        "elench_reconcile_empty_{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, "[]").unwrap();
    let (stdout, _, code) = elench(&[
        "reconcile",
        "abc123def456789abc123def456789abc123def456789abc123def456789abcd",
        path.to_str().unwrap(),
    ]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 0);
    assert!(stdout.contains("intact:   0 claims"));
    assert!(stdout.contains("drifted:  0 claims"));
    assert!(stdout.contains("no drift"));
}

#[test]
fn scenario_cli_reconcile_drifted_claims() {
    // Claims with anchors that have no path/symbol/digest → all fail.
    let claims = r#"{
        "id": "cl_0000000000000000000000000000000000000000000000000000000000000080",
        "kind": "assertion",
        "target": [],
        "assertion": {"form": "annotation", "text": "test"},
        "origin": {"kind": "agent-asserted", "producer": {"id": "test"}},
        "anchor": {"tree": "abc123def456789abc123def456789abc123def456789abc123def456789abcd", "strategy": "multi", "path": null, "range": null, "symbol": null, "content_digest": null},
        "timestamp": 1700000000,
        "evidence": [],
        "depends_on": []
    }"#;
    let path = std::env::temp_dir().join(format!(
        "elench_reconcile_drift_{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, claims).unwrap();
    let (stdout, _, code) = elench(&[
        "reconcile",
        "abc123def456789abc123def456789abc123def456789abc123def456789abcd",
        path.to_str().unwrap(),
    ]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 0);
    assert!(stdout.contains("drifted:  1 claims"));
    assert!(stdout.contains("cl_0000000000000000000000000000000000000000000000000000000000000080"));
    assert!(stdout.contains("Failed"));
}

// --- conflicts CLI (fixed: same-anchor, different expression) ---

#[test]
fn scenario_cli_conflicts_no_args_exits_1() {
    let (_, stderr, code) = elench(&["conflicts"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a tree OID"));
}

#[test]
fn scenario_cli_conflicts_missing_claims_file_exits_1() {
    let (_, stderr, code) = elench(&[
        "conflicts",
        "abc123def456789abc123def456789abc123def456789abc123def456789abcd",
    ]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a claims file"));
}

#[test]
fn scenario_cli_conflicts_different_paths_no_conflict() {
    let claims = r#"[
  {"id":"cl_0000000000000000000000000000000000000000000000000000000000000090","kind":"assertion","target":[],"assertion":{"form":"predicate","expression":{"language":"elench-predicate-v1","source":"exists(\"a.txt\")"}},"origin":{"kind":"agent-asserted","producer":{"id":"test"}},"anchor":{"tree":"abc123def456789abc123def456789abc123def456789abc123def456789abcd","strategy":"multi","path":"src/lib.rs","range":[1,10]},"timestamp":1700000000,"evidence":[],"depends_on":[]},
  {"id":"cl_0000000000000000000000000000000000000000000000000000000000000091","kind":"assertion","target":[],"assertion":{"form":"predicate","expression":{"language":"elench-predicate-v1","source":"exists(\"b.txt\")"}},"origin":{"kind":"agent-asserted","producer":{"id":"test"}},"anchor":{"tree":"abc123def456789abc123def456789abc123def456789abc123def456789abcd","strategy":"multi","path":"src/parser.rs","range":[1,10]},"timestamp":1700000001,"evidence":[],"depends_on":[]}
]"#;
    let path = std::env::temp_dir().join(format!(
        "elench_conflicts_np_{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, claims).unwrap();
    let (stdout, _, code) = elench(&[
        "conflicts",
        "abc123def456789abc123def456789abc123def456789abc123def456789abcd",
        path.to_str().unwrap(),
    ]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 0);
    assert!(stdout.contains("conflicts:         0"));
    assert!(stdout.contains("no contradictions detected"));
}

#[test]
fn scenario_cli_conflicts_same_anchor_detected() {
    let claims = r#"[
  {"id":"cl_0000000000000000000000000000000000000000000000000000000000000092","kind":"assertion","target":[],"assertion":{"form":"predicate","expression":{"language":"elench-predicate-v1","source":"exists(\"a.txt\")"}},"origin":{"kind":"agent-asserted","producer":{"id":"test"}},"anchor":{"tree":"abc123def456789abc123def456789abc123def456789abc123def456789abcd","strategy":"multi","path":"src/lib.rs","range":[1,10]},"timestamp":1700000000,"evidence":[],"depends_on":[]},
  {"id":"cl_0000000000000000000000000000000000000000000000000000000000000093","kind":"assertion","target":[],"assertion":{"form":"predicate","expression":{"language":"elench-predicate-v1","source":"exists(\"b.txt\")"}},"origin":{"kind":"agent-asserted","producer":{"id":"test"}},"anchor":{"tree":"abc123def456789abc123def456789abc123def456789abc123def456789abcd","strategy":"multi","path":"src/lib.rs","range":[1,10]},"timestamp":1700000001,"evidence":[],"depends_on":[]}
]"#;
    let path = std::env::temp_dir().join(format!(
        "elench_conflicts_sa_{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, claims).unwrap();
    let (stdout, _, code) = elench(&[
        "conflicts",
        "abc123def456789abc123def456789abc123def456789abc123def456789abcd",
        path.to_str().unwrap(),
    ]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 0);
    assert!(stdout.contains("conflicts:         1"));
    assert!(stdout.contains("src/lib.rs:1-10"));
}

// --- CLI tests for previously untested commands ---

fn write_temp_json(content: &str, prefix: &str) -> (std::path::PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "{prefix}_{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, content).unwrap();
    let path_str = path.to_str().unwrap().to_string();
    (path, path_str)
}

#[test]
fn scenario_cli_emit_no_args_exits_1() {
    let (_, stderr, code) = elench(&["emit"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a claim JSON file"));
}

#[test]
fn scenario_cli_emit_happy_path() {
    let claim = r#"{"id":"cl_0000000000000000000000000000000000000000000000000000000000000050","kind":"assertion","target":[],"assertion":{"form":"annotation","text":"test"},"origin":{"kind":"agent-asserted","producer":{"id":"test"}},"anchor":{"tree":"abc123def456789abc123def456789abc123def456789abc123def456789abcd","strategy":"multi","path":"src/main.rs","range":[1,10]},"timestamp":1700000000,"evidence":[],"depends_on":[]}"#;
    let (path, path_str) = write_temp_json(claim, "elench_emit_happy");
    let (stdout, _, code) = elench(&["emit", &path_str]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 0, "stdout was: {stdout}");
    assert!(stdout.contains("claim emitted:"));
    assert!(stdout.contains("kind:     assertion"));
}

#[test]
fn scenario_cli_verify_no_args_exits_1() {
    let (_, stderr, code) = elench(&["verify"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires an envelope JSON file"));
}

#[test]
fn scenario_cli_verify_invalid_json_exits_1() {
    let (path, path_str) = write_temp_json("not json", "elench_verify_bad");
    let (_, stderr, code) = elench(&["verify", &path_str]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 1);
    assert!(stderr.contains("invalid envelope JSON"));
}

#[test]
fn scenario_cli_log_no_args_exits_1() {
    let (_, stderr, code) = elench(&["log"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a claims file"));
}

#[test]
fn scenario_cli_log_happy_path() {
    let claims = r#"[{"id":"cl_0000000000000000000000000000000000000000000000000000000000000051","kind":"assertion","target":[],"assertion":{"form":"annotation","text":"a"},"origin":{"kind":"agent-asserted","producer":{"id":"t"}},"anchor":{"tree":"t","strategy":"multi","path":null,"range":null,"symbol":null,"content_digest":null},"timestamp":1700000000,"evidence":[],"depends_on":[]}]"#;
    let (path, path_str) = write_temp_json(claims, "elench_log_happy");
    let (stdout, _, code) = elench(&["log", &path_str]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 0);
    assert!(stdout.contains("log statistics:"));
    assert!(stdout.contains("total:            1"));
}

#[test]
fn scenario_cli_review_no_args_exits_1() {
    let (_, stderr, code) = elench(&["review"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a tree OID"));
}

#[test]
fn scenario_cli_review_missing_claims_file_exits_1() {
    let (_, stderr, code) = elench(&["review", TREE_OID]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a claims file"));
}

#[test]
fn scenario_cli_review_no_claims_for_tree() {
    let claims = "[]";
    let (path, path_str) = write_temp_json(claims, "elench_review_empty");
    let (stdout, _, code) = elench(&["review", TREE_OID, &path_str]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 0);
    assert!(stdout.contains("no claims for tree"));
}

#[test]
fn scenario_cli_accept_no_args_exits_1() {
    let (_, stderr, code) = elench(&["accept"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a tree OID"));
}

#[test]
fn scenario_cli_accept_missing_claims_file_exits_1() {
    let (_, stderr, code) = elench(&["accept", TREE_OID]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a claims file"));
}

#[test]
fn scenario_cli_accept_no_claims_named_exits_1() {
    let claims = "[]";
    let (path, path_str) = write_temp_json(claims, "elench_accept_noclaims");
    let (_, stderr, code) = elench(&["accept", TREE_OID, &path_str]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 1);
    assert!(stderr.contains("no claims named"));
}

#[test]
fn scenario_cli_compact_no_args_exits_1() {
    let (_, stderr, code) = elench(&["compact"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a claims file"));
}

#[test]
fn scenario_cli_compact_empty_log() {
    let (path, path_str) = write_temp_json("[]", "elench_compact_empty");
    let (stdout, stderr, code) = elench(&["compact", &path_str]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("empty claim log") || stderr.contains("empty claim log"),
        "stdout: {stdout}, stderr: {stderr}"
    );
}

#[test]
fn scenario_cli_compact_happy_path() {
    let claims = r#"[{"id":"cl_0000000000000000000000000000000000000000000000000000000000000052","kind":"assertion","target":[],"assertion":{"form":"annotation","text":"a"},"origin":{"kind":"agent-asserted","producer":{"id":"t"}},"anchor":{"tree":"t","strategy":"multi","path":null,"range":null,"symbol":null,"content_digest":null},"timestamp":1700000000,"evidence":[],"depends_on":[]}]"#;
    let (path, path_str) = write_temp_json(claims, "elench_compact_happy");
    let (stdout, _, code) = elench(&["compact", &path_str, "--before", "9999999999"]);
    let _ = std::fs::remove_file(&path);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("compaction report:") || stdout.contains("cut-off"),
        "stdout: {stdout}"
    );
}

#[test]
fn scenario_cli_artifact_no_args_exits_1() {
    let (_, stderr, code) = elench(&["artifact"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a subcommand"));
}

#[test]
fn scenario_cli_artifact_create_happy_path() {
    let (stdout, _, code) = elench(&[
        "artifact",
        "create",
        TREE_OID,
        "test-policy",
        "abc123def456",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("\"version\""));
    assert!(stdout.contains("\"tree\""));
    assert!(stdout.contains("\"policy\""));
    assert!(stdout.contains("\"digest\""));
}

#[test]
fn scenario_cli_artifact_create_too_few_args_exits_1() {
    let (_, stderr, code) = elench(&["artifact", "create", TREE_OID]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires <tree> <policy> <digest>"));
}

#[test]
fn scenario_cli_artifact_unknown_subcommand_exits_1() {
    let (_, stderr, code) = elench(&["artifact", "bogus"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("unknown subcommand"));
}

// --- Error branches for partially tested commands ---

#[test]
fn scenario_cli_status_no_args_exits_1() {
    let (_, stderr, code) = elench(&["status"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a claim ID"));
}

#[test]
fn scenario_cli_gate_no_args_exits_1() {
    let (_, stderr, code) = elench(&["gate"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a tree OID"));
}

#[test]
fn scenario_cli_blast_no_args_exits_1() {
    let (_, stderr, code) = elench(&["blast"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a claim ID"));
}

#[test]
fn scenario_cli_blast_invalid_claim_id_exits_1() {
    let (_, stderr, code) = elench(&["blast", "invalid_id"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("invalid claim ID"));
}

#[test]
fn scenario_cli_git_no_args_exits_1() {
    let (_, stderr, code) = elench(&["git"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a claim log file"));
}

#[test]
fn scenario_cli_store_no_args_exits_1() {
    let (_, stderr, code) = elench(&["store"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a subcommand"));
}

#[test]
fn scenario_cli_store_blob_no_file_exits_1() {
    let (_, stderr, code) = elench(&["store", "blob"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a file path"));
}

#[test]
fn scenario_cli_store_tree_no_files_exits_1() {
    let (_, stderr, code) = elench(&["store", "tree"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires at least one file path"));
}

#[test]
fn scenario_cli_build_no_args_exits_1() {
    let (_, stderr, code) = elench(&["build"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("requires a command and tree OID"));
}

#[test]
fn scenario_cli_build_empty_command_exits_1() {
    let (_, stderr, code) = elench(&["--harness", "build", TREE_OID, "--"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("empty command"));
}

#[test]
fn scenario_cli_build_execution_failure_exits_1() {
    let (_, stderr, code) = elench(&["--harness", "build", TREE_OID, "--", "/nonexistent/binary"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("failed to execute"));
}

#[test]
fn scenario_cli_build_stderr_output() {
    let (stdout, _, code) = elench(&[
        "--harness",
        "build",
        TREE_OID,
        "--",
        "sh",
        "-c",
        "echo err >&2",
    ]);
    assert_eq!(code, 0);
    assert!(stdout.contains("stderr (first 500 chars):"));
    assert!(stdout.contains("err"));
}
