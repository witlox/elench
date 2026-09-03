//! Integration tests verifying the 6 cross-context interactions from
//! `specs/cross-context/interactions.md`.
//!
//! These tests exercise multiple crates together, verifying that data
//! flows correctly across bounded context boundaries.

use elench_claim::{
    Anchor, AnchorStrategy, AssertionForm, Claim, ClaimId, ClaimKind, Expression, Origin,
    OriginKind, Producer, SignerEntity, SignerIdentity,
};
use elench_envelope::{SigningKey, sign, verify};
use elench_gate::{Policy, VerdictResult, evaluate};
use elench_projection::{git_log_oneline, synthesize};
use elench_store::{MemoryStore as Store, StoreBackend};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_claim(
    id: &str,
    kind: ClaimKind,
    origin_kind: OriginKind,
    tree: &str,
    timestamp: i64,
) -> Claim {
    Claim {
        id: ClaimId::new(id).unwrap(),
        kind,
        target: vec![],
        assertion: AssertionForm::Annotation {
            text: "test".into(),
        },
        origin: Origin {
            kind: origin_kind,
            producer: Producer {
                id: "test-producer".into(),
                session_id: None,
                hermeticity: None,
            },
        },
        anchor: Anchor {
            tree: tree.into(),
            strategy: AnchorStrategy::Multi,
            path: Some("src/main.rs".into()),
            range: Some([1, 10]),
            symbol: None,
            content_digest: None,
        },
        timestamp,
        evidence: vec![],
        depends_on: vec![],
    }
}

fn make_predicate_claim(id: &str, tree: &str, ts: i64) -> Claim {
    let mut c = make_claim(
        id,
        ClaimKind::Assertion,
        OriginKind::AgentAsserted,
        tree,
        ts,
    );
    c.assertion = AssertionForm::Predicate {
        expression: Expression {
            language: "elench-predicate-v1".into(),
            source: "exists(\"Cargo.toml\")".into(),
            digest: None,
        },
    };
    c
}

const TREE: &str = "abc123def456789abc123def456789abc123def456789abc123def456789abcd";
const ID_A: &str = "cl_0000000000000000000000000000000000000000000000000000000000000001";
const ID_B: &str = "cl_0000000000000000000000000000000000000000000000000000000000000002";
const ID_C: &str = "cl_0000000000000000000000000000000000000000000000000000000000000003";

// ---------------------------------------------------------------------------
// Interaction 1: Claim Emission → elench Store
// ---------------------------------------------------------------------------

#[test]
fn interaction_1_claim_emission_to_store() {
    let claim = make_predicate_claim(ID_A, TREE, 1_700_000_000);

    // Validate the claim (elench-claim)
    let signer = SignerIdentity {
        key_id: "agent-key".into(),
        entity: SignerEntity::Agent,
    };
    elench_claim::validate_claim(&claim, &signer, &[]).unwrap();

    // Sign the claim (elench-envelope)
    let key = SigningKey::generate(SignerEntity::Agent);
    let _envelope = sign(&claim, &key);

    // Store the claim (elench-store)
    let mut store = Store::new();
    let oid = store.store_claim(&claim).unwrap();

    // Verify: claim is in the store and can be read back
    assert!(store.has_claim(&oid));
    let claims = store.read_all_claims().unwrap();
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].id, claim.id);
    assert_eq!(claims[0].anchor.tree, TREE);
}

// ---------------------------------------------------------------------------
// Interaction 2: Claim Evaluation ← elench Store
// ---------------------------------------------------------------------------

#[test]
fn interaction_2_claim_evaluation_from_store() {
    let claim_a = make_predicate_claim(ID_A, TREE, 1_700_000_000);
    let mut claim_b = make_predicate_claim(ID_B, TREE, 1_700_000_001);
    claim_b.depends_on = vec![ClaimId::new(ID_A).unwrap()];

    // Store claims
    let mut store = Store::new();
    store.store_claim(&claim_a).unwrap();
    store.store_claim(&claim_b).unwrap();

    // Read claims from store
    let log = store.read_all_claims().unwrap();
    assert_eq!(log.len(), 2);

    // Compute status (elench-claim) using claims from the store
    let status_a = elench_claim::compute_status(&claim_a.id, &log).unwrap();
    assert_eq!(status_a, elench_claim::ClaimStatus::Unevaluated);

    // Falsify A
    let falsification = Claim {
        id: ClaimId::new(ID_C).unwrap(),
        kind: ClaimKind::Falsification,
        target: vec![ClaimId::new(ID_A).unwrap()],
        assertion: AssertionForm::Annotation {
            text: "wrong".into(),
        },
        origin: Origin {
            kind: OriginKind::HarnessObserved,
            producer: Producer {
                id: "harness".into(),
                session_id: None,
                hermeticity: None,
            },
        },
        anchor: Anchor {
            tree: TREE.into(),
            strategy: AnchorStrategy::Multi,
            path: None,
            range: None,
            symbol: None,
            content_digest: None,
        },
        timestamp: 1_700_000_002,
        evidence: vec![],
        depends_on: vec![],
    };
    let mut log_with_f = log.clone();
    log_with_f.push(falsification);

    let status_a_after = elench_claim::compute_status(&claim_a.id, &log_with_f).unwrap();
    assert_eq!(status_a_after, elench_claim::ClaimStatus::Falsified);
}

// ---------------------------------------------------------------------------
// Interaction 3: Release Gating ← Claim Evaluation
// ---------------------------------------------------------------------------

#[test]
fn interaction_3_release_gating_from_claim_evaluation() {
    let claim = make_predicate_claim(ID_A, TREE, 1_700_000_000);

    // Store the claim
    let mut store = Store::new();
    store.store_claim(&claim).unwrap();

    // Read claims and evaluate the gate (elench-gate)
    let log = store.read_all_claims().unwrap();
    let policy = Policy::permissive("test");
    let verdict = evaluate(TREE, &policy, &log).unwrap();

    // With one unevaluated agent-asserted claim and permissive policy: pass
    assert_eq!(verdict.result, VerdictResult::Pass);

    // Now falsify the claim and re-evaluate
    let falsification = Claim {
        id: ClaimId::new(ID_B).unwrap(),
        kind: ClaimKind::Falsification,
        target: vec![ClaimId::new(ID_A).unwrap()],
        assertion: AssertionForm::Annotation {
            text: "wrong".into(),
        },
        origin: Origin {
            kind: OriginKind::HarnessObserved,
            producer: Producer {
                id: "harness".into(),
                session_id: None,
                hermeticity: None,
            },
        },
        anchor: Anchor {
            tree: TREE.into(),
            strategy: AnchorStrategy::Multi,
            path: None,
            range: None,
            symbol: None,
            content_digest: None,
        },
        timestamp: 1_700_000_001,
        evidence: vec![],
        depends_on: vec![],
    };
    store.store_claim(&falsification).unwrap();
    let log_after = store.read_all_claims().unwrap();
    let verdict_after = evaluate(TREE, &policy, &log_after).unwrap();

    // Falsified premise → gate fails
    assert_eq!(verdict_after.result, VerdictResult::Fail);
    assert!(
        verdict_after
            .reasons
            .iter()
            .any(|r| r.contains("falsified premise"))
    );
}

// ---------------------------------------------------------------------------
// Interaction 4: Anchor Resolution ← Claim Evaluation (blast radius)
// ---------------------------------------------------------------------------

#[test]
fn interaction_4_anchor_resolution_blast_radius() {
    let claim_a = make_predicate_claim(ID_A, TREE, 1_700_000_000);
    let mut claim_b = make_predicate_claim(ID_B, TREE, 1_700_000_001);
    claim_b.depends_on = vec![ClaimId::new(ID_A).unwrap()];
    let mut claim_c = make_predicate_claim(ID_C, TREE, 1_700_000_002);
    claim_c.depends_on = vec![ClaimId::new(ID_B).unwrap()];

    let log = vec![claim_a.clone(), claim_b.clone(), claim_c.clone()];

    // Falsify A
    let falsification = Claim {
        id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000004")
            .unwrap(),
        kind: ClaimKind::Falsification,
        target: vec![ClaimId::new(ID_A).unwrap()],
        assertion: AssertionForm::Annotation {
            text: "wrong".into(),
        },
        origin: Origin {
            kind: OriginKind::HarnessObserved,
            producer: Producer {
                id: "harness".into(),
                session_id: None,
                hermeticity: None,
            },
        },
        anchor: Anchor {
            tree: TREE.into(),
            strategy: AnchorStrategy::Multi,
            path: None,
            range: None,
            symbol: None,
            content_digest: None,
        },
        timestamp: 1_700_000_003,
        evidence: vec![],
        depends_on: vec![],
    };
    let full_log = [log.as_slice(), std::slice::from_ref(&falsification)].concat();

    // Blast radius from A includes B and C
    let radius = elench_claim::blast_radius(&claim_a.id, &full_log);
    assert!(radius.contains(&claim_b.id));
    assert!(radius.contains(&claim_c.id));

    // A, B, and C are all falsified
    assert_eq!(
        elench_claim::compute_status(&claim_a.id, &full_log).unwrap(),
        elench_claim::ClaimStatus::Falsified
    );
    assert_eq!(
        elench_claim::compute_status(&claim_b.id, &full_log).unwrap(),
        elench_claim::ClaimStatus::Falsified
    );
    assert_eq!(
        elench_claim::compute_status(&claim_c.id, &full_log).unwrap(),
        elench_claim::ClaimStatus::Falsified
    );
}

// ---------------------------------------------------------------------------
// Interaction 5: Envelope Verification → Claim Emission
// ---------------------------------------------------------------------------

#[test]
fn interaction_5_envelope_verification_to_claim_emission() {
    let claim = make_predicate_claim(ID_A, TREE, 1_700_000_000);

    // Sign (elench-envelope)
    let key = SigningKey::generate(SignerEntity::Agent);
    let envelope = sign(&claim, &key);

    // Verify (elench-envelope) — extracts claim and signer
    let keys = vec![key.verifier()];
    let (extracted_claim, signer) = verify(&envelope, &keys).unwrap();

    // Validate the extracted claim (elench-claim) using the signer
    elench_claim::validate_claim(&extracted_claim, &signer, &[]).unwrap();

    // Store the verified claim (elench-store)
    let mut store = Store::new();
    let oid = store.store_claim(&extracted_claim).unwrap();
    assert!(store.has_claim(&oid));
}

// ---------------------------------------------------------------------------
// Interaction 6: Git Projection ← elench Store + Claim Log
// ---------------------------------------------------------------------------

#[test]
fn interaction_6_git_projection_from_store_and_log() {
    let claim_a = make_predicate_claim(
        ID_A,
        "1111111111111111111111111111111111111111111111111111111111111111",
        1_700_000_000,
    );
    let claim_b = make_predicate_claim(
        ID_B,
        "2222222222222222222222222222222222222222222222222222222222222222",
        1_700_000_001,
    );

    // Store claims
    let mut store = Store::new();
    store.store_claim(&claim_a).unwrap();
    store.store_claim(&claim_b).unwrap();

    // Read claims from store
    let log = store.read_all_claims().unwrap();
    assert_eq!(log.len(), 2);

    // Synthesize git projection (elench-projection) using claims from store
    let projection = synthesize(&log, &store).unwrap();
    assert_eq!(projection.commits.len(), 2);

    // Verify determinism: same log → same git output
    let projection2 = synthesize(&log, &store).unwrap();
    assert_eq!(git_log_oneline(&projection), git_log_oneline(&projection2));

    // Verify store is unchanged after projection (INV-19)
    assert_eq!(store.claim_count(), 2);
    assert_eq!(store.blob_count(), 0);
    assert_eq!(store.tree_count(), 0);
}

// ---------------------------------------------------------------------------
// Full end-to-end: emit → store → gate → blast → project
// ---------------------------------------------------------------------------

#[test]
fn e2e_full_pipeline() {
    // 1. Create and validate a claim
    let claim = make_predicate_claim(ID_A, TREE, 1_700_000_000);
    let signer = SignerIdentity {
        key_id: "agent-key".into(),
        entity: SignerEntity::Agent,
    };
    elench_claim::validate_claim(&claim, &signer, &[]).unwrap();

    // 2. Sign in DSSE envelope
    let key = SigningKey::generate(SignerEntity::Agent);
    let envelope = sign(&claim, &key);

    // 3. Verify the envelope
    let keys = vec![key.verifier()];
    let (verified_claim, _) = verify(&envelope, &keys).unwrap();

    // 4. Store the verified claim
    let mut store = Store::new();
    let _claim_oid = store.store_claim(&verified_claim).unwrap();

    // 5. Read claims back from store
    let log = store.read_all_claims().unwrap();
    assert_eq!(log.len(), 1);

    // 6. Evaluate the release gate
    let policy = Policy::permissive("default");
    let verdict = evaluate(TREE, &policy, &log).unwrap();
    assert_eq!(verdict.result, VerdictResult::Pass);

    // 7. Compute blast radius (empty — no dependents)
    let radius = elench_claim::blast_radius(&verified_claim.id, &log);
    assert_eq!(radius.len(), 0);

    // 8. Synthesize git projection
    let projection = synthesize(&log, &store).unwrap();
    assert_eq!(projection.commits.len(), 1);
    let git_log = git_log_oneline(&projection);
    assert!(git_log.contains("elench claim:"));

    // 9. Store is unchanged after projection
    assert_eq!(store.claim_count(), 1);
    assert_eq!(store.blob_count(), 0);
    assert_eq!(store.tree_count(), 0);

    // Claim OID is consistent
}
