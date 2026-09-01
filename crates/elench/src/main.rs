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

use std::path::PathBuf;

#[allow(clippy::unnecessary_debug_formatting)]
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let command = &args[1];
    let rest = &args[2..];

    match command.as_str() {
        "emit" => cmd_emit(rest),
        "verify" => cmd_verify(rest),
        "status" => cmd_status(rest),
        "gate" => cmd_gate(rest),
        "blast" => cmd_blast(rest),
        "git" => cmd_git(rest),
        "store" => cmd_store(rest),
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
    println!("    elench <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    emit       Create and sign a claim, store in the store");
    println!("    verify     Verify an envelope and validate the claim");
    println!("    status     Compute a claim's status by folding the log");
    println!("    gate       Evaluate the release gate for a tree");
    println!("    blast      Compute the blast radius from a claim");
    println!("    git        Materialize the git projection");
    println!("    store      Store a blob or tree");
    println!("    help       Print this message");
    println!("    version    Print version information");
    println!();
    println!("The claim log IS the primary history (ADR-0001).");
    println!("The git CLI works because elench synthesizes git objects");
    println!("from the claim log (ADR-0002). The projection is read-only.");
}

// ---------------------------------------------------------------------------
// emit — create, sign, and store a claim
// ---------------------------------------------------------------------------

#[allow(clippy::unnecessary_debug_formatting)]
fn cmd_emit(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench emit: requires a claim JSON file");
        eprintln!("  elench emit <claim.json>");
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

    // 1. Parse the claim from JSON
    let mut claim: elench_claim::Claim = match serde_json::from_str(&json) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("elench emit: invalid claim JSON: {e}");
            std::process::exit(1);
        }
    };

    // 2. Compute the claim ID from content (INV-28)
    let computed_id = elench_claim::ClaimId::from_content(&claim);
    claim.id = computed_id.clone();

    // 3. Validate the claim against emission rules
    //    For Phase 5, we use a default signer (agent).
    let signer = elench_claim::SignerIdentity {
        key_id: "default-agent-key".into(),
        entity: elench_claim::SignerEntity::Agent,
    };
    let log: Vec<elench_claim::Claim> = Vec::new();

    if let Err(e) = elench_claim::validate_claim(&claim, &signer, &log) {
        eprintln!("elench emit: claim rejected by validator: {e}");
        std::process::exit(1);
    }

    // 4. Sign the claim in a DSSE envelope
    let signing_key = elench_envelope::SigningKey::new(
        "default-agent-key",
        elench_claim::SignerEntity::Agent,
        "elench-default-secret",
    );
    let envelope = elench_envelope::sign(&claim, &signing_key);

    // 5. Store the claim
    let mut store = elench_store::Store::new();
    let stored_oid = match store.store_claim(&claim) {
        Ok(oid) => oid,
        Err(e) => {
            eprintln!("elench emit: failed to store claim: {e}");
            std::process::exit(1);
        }
    };

    // 6. Print results
    println!("claim emitted:");
    println!("  id:       {computed_id}");
    println!("  kind:     {}", claim.kind_str());
    println!("  tree:     {}", claim.anchor.tree);
    println!("  producer: {}", claim.origin.producer.id);
    println!("  stored:   {stored_oid}");
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

#[allow(clippy::unnecessary_debug_formatting)]
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

    // 1. Parse the envelope from JSON
    let envelope: elench_envelope::Envelope = match serde_json::from_str(&json) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("elench verify: invalid envelope JSON: {e}");
            std::process::exit(1);
        }
    };

    // 2. Set up verifying keys (for Phase 5, a default agent key)
    let keys = vec![elench_envelope::VerifyingKey {
        key_id: "default-agent-key".into(),
        entity: elench_claim::SignerEntity::Agent,
    }];
    let secrets = vec![(
        "default-agent-key".to_string(),
        "elench-default-secret".to_string(),
    )];

    // 3. Verify the envelope's signature and extract the claim
    let (claim, signer) = match elench_envelope::verify(&envelope, &keys, &secrets) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("elench verify: envelope verification failed: {e}");
            std::process::exit(1);
        }
    };

    // 4. Validate the claim against emission rules
    let log: Vec<elench_claim::Claim> = Vec::new();
    if let Err(e) = elench_claim::validate_claim(&claim, &signer, &log) {
        eprintln!("elench verify: claim rejected by validator: {e}");
        std::process::exit(1);
    }

    // 5. Compute the claim's status
    let status = elench_claim::compute_status(&claim.id, &log)
        .unwrap_or(elench_claim::ClaimStatus::Unevaluated);

    // 6. Print results
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
        eprintln!("  elench status <claim_id>");
        std::process::exit(1);
    }

    let claim_id = &args[0];

    if let Ok(id) = elench_claim::ClaimId::new(claim_id) {
        let status = elench_claim::compute_status(&id, &[])
            .unwrap_or(elench_claim::ClaimStatus::Unevaluated);
        println!("claim: {id}");
        println!("status: {status:?}");
        println!();
        println!("(computed from empty log — live evaluation, INV-14)");
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
        eprintln!("  elench gate <tree_oid>");
        std::process::exit(1);
    }

    let tree = &args[0];

    let policy = elench_gate::Policy::permissive("default");
    let verdict = elench_gate::evaluate(tree, &policy, &[]).unwrap_or_else(|e| {
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
    println!();
    println!("(live evaluation — INV-13: no build capability required)");
}

// ---------------------------------------------------------------------------
// blast — compute the blast radius from a claim
// ---------------------------------------------------------------------------

fn cmd_blast(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench blast: requires a claim ID");
        eprintln!("  elench blast <claim_id>");
        std::process::exit(1);
    }

    let claim_id = &args[0];

    if let Ok(id) = elench_claim::ClaimId::new(claim_id) {
        let radius = elench_claim::blast_radius(&id, &[]);
        println!("claim: {id}");
        println!("blast radius: {} claims", radius.len());
        if !radius.is_empty() {
            println!("claims in blast radius:");
            for c in &radius {
                println!("  - {c}");
            }
        }
        println!();
        println!("(computed from empty log — transitive dependsOn closure)");
    } else {
        eprintln!("elench blast: invalid claim ID: {claim_id}");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// git — materialize the git projection
// ---------------------------------------------------------------------------

#[allow(clippy::unnecessary_debug_formatting)]
fn cmd_git(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench git: requires a claim log file");
        eprintln!("  elench git <claims.json>");
        eprintln!("  elench git oneline <claims.json>");
        eprintln!("  elench git full <claims.json>");
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

    // For Phase 5, demonstrate the projection with an empty log.
    let log: Vec<elench_claim::Claim> = Vec::new();
    let store = elench_store::Store::new();

    if log.is_empty() {
        println!("(empty claim log — nothing to project)");
        println!("(elench git: read claims from {path:?}, synthesize git objects)");
        println!("(elench-projection::synthesize: deterministic, ADR-0002/0007)");
        return;
    }

    match elench_projection::synthesize(&log, &store) {
        Ok(projection) => {
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

#[allow(clippy::unnecessary_debug_formatting)]
fn cmd_store(args: &[String]) {
    if args.is_empty() {
        eprintln!("elench store: requires a subcommand");
        eprintln!("  elench store blob <file>");
        eprintln!("  elench store tree <file1> <file2> ...");
        std::process::exit(1);
    }

    match args[0].as_str() {
        "blob" => {
            if args.len() < 2 {
                eprintln!("elench store blob: requires a file path");
                std::process::exit(1);
            }
            let path = PathBuf::from(&args[1]);
            match std::fs::read(&path) {
                Ok(data) => {
                    let oid = elench_store::Oid::from_blob_data(&data);
                    println!("blob: {oid}");
                    println!("size: {} bytes", data.len());
                    println!("(SHA-256 content address — identical to git SHA-256 blob OID)");
                }
                Err(e) => {
                    eprintln!("elench store blob: failed to read {path:?}: {e}");
                    std::process::exit(1);
                }
            }
        }
        "tree" => {
            if args.len() < 2 {
                eprintln!("elench store tree: requires at least one file path");
                std::process::exit(1);
            }

            let mut store = elench_store::Store::new();
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

            let tree = elench_store::Tree::from_entries(entries);
            println!("tree: {}", tree.oid);
            println!("entries: {}", tree.entries.len());
            println!("(SHA-256 content address — identical to git SHA-256 tree OID)");
        }
        other => {
            eprintln!("elench store: unknown subcommand '{other}'");
            eprintln!("  elench store blob <file>");
            eprintln!("  elench store tree <file1> <file2> ...");
            std::process::exit(1);
        }
    }
}
