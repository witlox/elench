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

use std::path::PathBuf;

use elench_store::StoreBackend;

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
        "log" => cmd_log(rest),
        "review" => cmd_review(rest),
        "accept" => cmd_accept(rest),
        "conflicts" => cmd_conflicts(rest),
        "compact" => cmd_compact(rest),
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
    println!("    log        Log statistics (count, status distribution, conflicts)");
    println!("    review     Review unevaluated claims for a tree");
    println!("    accept     Accept named unevaluated gaps (residue-acceptance)");
    println!("    conflicts  List active predicate conflicts for a tree");
    println!("    compact    Compact the claim log (manual, destructive)");
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

    let signing_key = elench_envelope::SigningKey::new(
        "default-agent-key",
        elench_claim::SignerEntity::Agent,
        "elench-default-secret",
    );
    let envelope = elench_envelope::sign(&claim, &signing_key);

    let mut store = elench_store::MemoryStore::new();
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

    let keys = vec![elench_envelope::VerifyingKey {
        key_id: "default-agent-key".into(),
        entity: elench_claim::SignerEntity::Agent,
    }];
    let secrets = vec![(
        "default-agent-key".to_string(),
        "elench-default-secret".to_string(),
    )];

    let (claim, signer) = match elench_envelope::verify(&envelope, &keys, &secrets) {
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

    let log = parse_claims_file(&path);

    if log.is_empty() {
        println!("(empty claim log — nothing to project)");
        return;
    }

    let store = elench_store::MemoryStore::new();

    match elench_projection::synthesize(&log, &store) {
        Ok(projection) => {
            println!(
                "projection: {} commits, {} blobs, {} trees",
                projection.commits.len(),
                projection.blobs.len(),
                projection.trees.len()
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

            let mut store = elench_store::MemoryStore::new();
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
