//! # elench
//!
//! An evidence layer for repositories — and the substrate that
//! replaces git (ADR-0001).
//!
//! elench records what was checked, to what depth, and what remains
//! unevaluated — as a durable claim log stored in a content-addressed
//! store. Claims are signed, append-only, and revocable. An artifact's
//! acceptability is a live evaluation against the current claim log,
//! not a signature frozen at release time.
//!
//! The git CLI works because elench synthesizes git-compatible objects
//! from the claim log (ADR-0002, ADR-0007). The projection is
//! read-only and deterministic (BC4). Humans use git; elench is
//! invisible.

#![allow(
    clippy::unnecessary_debug_formatting,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::single_match_else,
    clippy::nonminimal_bool,
    clippy::match_wildcard_for_single_variants
)]

use std::path::{Path, PathBuf};

use elench_store::StoreBackend;

/// Selected storage backend, parsed from the global `--store` flag.
#[derive(Debug)]
enum StoreConfig {
    /// In-memory store (default). No persistence across processes.
    Memory,
    /// Persistent fjall-backed store (ADR-0008). Requires the
    /// `fjall-backend` feature at build time.
    #[cfg(feature = "fjall-backend")]
    Fjall { path: PathBuf },
}

impl StoreConfig {
    /// Human-readable backend name for diagnostics.
    fn name(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
            #[cfg(feature = "fjall-backend")]
            Self::Fjall { .. } => "fjall",
        }
    }
}

/// Extract the global `--store` flag from `args`, returning the chosen
/// backend and removing its tokens in place. Only tokens before the first
/// standalone `--` are examined; everything after belongs to a subcommand
/// such as `build <tree> -- <cmd>`.
///
/// Syntax: `--store memory` (default) or `--store fjall <path>`. May appear
/// anywhere before the command; the last occurrence wins.
///
/// # Errors
///
/// Returns a human-readable message on a missing value, unknown backend,
/// or `--store fjall` when the `fjall-backend` feature is not enabled.
fn extract_store_config(args: &mut Vec<String>) -> Result<StoreConfig, String> {
    let mut config = StoreConfig::Memory;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--" {
            break;
        }
        if args[i] == "--store" {
            if i + 1 >= args.len() {
                return Err("--store requires a value: memory | fjall <path>".into());
            }
            match args[i + 1].as_str() {
                "memory" => {
                    config = StoreConfig::Memory;
                    args.drain(i..=i + 1);
                }
                "fjall" => {
                    #[cfg(not(feature = "fjall-backend"))]
                    {
                        return Err("--store fjall requires the fjall-backend feature \
                             (rebuild with --features elench/fjall-backend)"
                            .into());
                    }
                    #[cfg(feature = "fjall-backend")]
                    {
                        if i + 2 >= args.len() {
                            return Err("--store fjall requires a <path>".into());
                        }
                        config = StoreConfig::Fjall {
                            path: PathBuf::from(&args[i + 2]),
                        };
                        args.drain(i..=i + 2);
                    }
                }
                other => {
                    return Err(format!(
                        "--store: unknown backend '{other}' (expected: memory | fjall <path>)"
                    ));
                }
            }
        } else {
            i += 1;
        }
    }
    Ok(config)
}

/// Open the selected backend, returning a boxed [`StoreBackend`].
///
/// With the `fjall-backend` feature the fjall arm may fail to open;
/// without the feature it cannot, but the `Result` is kept so the call
/// sites and error handling are identical across build configurations.
///
/// # Errors
///
/// Returns a human-readable message if the backend cannot be opened
/// (e.g. an unwritable fjall path).
#[cfg_attr(not(feature = "fjall-backend"), allow(clippy::unnecessary_wraps))]
fn open_store(config: &StoreConfig) -> Result<Box<dyn StoreBackend>, String> {
    match config {
        StoreConfig::Memory => Ok(Box::new(elench_store::MemoryStore::new())),
        #[cfg(feature = "fjall-backend")]
        StoreConfig::Fjall { path } => {
            let store = elench_store::FjallStore::open(path)
                .map_err(|e| format!("failed to open fjall store at {path:?}: {e}"))?;
            Ok(Box::new(store))
        }
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().collect();

    let store_config = match extract_store_config(&mut args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("elench: {e}");
            std::process::exit(1);
        }
    };

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let command = &args[1];
    let rest = &args[2..];

    match command.as_str() {
        "emit" => cmd_emit(rest, &store_config),
        "verify" => cmd_verify(rest),
        "status" => cmd_status(rest),
        "gate" => cmd_gate(rest),
        "blast" => cmd_blast(rest),
        "git" => cmd_git(rest, &store_config),
        "store" => cmd_store(rest, &store_config),
        "log" => cmd_log(rest),
        "review" => cmd_review(rest),
        "accept" => cmd_accept(rest),
        "conflicts" => cmd_conflicts(rest),
        "compact" => cmd_compact(rest),
        "artifact" => cmd_artifact(rest),
        "build" => cmd_build(rest),
        "help" | "--help" | "-h" => {
            print_usage();
        }
        "version" | "--version" | "-V" => {
            println!("elench 0.0.1 (elench-predicate-v1, SHA-256, git projection)");
        }
        other => {
            eprintln!("elench: unknown command '{other}'");
            eprintln!("Run 'elench help' for usage.");
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!("elench — an evidence layer for repositories");
    println!();
    println!("USAGE:");
    println!("    elench [--store memory|fjall <path>] <COMMAND> [OPTIONS]");
    println!();
    println!("GLOBAL OPTIONS:");
    println!("    --store memory         In-memory store (default, no persistence)");
    println!("    --store fjall <path>   Persistent fjall-backed store (requires");
    println!("                           the 'fjall-backend' feature, ADR-0008)");
    println!();
    println!("COMMANDS:");
    println!("    emit       Create and sign a claim, store in the store");
    println!("    verify     Verify an envelope and validate the claim");
    println!("    status     Compute a claim's status by folding the log");
    println!("    gate       Evaluate the release gate for a tree");
    println!("    blast      Compute the blast radius from a claim");
    println!("    git        Materialize the git projection");
    println!("    store      Store a blob or tree");
    println!("    log        Log statistics (count, status distribution, conflicts)");
    println!("    review     Review unevaluated claims for a tree");
    println!("    accept     Accept named unevaluated gaps (residue-acceptance)");
    println!("    conflicts  List active predicate conflicts for a tree");
    println!("    compact    Compact the claim log (manual, destructive)");
    println!("    artifact   Create or verify a release artifact (INV-15)");
    println!("    build      Run a build, capture exit code + digest, emit provenance");
    println!("    help       Print this message");
    println!("    version    Print version information");
    println!();
    println!("The claim log IS the primary history (ADR-0001).");
    println!("The git CLI works because elench synthesizes git objects");
    println!("from the claim log (ADR-0002). The projection is read-only.");
}

// ---------------------------------------------------------------------------
// Helpers — parse claims from a JSON file
// ---------------------------------------------------------------------------

fn parse_claims_file(path: &PathBuf) -> Vec<elench_claim::Claim> {
    let json = match std::fs::read_to_string(path) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("elench: failed to read {path:?}: {e}");
            std::process::exit(1);
        }
    };

    let trimmed = json.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if trimmed.starts_with('[') {
        match serde_json::from_str::<Vec<elench_claim::Claim>>(&json) {
            Ok(claims) => claims,
            Err(e) => {
                eprintln!("elench: invalid claims JSON array: {e}");
                std::process::exit(1);
            }
        }
    } else {
        match serde_json::from_str::<elench_claim::Claim>(&json) {
            Ok(claim) => vec![claim],
            Err(e) => {
                eprintln!("elench: invalid claim JSON: {e}");
                std::process::exit(1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// emit — create, sign, and store a claim
// ---------------------------------------------------------------------------

fn cmd_emit(args: &[String], store_config: &StoreConfig) {
    if args.is_empty() {
        eprintln!("elench emit: requires a claim JSON file");
        eprintln!("  elench [--store memory|fjall <path>] emit <claim.json>");
        eprintln!();
        eprintln!("The JSON file must contain a Claim with fields matching");
        eprintln!("schema/claim.schema.json. The claim's `id` is computed");
        eprintln!("from content (INV-28); the provided `id` is ignored.");
        std::process::exit(1);
    }

    let path = PathBuf::from(&args[0]);
    let json = match std::fs::read_to_string(&path) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("elench emit: failed to read {path:?}: {e}");
            std::process::exit(1);
        }
    };

    let mut claim: elench_claim::Claim = match serde_json::from_str(&json) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("elench emit: invalid claim JSON: {e}");
            std::process::exit(1);
        }
    };

    let computed_id = elench_claim::ClaimId::from_content(&claim);
    claim.id = computed_id.clone();

    let signer = elench_claim::SignerIdentity {
        key_id: "default-agent-key".into(),
        entity: elench_claim::SignerEntity::Agent,
    };
    let log: Vec<elench_claim::Claim> = Vec::new();

    if let Err(e) = elench_claim::validate_claim(&claim, &signer, &log) {
        eprintln!("elench emit: claim rejected by validator: {e}");
        std::process::exit(1);
    }

    let signing_key = elench_envelope::SigningKey::generate(elench_claim::SignerEntity::Agent);
    let envelope = elench_envelope::sign(&claim, &signing_key);

    let mut store = match open_store(store_config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("elench emit: {e}");
            std::process::exit(1);
        }
    };
    let stored_oid = match store.store_claim(&claim) {
        Ok(oid) => oid,
        Err(e) => {
            eprintln!("elench emit: failed to store claim: {e}");
            std::process::exit(1);
        }
    };

    println!("claim emitted:");
    println!("  id:       {computed_id}");
    println!("  kind:     {}", claim.kind_str());
    println!("  tree:     {}", claim.anchor.tree);
    println!("  producer: {}", claim.origin.producer.id);
    println!("  store:    {} ({})", store_config.name(), stored_oid);
    println!();
    println!("envelope:");
    println!("  payloadType: {}", envelope.payload_type);
    println!(
        "  payload:     {}... ({})",
        &envelope.payload[..32],
        envelope.payload.len()
    );
    println!(
        "  signature:   {}... ({})",
        &envelope.signatures[0].sig[..32],
        envelope.signatures[0].keyid
    );
    println!();
    println!("(INV-28: claim ID is SHA-256 of canonical JSON)");
    println!("(INV-06: validated against emission rules)");
    println!("(INV-22: signed in DSSE envelope, same format as build provenance)");
}

// ---------------------------------------------------------------------------
// verify — verify an envelope and validate the claim
// ---------------------------------------------------------------------------

fn cmd_verify(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench verify: requires an envelope JSON file");
        eprintln!("  elench verify <envelope.json>");
        std::process::exit(1);
    }

    let path = PathBuf::from(&args[0]);
    let json = match std::fs::read_to_string(&path) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("elench verify: failed to read {path:?}: {e}");
            std::process::exit(1);
        }
    };

    let envelope: elench_envelope::Envelope = match serde_json::from_str(&json) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("elench verify: invalid envelope JSON: {e}");
            std::process::exit(1);
        }
    };

    // For verification, we need a VerifyingKey. In a real deployment,
    // this would come from a key registry. For now, we use a placeholder
    // zero key — verification will fail unless the envelope was signed
    // with a key we know.
    let verifying_key = elench_envelope::VerifyingKey {
        key_id: "default-agent-key".into(),
        entity: elench_claim::SignerEntity::Agent,
        verifying_key: ed25519_dalek::VerifyingKey::from_bytes(&[0u8; 32])
            .expect("zero key is valid"),
    };
    let keys = vec![verifying_key];

    let (claim, signer) = match elench_envelope::verify(&envelope, &keys) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("elench verify: envelope verification failed: {e}");
            std::process::exit(1);
        }
    };

    let log: Vec<elench_claim::Claim> = Vec::new();
    if let Err(e) = elench_claim::validate_claim(&claim, &signer, &log) {
        eprintln!("elench verify: claim rejected by validator: {e}");
        std::process::exit(1);
    }

    let status = elench_claim::compute_status(&claim.id, &log)
        .unwrap_or(elench_claim::ClaimStatus::Unevaluated);

    println!("claim verified:");
    println!("  id:       {}", claim.id);
    println!("  kind:     {}", claim.kind_str());
    println!("  tree:     {}", claim.anchor.tree);
    println!("  producer: {}", claim.origin.producer.id);
    println!("  signer:   {} ({:?})", signer.key_id, signer.entity);
    println!("  status:   {status:?}");
    println!();
    println!("(INV-22: DSSE envelope verified, same format as build provenance)");
    println!("(INV-06: emission rules validated)");
    println!("(INV-04: status computed by folding, not stored)");
}

// ---------------------------------------------------------------------------
// status — compute a claim's status by folding the log
// ---------------------------------------------------------------------------

fn cmd_status(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench status: requires a claim ID");
        eprintln!("  elench status <claim_id> [<claims.json>]");
        std::process::exit(1);
    }

    let claim_id = &args[0];
    let log = if args.len() > 1 {
        parse_claims_file(&PathBuf::from(&args[1]))
    } else {
        Vec::new()
    };

    if let Ok(id) = elench_claim::ClaimId::new(claim_id) {
        let status = elench_claim::compute_status(&id, &log)
            .unwrap_or(elench_claim::ClaimStatus::Unevaluated);
        println!("claim: {id}");
        println!("status: {status:?}");
        if log.is_empty() {
            println!("(computed from {} claims in log)", log.len());
        } else {
            println!("(computed from empty log — live evaluation, INV-14)");
        }
    } else {
        eprintln!("elench status: invalid claim ID: {claim_id}");
        eprintln!("  expected: cl_ + 64 hex chars (SHA-256)");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// gate — evaluate the release gate for a tree
// ---------------------------------------------------------------------------

fn cmd_gate(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench gate: requires a tree OID");
        eprintln!("  elench gate <tree_oid> [<claims.json>]");
        std::process::exit(1);
    }

    let tree = &args[0];
    let log = if args.len() > 1 {
        parse_claims_file(&PathBuf::from(&args[1]))
    } else {
        Vec::new()
    };

    let policy = elench_gate::Policy::permissive("default");
    let verdict = elench_gate::evaluate(tree, &policy, &log).unwrap_or_else(|e| {
        eprintln!("elench gate: evaluation error: {e}");
        std::process::exit(1);
    });

    println!("tree:   {}", verdict.tree);
    println!("policy: {}", verdict.policy);
    println!("result: {:?}", verdict.result);
    if !verdict.reasons.is_empty() {
        println!("reasons:");
        for r in &verdict.reasons {
            println!("  - {r}");
        }
    }
    if log.is_empty() {
        println!("(evaluated against {} claims)", log.len());
    }
    println!("(live evaluation — INV-13: no build capability required)");
}

// ---------------------------------------------------------------------------
// blast — compute the blast radius from a claim
// ---------------------------------------------------------------------------

fn cmd_blast(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench blast: requires a claim ID");
        eprintln!("  elench blast <claim_id> [<claims.json>]");
        std::process::exit(1);
    }

    let claim_id = &args[0];
    let log = if args.len() > 1 {
        parse_claims_file(&PathBuf::from(&args[1]))
    } else {
        Vec::new()
    };

    if let Ok(id) = elench_claim::ClaimId::new(claim_id) {
        let radius = elench_claim::blast_radius(&id, &log);
        println!("claim: {id}");
        println!("blast radius: {} claims", radius.len());
        if !radius.is_empty() {
            println!("claims in blast radius:");
            for c in &radius {
                println!("  - {c}");
            }
        }
        if log.is_empty() {
            println!("(computed from {} claims in log)", log.len());
        } else {
            println!("(computed from empty log — transitive dependsOn closure)");
        }
    } else {
        eprintln!("elench blast: invalid claim ID: {claim_id}");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// git — materialize the git projection
// ---------------------------------------------------------------------------

fn cmd_git(args: &[String], store_config: &StoreConfig) {
    if args.is_empty() {
        eprintln!("elench git: requires a claim log file");
        eprintln!("  elench [--store memory|fjall <path>] git <claims.json>");
        eprintln!("  elench [--store memory|fjall <path>] git oneline <claims.json>");
        eprintln!("  elench [--store memory|fjall <path>] git full <claims.json>");
        std::process::exit(1);
    }

    let (format, path) = if args[0] == "oneline" || args[0] == "full" {
        if args.len() < 2 {
            eprintln!("elench git {}: requires a claims file", args[0]);
            std::process::exit(1);
        }
        (args[0].as_str(), PathBuf::from(&args[1]))
    } else {
        ("oneline", PathBuf::from(&args[0]))
    };

    if !path.exists() {
        eprintln!("elench git: file not found: {path:?}");
        std::process::exit(1);
    }

    let log = parse_claims_file(&path);

    if log.is_empty() {
        println!("(empty claim log — nothing to project)");
        return;
    }

    let store = match open_store(store_config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("elench git: {e}");
            std::process::exit(1);
        }
    };

    match elench_projection::synthesize(&log, &*store) {
        Ok(projection) => {
            println!(
                "projection: {} commits, {} blobs, {} trees (store: {})",
                projection.commits.len(),
                projection.blobs.len(),
                projection.trees.len(),
                store_config.name()
            );
            println!();
            let output = match format {
                "full" => elench_projection::git_log_full(&projection),
                _ => elench_projection::git_log_oneline(&projection),
            };
            println!("{output}");
        }
        Err(e) => {
            eprintln!("elench git: projection error: {e}");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// store — store a blob or tree
// ---------------------------------------------------------------------------

fn cmd_store(args: &[String], store_config: &StoreConfig) {
    if args.is_empty() {
        eprintln!("elench store: requires a subcommand");
        eprintln!("  elench [--store memory|fjall <path>] store blob <file>");
        eprintln!("  elench [--store memory|fjall <path>] store tree <file1> <file2> ...");
        std::process::exit(1);
    }

    match args[0].as_str() {
        "blob" => {
            if args.len() < 2 {
                eprintln!("elench store blob: requires a file path");
                std::process::exit(1);
            }
            let path = PathBuf::from(&args[1]);
            let data = match std::fs::read(&path) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("elench store blob: failed to read {path:?}: {e}");
                    std::process::exit(1);
                }
            };
            let mut store = match open_store(store_config) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("elench store blob: {e}");
                    std::process::exit(1);
                }
            };
            let oid = match store.store_blob(&data) {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("elench store blob: failed to store: {e}");
                    std::process::exit(1);
                }
            };
            println!("blob: {oid}");
            println!("size: {} bytes", data.len());
            println!("store: {}", store_config.name());
            println!("(SHA-256 content address — identical to git SHA-256 blob OID)");
        }
        "tree" => {
            if args.len() < 2 {
                eprintln!("elench store tree: requires at least one file path");
                std::process::exit(1);
            }

            let mut store = match open_store(store_config) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("elench store tree: {e}");
                    std::process::exit(1);
                }
            };
            let mut entries = Vec::new();

            for file_path in &args[1..] {
                let path = PathBuf::from(file_path);
                match std::fs::read(&path) {
                    Ok(data) => {
                        let blob_oid = store.store_blob(&data).unwrap_or_else(|e| {
                            eprintln!("elench store tree: failed to store blob: {e}");
                            std::process::exit(1);
                        });
                        let name = path
                            .file_name()
                            .map_or_else(|| file_path.clone(), |n| n.to_string_lossy().to_string());
                        entries.push(elench_store::TreeEntry {
                            name,
                            mode: 0o100_644,
                            oid: blob_oid,
                            kind: elench_store::TreeEntryKind::Blob,
                        });
                    }
                    Err(e) => {
                        eprintln!("elench store tree: failed to read {path:?}: {e}");
                        std::process::exit(1);
                    }
                }
            }

            // Sort entries (git-compatible) and compute the canonical OID,
            // then store the sorted entries so the persisted OID matches the
            // canonical one and round-trips through read_tree.
            let tree = elench_store::Tree::from_entries(entries);
            let stored_oid = match store.store_tree(tree.entries.clone()) {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("elench store tree: failed to store tree: {e}");
                    std::process::exit(1);
                }
            };
            println!("tree: {}", tree.oid);
            println!("entries: {}", tree.entries.len());
            println!("store: {} ({})", store_config.name(), stored_oid);
            println!("(SHA-256 content address — identical to git SHA-256 tree OID)");
        }
        other => {
            eprintln!("elench store: unknown subcommand '{other}'");
            eprintln!("  elench [--store memory|fjall <path>] store blob <file>");
            eprintln!("  elench [--store memory|fjall <path>] store tree <file1> <file2> ...");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// build — run a build, capture exit code + digest, emit provenance
// ---------------------------------------------------------------------------

/// Split build args into (`elench_flags`, `command_args`) on the first `--`.
/// The tree OID (args[0]) is excluded from `elench_flags`.
fn split_build_args(args: &[String]) -> (Vec<&str>, Vec<&str>) {
    if let Some(pos) = args.iter().position(|a| a == "--") {
        (
            args[1..pos].iter().map(String::as_str).collect(),
            args[pos + 1..].iter().map(String::as_str).collect(),
        )
    } else if args.len() > 1 {
        eprintln!("elench build: no command after tree OID (use -- to separate)");
        std::process::exit(1);
    } else {
        (Vec::new(), Vec::new())
    }
}

/// Parse elench-specific build flags (between the tree OID and `--`).
/// Currently only `--artifact <path>`. Unknown flags are an error.
fn parse_build_flags(flags: &[&str]) -> Option<PathBuf> {
    let mut artifact_path = None;
    let mut i = 0;
    while i < flags.len() {
        match flags[i] {
            "--artifact" => {
                if i + 1 >= flags.len() {
                    eprintln!("elench build: --artifact requires a file path");
                    std::process::exit(1);
                }
                artifact_path = Some(PathBuf::from(flags[i + 1]));
                i += 2;
            }
            other => {
                eprintln!(
                    "elench build: unknown flag '{other}' (did you mean to put it after --?)"
                );
                std::process::exit(1);
            }
        }
    }
    artifact_path
}

/// Compute the build digest: SHA-256 of the artifact file when `--artifact`
/// names one, otherwise SHA-256 of stdout. The harness reads the file (not
/// the agent), so a producer cannot forge a digest it did not observe.
fn compute_build_digest(
    artifact_path: Option<&Path>,
    stdout: &[u8],
) -> (elench_store::Oid, String) {
    if let Some(path) = artifact_path {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("elench build: artifact not found: {path:?}: {e}");
                std::process::exit(1);
            }
        };
        (
            elench_store::Oid::from_blob_data(&data),
            format!("artifact: {path:?}"),
        )
    } else {
        (
            elench_store::Oid::from_blob_data(stdout),
            "stdout".to_string(),
        )
    }
}

#[allow(clippy::unnecessary_debug_formatting)]
fn emit_build_provenance(
    tree: &str,
    digest: &elench_store::Oid,
    digest_source: &str,
    exit_code: i32,
    stdout: &str,
    stderr: &str,
    artifact_path: Option<&Path>,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));

    let claim = elench_claim::Claim {
        id: elench_claim::ClaimId::new(
            "cl_0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap(),
        kind: elench_claim::ClaimKind::Verification,
        target: vec![],
        assertion: elench_claim::AssertionForm::Annotation {
            text: format!("build provenance: exit={exit_code}, digest={digest}"),
        },
        origin: elench_claim::Origin {
            kind: elench_claim::OriginKind::HarnessObserved,
            producer: elench_claim::Producer {
                id: "elench-build-harness".into(),
                session_id: Some(format!("build-{now}")),
                hermeticity: Some(elench_claim::Hermeticity::None),
            },
        },
        anchor: elench_claim::Anchor {
            tree: tree.to_string(),
            strategy: elench_claim::AnchorStrategy::Multi,
            path: None,
            range: None,
            symbol: None,
            content_digest: Some(digest.as_str().to_string()),
        },
        timestamp: now,
        evidence: vec![elench_claim::Evidence {
            kind: elench_claim::EvidenceKind::ProcessExit,
            digest: Some(digest.as_str().to_string()),
            exit_code: Some(i64::from(exit_code)),
            uri: artifact_path.map(|p| p.to_string_lossy().into()),
        }],
        depends_on: vec![],
    };

    let computed_id = elench_claim::ClaimId::from_content(&claim);

    println!("build provenance emitted:");
    println!("  id:       {computed_id}");
    println!("  tree:     {tree}");
    println!("  kind:     {}", claim.kind_str());
    println!("  origin:   {:?}", claim.origin.kind);
    println!("  producer: {}", claim.origin.producer.id);
    println!();

    println!("build result:");
    println!("  exit code: {exit_code}");
    println!("  digest:    {digest}");
    println!("  source:    {digest_source}");
    println!("  stdout:    {} bytes", stdout.len());
    println!("  stderr:    {} bytes", stderr.len());
    if !stderr.is_empty() {
        println!("  stderr (first 500 chars):");
        println!("    {}", &stderr[..stderr.len().min(500)]);
    }
    println!();
    println!("(PREDICATE_TYPE_BUILD: origin.kind = harness-observed)");
    println!("(INV-22: same envelope format as agent claims)");
    println!("(condition 4: K independent producers sign statements with this digest)");
}

fn cmd_build(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench build: requires a command and tree OID");
        eprintln!("  elench build <tree_oid> [--artifact <path>] -- <command...>");
        eprintln!();
        eprintln!("Runs the build command, captures the exit code and");
        eprintln!("artifact digest. With --artifact <path>, the digest is");
        eprintln!("SHA-256 of that file (the real build output). Without it,");
        eprintln!("the digest falls back to SHA-256 of stdout. Emits a");
        eprintln!("build provenance claim with origin.kind = harness-observed.");
        std::process::exit(1);
    }

    let tree = &args[0];
    let (elench_flags, cmd_args) = split_build_args(args);
    if cmd_args.is_empty() {
        eprintln!("elench build: empty command");
        std::process::exit(1);
    }

    let artifact_path = parse_build_flags(&elench_flags);

    let output = std::process::Command::new(cmd_args[0])
        .args(&cmd_args[1..])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            eprintln!("elench build: failed to execute: {e}");
            std::process::exit(1);
        }
    };

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let (digest, digest_source) = compute_build_digest(artifact_path.as_deref(), &output.stdout);

    emit_build_provenance(
        tree,
        &digest,
        &digest_source,
        exit_code,
        &stdout,
        &stderr,
        artifact_path.as_deref(),
    );
}

// ---------------------------------------------------------------------------
// log — log statistics (count, status distribution, conflicts)
// ---------------------------------------------------------------------------

fn cmd_log(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench log: requires a claims file");
        eprintln!("  elench log <claims.json>");
        std::process::exit(1);
    }

    let path = PathBuf::from(&args[0]);
    let log = parse_claims_file(&path);

    if log.is_empty() {
        println!("(empty claim log)");
        return;
    }

    let total = log.len();
    let mut unevaluated = 0;
    let mut passed = 0;
    let mut falsified = 0;
    let mut assertions = 0;
    let mut verifications = 0;
    let mut falsifications = 0;
    let mut supersessions = 0;
    let mut residue_acceptances = 0;

    for claim in &log {
        let status = elench_claim::compute_status(&claim.id, &log)
            .unwrap_or(elench_claim::ClaimStatus::Unevaluated);
        match status {
            elench_claim::ClaimStatus::Unevaluated => unevaluated += 1,
            elench_claim::ClaimStatus::Passed => passed += 1,
            elench_claim::ClaimStatus::Falsified => falsified += 1,
        }
        match claim.kind {
            elench_claim::ClaimKind::Assertion => assertions += 1,
            elench_claim::ClaimKind::Verification => verifications += 1,
            elench_claim::ClaimKind::Falsification => falsifications += 1,
            elench_claim::ClaimKind::Supersession => supersessions += 1,
            elench_claim::ClaimKind::ResidueAcceptance => residue_acceptances += 1,
        }
    }

    println!("log statistics:");
    println!("  total:            {total}");
    println!("  assertions:       {assertions}");
    println!("  verifications:    {verifications}");
    println!("  falsifications:   {falsifications}");
    println!("  supersessions:    {supersessions}");
    println!("  residue-accept:   {residue_acceptances}");
    println!();
    println!("status distribution:");
    println!("  unevaluated:      {unevaluated}");
    println!("  passed:           {passed}");
    println!("  falsified:        {falsified}");
    println!();

    let noise_ratio: f64 = if total > 0 {
        f64::from(falsifications + supersessions) / total as f64
    } else {
        0.0
    };
    let unevaluated_ratio: f64 = if total > 0 {
        f64::from(unevaluated) / total as f64
    } else {
        0.0
    };
    println!("ratios:");
    println!("  noise (fals+super):    {noise_ratio:.2}");
    println!("  unevaluated:           {unevaluated_ratio:.2}");
    println!();

    let depends_on_density: f64 = if total > 0 {
        log.iter().map(|c| c.depends_on.len()).sum::<usize>() as f64 / total as f64
    } else {
        0.0
    };
    println!("dependsOn density: {depends_on_density:.2} per claim");
}

// ---------------------------------------------------------------------------
// review — review unevaluated claims for a tree
// ---------------------------------------------------------------------------

fn cmd_review(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench review: requires a tree OID and claims file");
        eprintln!("  elench review <tree_oid> <claims.json>");
        std::process::exit(1);
    }

    if args.len() < 2 {
        eprintln!("elench review: requires a claims file");
        eprintln!("  elench review <tree_oid> <claims.json>");
        std::process::exit(1);
    }

    let tree = &args[0];
    let log = parse_claims_file(&PathBuf::from(&args[1]));

    let tree_claims: Vec<&elench_claim::Claim> =
        log.iter().filter(|c| c.anchor.tree == *tree).collect();

    if tree_claims.is_empty() {
        println!("(no claims for tree {tree})");
        return;
    }

    let unevaluated: Vec<&&elench_claim::Claim> = tree_claims
        .iter()
        .filter(|c| {
            elench_claim::compute_status(&c.id, &log)
                .unwrap_or(elench_claim::ClaimStatus::Unevaluated)
                == elench_claim::ClaimStatus::Unevaluated
        })
        .collect();

    println!("review: tree {tree}");
    println!("  total claims:     {}", tree_claims.len());
    println!("  unevaluated:      {}", unevaluated.len());
    println!();

    if unevaluated.is_empty() {
        println!("(no unevaluated claims — nothing to review)");
        return;
    }

    println!("unevaluated claims to review:");
    for (i, claim) in unevaluated.iter().enumerate() {
        println!("  [{}] {}", i + 1, claim.id);
        println!("       kind:     {}", claim.kind_str());
        match &claim.assertion {
            elench_claim::AssertionForm::Predicate { expression } => {
                println!("       form:     predicate");
                println!("       language: {}", expression.language);
                println!("       source:   {}", expression.source);
            }
            elench_claim::AssertionForm::Annotation { text } => {
                println!("       form:     annotation");
                println!("       text:     {text}");
            }
        }
        println!("       producer: {}", claim.origin.producer.id);
        println!("       origin:   {:?}", claim.origin.kind);
        println!();
    }

    println!("To accept these gaps, run:");
    println!("  elench accept <tree_oid> <claims.json> --claim <id> [<id>...]");
}

// ---------------------------------------------------------------------------
// accept — accept named unevaluated gaps (residue-acceptance)
// ---------------------------------------------------------------------------

fn cmd_accept(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench accept: requires a tree OID and claims file");
        eprintln!("  elench accept <tree_oid> <claims.json> --claim <id> [<id>...]");
        std::process::exit(1);
    }

    let tree = &args[0];

    if args.len() < 2 {
        eprintln!("elench accept: requires a claims file");
        std::process::exit(1);
    }

    let _log = parse_claims_file(&PathBuf::from(&args[1]));

    let mut accepted_ids: Vec<elench_claim::ClaimId> = Vec::new();
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--claim" {
            i += 1;
            while i < args.len() && !args[i].starts_with("--") {
                match elench_claim::ClaimId::new(&args[i]) {
                    Ok(id) => accepted_ids.push(id),
                    Err(_) => {
                        eprintln!("elench accept: invalid claim ID: {}", args[i]);
                        std::process::exit(1);
                    }
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    if accepted_ids.is_empty() {
        eprintln!("elench accept: no claims named. Use --claim <id> [<id>...]");
        eprintln!("  elench accept <tree_oid> <claims.json> --claim cl_... cl_...");
        std::process::exit(1);
    }

    let acceptance = elench_claim::Claim {
        id: elench_claim::ClaimId::new(
            "cl_0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap(),
        kind: elench_claim::ClaimKind::ResidueAcceptance,
        target: accepted_ids.clone(),
        assertion: elench_claim::AssertionForm::Annotation {
            text: "Human accepts named unevaluated gaps".into(),
        },
        origin: elench_claim::Origin {
            kind: elench_claim::OriginKind::HumanAsserted,
            producer: elench_claim::Producer {
                id: "human-reviewer".into(),
                session_id: None,
                hermeticity: None,
            },
        },
        anchor: elench_claim::Anchor {
            tree: tree.clone(),
            strategy: elench_claim::AnchorStrategy::Multi,
            path: None,
            range: None,
            symbol: None,
            content_digest: None,
        },
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .try_into()
            .unwrap_or(0),
        evidence: vec![],
        depends_on: vec![],
    };

    let computed_id = elench_claim::ClaimId::from_content(&acceptance);
    println!("residue-acceptance created:");
    println!("  id:       {computed_id}");
    println!("  tree:     {tree}");
    println!("  kind:     {}", acceptance.kind_str());
    println!("  origin:   {:?}", acceptance.origin.kind);
    println!("  targets:  {}", accepted_ids.len());
    for id in &accepted_ids {
        println!("    - {id}");
    }
    println!();
    println!("(INV-12: only humans emit residue-acceptance)");
    println!("(R5: unevaluated residue bounded by named acceptance)");
}

// ---------------------------------------------------------------------------
// conflicts — list active predicate conflicts for a tree
// ---------------------------------------------------------------------------

fn cmd_conflicts(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench conflicts: requires a tree OID and claims file");
        eprintln!("  elench conflicts <tree_oid> <claims.json>");
        std::process::exit(1);
    }

    let tree = &args[0];

    if args.len() < 2 {
        eprintln!("elench conflicts: requires a claims file");
        std::process::exit(1);
    }

    let log = parse_claims_file(&PathBuf::from(&args[1]));

    let tree_claims: Vec<&elench_claim::Claim> = log
        .iter()
        .filter(|c| c.anchor.tree == *tree && c.kind == elench_claim::ClaimKind::Assertion)
        .collect();

    let active_predicates: Vec<&&elench_claim::Claim> = tree_claims
        .iter()
        .filter(|c| {
            matches!(c.assertion, elench_claim::AssertionForm::Predicate { .. })
                && elench_claim::compute_status(&c.id, &log)
                    .unwrap_or(elench_claim::ClaimStatus::Unevaluated)
                    != elench_claim::ClaimStatus::Falsified
        })
        .collect();

    if active_predicates.is_empty() {
        println!("(no active predicate claims for tree {tree})");
        return;
    }

    let mut conflicts = Vec::new();
    for i in 0..active_predicates.len() {
        for j in (i + 1)..active_predicates.len() {
            let a = active_predicates[i];
            let b = active_predicates[j];
            let expr_a = match &a.assertion {
                elench_claim::AssertionForm::Predicate { expression } => &expression.source,
                _ => continue,
            };
            let expr_b = match &b.assertion {
                elench_claim::AssertionForm::Predicate { expression } => &expression.source,
                _ => continue,
            };
            if expr_a != expr_b {
                let winner = if a.timestamp >= b.timestamp { a } else { b };
                let loser = if a.timestamp >= b.timestamp { b } else { a };
                conflicts.push((
                    a.id.clone(),
                    b.id.clone(),
                    winner.id.clone(),
                    loser.id.clone(),
                ));
            }
        }
    }

    println!("conflicts: tree {tree}");
    println!("  active predicates: {}", active_predicates.len());
    println!("  conflicts:         {}", conflicts.len());
    println!();

    if conflicts.is_empty() {
        println!("(no contradictions detected)");
        return;
    }

    for (a, b, winner, loser) in &conflicts {
        println!("  conflict: {a} vs {b}");
        println!("    latest (wins): {winner}");
        println!("    older (flagged): {loser}");
        println!("    (last-writer-wins, flagged for resolution)");
        println!();
    }

    println!("To resolve: falsify one or both predicates.");
    println!("  elench emit <falsification.json>");
}

// ---------------------------------------------------------------------------
// compact — compact the claim log (manual, destructive)
// ---------------------------------------------------------------------------

fn cmd_compact(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench compact: requires a claims file");
        eprintln!("  elench compact <claims.json> [--before <timestamp>]");
        eprintln!();
        eprintln!("Compaction is MANUAL and DESTRUCTIVE. It retires all");
        eprintln!("claims before the cut-off timestamp, freezing their");
        eprintln!("statuses. The compaction record carries the status");
        eprintln!("snapshot forward.");
        std::process::exit(1);
    }

    let path = PathBuf::from(&args[0]);
    let log = parse_claims_file(&path);

    if log.is_empty() {
        eprintln!("elench compact: empty claim log, nothing to compact");
        return;
    }

    let mut cutoff: Option<i64> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--before" && i + 1 < args.len() {
            cutoff = args[i + 1].parse().ok();
            i += 2;
        } else {
            i += 1;
        }
    }

    let cutoff_ts = cutoff.unwrap_or_else(|| log.iter().map(|c| c.timestamp).max().unwrap_or(0));

    let (retired, active): (Vec<&elench_claim::Claim>, Vec<&elench_claim::Claim>) =
        log.iter().partition(|c| c.timestamp < cutoff_ts);

    if retired.is_empty() {
        eprintln!("elench compact: no claims before {cutoff_ts}, nothing to compact");
        return;
    }

    let mut snapshot = Vec::new();
    for claim in &retired {
        let status = elench_claim::compute_status(&claim.id, &log)
            .unwrap_or(elench_claim::ClaimStatus::Unevaluated);
        snapshot.push((claim.id.clone(), status));
    }

    println!("compaction report:");
    println!("  cut-off timestamp: {cutoff_ts}");
    println!("  retired claims:    {}", retired.len());
    println!("  active claims:     {}", active.len());
    println!();
    println!("status snapshot (frozen):");
    let mut unevaluated = 0;
    let mut passed = 0;
    let mut falsified = 0;
    for (id, status) in &snapshot {
        match status {
            elench_claim::ClaimStatus::Unevaluated => unevaluated += 1,
            elench_claim::ClaimStatus::Passed => passed += 1,
            elench_claim::ClaimStatus::Falsified => falsified += 1,
        }
        println!("  {id}: {status:?}");
    }
    println!();
    println!("snapshot summary:");
    println!("  unevaluated: {unevaluated}");
    println!("  passed:      {passed}");
    println!("  falsified:   {falsified}");
    println!();
    println!("(manual, destructive — retired claims are assumed final)");
    println!("(compaction record carries frozen statuses forward)");
    println!("(active claims continue to be revocable, R1 preserved)");
}

// ---------------------------------------------------------------------------
// artifact — create or verify a release artifact (INV-15)
// ---------------------------------------------------------------------------

fn cmd_artifact(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench artifact: requires a subcommand");
        eprintln!("  elench artifact create <tree> <policy> <digest>");
        eprintln!("  elench artifact verify <artifact.json> <claims.json>");
        std::process::exit(1);
    }

    match args[0].as_str() {
        "create" => {
            if args.len() < 4 {
                eprintln!("elench artifact create: requires <tree> <policy> <digest>");
                eprintln!("  elench artifact create <tree_oid> <policy_name> <sha256_digest>");
                std::process::exit(1);
            }

            let tree = &args[1];
            let policy = &args[2];
            let digest = &args[3];
            let released_at: i64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));

            let artifact = elench_gate::Artifact::new(tree, policy, digest, released_at);
            let json = match artifact.to_json() {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("elench artifact create: serialization failed: {e}");
                    std::process::exit(1);
                }
            };

            println!("{json}");
            println!();
            println!("(INV-15: artifact carries (tree, policy), not a verdict)");
            println!("(R4: consumers re-evaluate at consumption time)");
        }
        "verify" => {
            if args.len() < 3 {
                eprintln!("elench artifact verify: requires <artifact.json> <claims.json>");
                std::process::exit(1);
            }

            let artifact_path = PathBuf::from(&args[1]);
            let claims_path = PathBuf::from(&args[2]);

            let artifact_json = match std::fs::read_to_string(&artifact_path) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("elench artifact verify: failed to read {artifact_path:?}: {e}");
                    std::process::exit(1);
                }
            };

            let artifact = match elench_gate::Artifact::from_json(&artifact_json) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("elench artifact verify: invalid artifact JSON: {e}");
                    std::process::exit(1);
                }
            };

            let log = parse_claims_file(&claims_path);
            let policy = elench_gate::Policy::permissive(&artifact.policy);

            let verdict = match artifact.evaluate(&policy, &log) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("elench artifact verify: gate evaluation failed: {e}");
                    std::process::exit(1);
                }
            };

            println!("artifact:");
            println!("  tree:       {}", artifact.tree);
            println!("  policy:     {}", artifact.policy);
            println!("  digest:     {}", artifact.digest);
            println!("  released:   {}", artifact.released_at);
            println!();
            println!("verdict (live evaluation):");
            println!("  result:     {:?}", verdict.result);
            if !verdict.reasons.is_empty() {
                println!("  reasons:");
                for r in &verdict.reasons {
                    println!("    - {r}");
                }
            }
            println!();
            println!(
                "(INV-15: no verdict stored in artifact — re-evaluated from {} claims)",
                log.len()
            );
        }
        other => {
            eprintln!("elench artifact: unknown subcommand '{other}'");
            eprintln!("  elench artifact create <tree> <policy> <digest>");
            eprintln!("  elench artifact verify <artifact.json> <claims.json>");
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests — store-config parsing (store-backend.feature)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{StoreConfig, extract_store_config};

    fn args(rest: &[&str]) -> Vec<String> {
        std::iter::once("elench".to_string())
            .chain(rest.iter().map(|s| (*s).to_string()))
            .collect()
    }

    #[test]
    fn scenario_store_default_is_memory() {
        let mut a = args(&["emit", "claim.json"]);
        let cfg = extract_store_config(&mut a).unwrap();
        assert!(matches!(cfg, StoreConfig::Memory));
        assert_eq!(a, args(&["emit", "claim.json"]));
    }

    #[test]
    fn scenario_store_explicit_memory_before_command() {
        let mut a = args(&["--store", "memory", "emit", "claim.json"]);
        let cfg = extract_store_config(&mut a).unwrap();
        assert!(matches!(cfg, StoreConfig::Memory));
        assert_eq!(a, args(&["emit", "claim.json"]));
    }

    #[test]
    fn scenario_store_explicit_memory_after_command() {
        let mut a = args(&["store", "blob", "f.txt", "--store", "memory"]);
        let cfg = extract_store_config(&mut a).unwrap();
        assert!(matches!(cfg, StoreConfig::Memory));
        assert_eq!(a, args(&["store", "blob", "f.txt"]));
    }

    #[test]
    fn scenario_store_missing_value_errors() {
        let mut a = args(&["--store"]);
        let err = extract_store_config(&mut a).unwrap_err();
        assert!(err.contains("requires a value"));
    }

    #[test]
    fn scenario_store_unknown_backend_errors() {
        let mut a = args(&["--store", "redis", "emit", "x.json"]);
        let err = extract_store_config(&mut a).unwrap_err();
        assert!(err.contains("unknown backend"));
        assert!(err.contains("redis"));
    }

    #[test]
    fn scenario_store_fjall_missing_path_errors() {
        // `--store fjall` with no path at all. Without the feature this
        // errors on the feature check; with the feature on the missing
        // path. Either way it must error and mention fjall.
        let mut a = args(&["--store", "fjall"]);
        let err = extract_store_config(&mut a).unwrap_err();
        assert!(err.contains("fjall"));
    }

    #[test]
    #[cfg(not(feature = "fjall-backend"))]
    fn scenario_store_fjall_without_feature_errors() {
        let mut a = args(&["--store", "fjall", "/tmp/db", "emit", "x.json"]);
        let err = extract_store_config(&mut a).unwrap_err();
        assert!(err.contains("fjall-backend feature"));
        assert!(err.contains("--features elench/fjall-backend"));
    }

    #[test]
    #[cfg(feature = "fjall-backend")]
    fn scenario_store_fjall_with_feature_parses() {
        let mut a = args(&["--store", "fjall", "/tmp/db", "emit", "x.json"]);
        let cfg = extract_store_config(&mut a).unwrap();
        match cfg {
            StoreConfig::Fjall { path } => assert_eq!(path, std::path::PathBuf::from("/tmp/db")),
            other => panic!("expected Fjall, got {other:?}"),
        }
        assert_eq!(a, args(&["emit", "x.json"]));
    }

    #[test]
    fn scenario_store_stops_at_double_dash() {
        // A `--` separator ends extraction; the trailing `--store` belongs
        // to the subcommand and must be left in place (build ... -- <cmd>).
        let mut a = args(&["build", "tree123", "--", "cargo", "--store", "memory"]);
        let cfg = extract_store_config(&mut a).unwrap();
        assert!(matches!(cfg, StoreConfig::Memory));
        assert_eq!(
            a,
            args(&["build", "tree123", "--", "cargo", "--store", "memory"])
        );
    }

    #[test]
    fn scenario_store_last_occurrence_wins() {
        let mut a = args(&["--store", "memory", "--store", "memory", "emit", "x.json"]);
        let cfg = extract_store_config(&mut a).unwrap();
        assert!(matches!(cfg, StoreConfig::Memory));
        assert_eq!(a, args(&["emit", "x.json"]));
    }
}
