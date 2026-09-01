//! # elench-gate
//!
//! Release gate evaluation — a predicate over claims, not a build.
//!
//! The gate is cheap and deterministic: a party with the claim log
//! and no compute can evaluate the release predicate and get the same
//! answer as anyone else (R3). The build is a separate, expensive
//! function of a tree. Keeping them apart is what lets an artifact's
//! acceptability be a live evaluation, not a frozen signature (R4).
//!
//! See `docs/release-policy.md` for the gate's four conditions:
//! 1. No falsified premise (blast radius from falsified claims)
//! 2. Bounded residue (unevaluated within policy, excess covered)
//! 3. Origin floor (load-bearing claims are harness-observed)
//! 4. Builder agreement (K independent producers — available per E2)
//!
//! The tree is an elench tree OID (ADR-0001), not a git commit.

use elench_claim::{Claim, ClaimId, ClaimKind, ClaimStatus, OriginKind};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Policy
// ---------------------------------------------------------------------------

/// Release policy. Determines whether a tree is releasable based on
/// the four conditions from `docs/release-policy.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// Human-readable name for this policy.
    pub name: String,
    /// Maximum number of unevaluated claims allowed without
    /// residue-acceptance (condition 2).
    pub max_unevaluated: usize,
    /// Whether to require harness-observed origin for load-bearing
    /// claims (condition 3).
    pub require_harness_origin: bool,
    /// Minimum number of independent builder signatures required
    /// (condition 4). 0 means condition 4 is not evaluated.
    pub min_builders: usize,
    /// Minimum hermeticity level for builders (condition 4).
    /// "none" = any, "container" = container or better, etc.
    pub min_hermeticity: HermeticityFloor,
}

/// Hermeticity floor for builder agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HermeticityFloor {
    /// Any hermeticity is accepted.
    Any,
    /// Container or better required.
    Container,
    /// VM or better required.
    Vm,
    /// Hermetic derivation required.
    HermeticDerivation,
}

impl Policy {
    /// Create a permissive policy (for testing).
    #[must_use]
    pub fn permissive(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            max_unevaluated: usize::MAX,
            require_harness_origin: false,
            min_builders: 0,
            min_hermeticity: HermeticityFloor::Any,
        }
    }

    /// Create a strict policy requiring harness origin and
    /// bounded residue.
    #[must_use]
    pub fn strict(name: impl Into<String>, max_unevaluated: usize) -> Self {
        Self {
            name: name.into(),
            max_unevaluated,
            require_harness_origin: true,
            min_builders: 0,
            min_hermeticity: HermeticityFloor::Any,
        }
    }
}

// ---------------------------------------------------------------------------
// Verdict
// ---------------------------------------------------------------------------

/// The result of evaluating a release gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// Pass or fail.
    pub result: VerdictResult,
    /// Failure reasons (empty if pass).
    pub reasons: Vec<String>,
    /// The elench tree OID evaluated.
    pub tree: String,
    /// The policy name.
    pub policy: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictResult {
    Pass,
    Fail,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GateError {
    #[error("falsified premise in blast radius: {0}")]
    FalsifiedPremise(String),

    #[error("unbounded residue: {0} > {1}")]
    UnboundedResidue(usize, usize),

    #[error("origin floor not met for claim: {0}")]
    OriginFloorNotMet(String),

    #[error("builder agreement not met: {0} < {1}")]
    BuilderAgreementNotMet(usize, usize),

    #[error("policy evaluation failed: {0}")]
    PolicyEvaluationFailed(String),
}

// ---------------------------------------------------------------------------
// Evaluate — the four conditions
// ---------------------------------------------------------------------------

/// Evaluate the release gate for a tree under a policy.
///
/// INV-13: evaluable without build capability. No process execution,
/// no network, no external state. Only the claim log and policy.
///
/// INV-14: live evaluation, not frozen. Called on demand; the verdict
/// is computed from the current claim log, not cached.
///
/// `tree` is an elench tree OID (ADR-0001), not a git commit.
///
/// # Errors
///
/// Returns [`GateError`] if any condition fails. The [`Verdict`]
/// contains all failure reasons (not just the first).
#[allow(clippy::too_many_lines)]
pub fn evaluate(tree: &str, policy: &Policy, log: &[Claim]) -> Result<Verdict, GateError> {
    let mut reasons = Vec::new();

    // Find all claims anchored to this tree.
    let tree_claims: Vec<&Claim> = log.iter().filter(|c| c.anchor.tree == tree).collect();

    if tree_claims.is_empty() {
        // Empty store: gate passes (no claims to evaluate).
        return Ok(Verdict {
            result: VerdictResult::Pass,
            reasons,
            tree: tree.to_string(),
            policy: policy.name.clone(),
        });
    }

    // Condition 1: No falsified premise.
    // No claim in the transitive dependsOn closure rooted at T's
    // claim set has status falsified.
    let mut falsified_premises = Vec::new();
    for claim in &tree_claims {
        // Check the claim itself
        let status =
            elench_claim::compute_status(&claim.id, log).unwrap_or(ClaimStatus::Unevaluated);
        if status == ClaimStatus::Falsified {
            falsified_premises.push(claim.id.as_str().to_string());
        }

        // Check transitive dependsOn closure
        for dep in &claim.depends_on {
            let dep_status =
                elench_claim::compute_status(dep, log).unwrap_or(ClaimStatus::Unevaluated);
            if dep_status == ClaimStatus::Falsified
                && !falsified_premises.contains(&dep.as_str().to_string())
            {
                falsified_premises.push(dep.as_str().to_string());
            }
        }
    }

    if !falsified_premises.is_empty() {
        for fp in &falsified_premises {
            reasons.push(format!("falsified premise: {fp}"));
        }
    }

    // Condition 2: Bounded residue.
    // Claims with status unevaluated are within P's allowance, and
    // each excess is covered by a residue-acceptance record.
    // Only non-residue-acceptance claims count as "residue" —
    // residue-acceptance claims are not load-bearing.
    let unevaluated_claims: Vec<&Claim> = tree_claims
        .iter()
        .filter(|c| {
            c.kind != ClaimKind::ResidueAcceptance
                && elench_claim::compute_status(&c.id, log).unwrap_or(ClaimStatus::Unevaluated)
                    == ClaimStatus::Unevaluated
        })
        .copied()
        .collect();

    // Find residue-acceptance records for this tree
    let residue_acceptances: Vec<&Claim> = tree_claims
        .iter()
        .filter(|c| c.kind == ClaimKind::ResidueAcceptance)
        .copied()
        .collect();

    // Collect all accepted claim IDs from residue-acceptance records
    let mut accepted_ids: Vec<&ClaimId> = Vec::new();
    for ra in &residue_acceptances {
        for target in &ra.target {
            accepted_ids.push(target);
        }
    }

    // Count unevaluated claims that are NOT covered by acceptance
    let uncovered_unevaluated: Vec<&&Claim> = unevaluated_claims
        .iter()
        .filter(|c| !accepted_ids.contains(&&c.id))
        .collect();

    if uncovered_unevaluated.len() > policy.max_unevaluated {
        reasons.push(format!(
            "unbounded residue: {} > {}",
            uncovered_unevaluated.len(),
            policy.max_unevaluated
        ));
    }

    // Condition 3: Origin floor.
    // Claims that P designates load-bearing have origin.kind =
    // harness-observed. Agent-asserted claims may inform, but P
    // should not let them alone carry a release.
    if policy.require_harness_origin {
        // Load-bearing claims are non-residue-acceptance claims with
        // form = predicate (annotations are never load-bearing, INV-09).
        for claim in &tree_claims {
            if claim.kind == ClaimKind::ResidueAcceptance {
                continue;
            }

            // Only predicates are load-bearing
            if !matches!(
                claim.assertion,
                elench_claim::AssertionForm::Predicate { .. }
            ) {
                continue;
            }

            // Check if this claim has an active status (not falsified)
            let status =
                elench_claim::compute_status(&claim.id, log).unwrap_or(ClaimStatus::Unevaluated);
            if status == ClaimStatus::Falsified {
                continue; // Already counted in condition 1
            }

            if claim.origin.kind != OriginKind::HarnessObserved {
                reasons.push(format!("origin floor not met for claim: {}", claim.id));
            }
        }
    }

    // Condition 4: Builder agreement.
    // K independent producers have signed statements with subject D
    // for tree T, each meeting P's hermeticity floor.
    if policy.min_builders > 0 {
        // Count distinct producers with verification claims for this tree,
        // each meeting the policy's hermeticity floor.
        let mut producer_ids: Vec<String> = Vec::new();
        for claim in &tree_claims {
            if claim.kind == ClaimKind::Verification {
                let status = elench_claim::compute_status(&claim.id, log)
                    .unwrap_or(ClaimStatus::Unevaluated);
                if status != ClaimStatus::Falsified {
                    // Check hermeticity floor (GAP-C1 fix)
                    let meets_floor = match &claim.origin.producer.hermeticity {
                        None => policy.min_hermeticity == HermeticityFloor::Any,
                        Some(elench_claim::Hermeticity::None) => {
                            policy.min_hermeticity == HermeticityFloor::Any
                        }
                        Some(elench_claim::Hermeticity::Container) => matches!(
                            policy.min_hermeticity,
                            HermeticityFloor::Any | HermeticityFloor::Container
                        ),
                        Some(elench_claim::Hermeticity::Vm) => matches!(
                            policy.min_hermeticity,
                            HermeticityFloor::Any
                                | HermeticityFloor::Container
                                | HermeticityFloor::Vm
                        ),
                        Some(elench_claim::Hermeticity::HermeticDerivation) => true,
                    };
                    if !meets_floor {
                        continue;
                    }
                    let pid = &claim.origin.producer.id;
                    if !producer_ids.contains(pid) {
                        producer_ids.push(pid.clone());
                    }
                }
            }
        }

        if producer_ids.len() < policy.min_builders {
            reasons.push(format!(
                "builder agreement not met: {} < {}",
                producer_ids.len(),
                policy.min_builders
            ));
        }
    }

    // Build verdict
    let result = if reasons.is_empty() {
        VerdictResult::Pass
    } else {
        VerdictResult::Fail
    };

    Ok(Verdict {
        result,
        reasons,
        tree: tree.to_string(),
        policy: policy.name.clone(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use elench_claim::{
        Anchor, AnchorStrategy, AssertionForm, Expression, Hermeticity, Origin, Producer,
    };

    fn make_claim(id: &str, kind: ClaimKind, origin_kind: OriginKind, tree: &str) -> Claim {
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
                    id: "producer".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: tree.into(),
                strategy: AnchorStrategy::PathRange,
                path: Some("src/main.rs".into()),
                range: Some([1, 10]),
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_000,
            evidence: vec![],
            depends_on: vec![],
        }
    }

    fn make_predicate_claim(id: &str, origin_kind: OriginKind, tree: &str) -> Claim {
        let mut claim = make_claim(id, ClaimKind::Assertion, origin_kind, tree);
        claim.assertion = AssertionForm::Predicate {
            expression: Expression {
                language: "elench-predicate-v1".into(),
                source: "exists(\"Cargo.toml\")".into(),
                digest: None,
            },
        };
        claim
    }

    const TREE: &str = "abc123def456";
    const ID_A: &str = "cl_0000000000000000000000000000000000000000000000000000000000000001";
    const ID_B: &str = "cl_0000000000000000000000000000000000000000000000000000000000000002";
    const ID_C: &str = "cl_0000000000000000000000000000000000000000000000000000000000000003";

    // --- Empty store: gate passes ---

    #[test]
    fn scenario_empty_store_gate_passes() {
        let policy = Policy::permissive("test");
        let verdict = evaluate(TREE, &policy, &[]).unwrap();
        assert_eq!(verdict.result, VerdictResult::Pass);
        assert!(verdict.reasons.is_empty());
    }

    // --- Condition 1: No falsified premise ---

    #[test]
    fn scenario_condition1_no_falsified_premise_passes() {
        let claim = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);
        let log = vec![claim];
        let policy = Policy::permissive("test");
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Pass);
    }

    #[test]
    fn scenario_condition1_falsified_premise_fails() {
        let target = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);
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
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        let log = vec![target, falsification];
        let policy = Policy::permissive("test");
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Fail);
        assert!(
            verdict
                .reasons
                .iter()
                .any(|r| r.contains("falsified premise"))
        );
    }

    #[test]
    fn scenario_condition1_falsified_premise_in_depends_on() {
        let claim_a = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);
        let mut claim_b = make_predicate_claim(ID_B, OriginKind::AgentAsserted, TREE);
        claim_b.depends_on = vec![ClaimId::new(ID_A).unwrap()];

        let falsification = Claim {
            id: ClaimId::new(ID_C).unwrap(),
            kind: ClaimKind::Falsification,
            target: vec![ClaimId::new(ID_A).unwrap()],
            assertion: AssertionForm::Annotation {
                text: "A is wrong".into(),
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
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };

        let log = vec![claim_a, claim_b, falsification];
        let policy = Policy::permissive("test");
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Fail);
        assert!(
            verdict
                .reasons
                .iter()
                .any(|r| r.contains("falsified premise"))
        );
    }

    // --- Condition 2: Bounded residue ---

    #[test]
    fn scenario_condition2_residue_within_bounds_passes() {
        let claims: Vec<Claim> = (1..=3)
            .map(|i| {
                let id = format!("cl_{i:064}");
                make_claim(&id, ClaimKind::Assertion, OriginKind::AgentAsserted, TREE)
            })
            .collect();
        let log = claims;
        let policy = Policy::strict("test", 3);
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Pass);
    }

    #[test]
    fn scenario_condition2_residue_exceeds_bounds_fails() {
        let claims: Vec<Claim> = (1..=5)
            .map(|i| {
                let id = format!("cl_{i:064}");
                make_claim(&id, ClaimKind::Assertion, OriginKind::AgentAsserted, TREE)
            })
            .collect();
        let log = claims;
        let policy = Policy::strict("test", 3);
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Fail);
        assert!(
            verdict
                .reasons
                .iter()
                .any(|r| r.contains("unbounded residue: 5 > 3"))
        );
    }

    #[test]
    fn scenario_condition2_excess_covered_by_acceptance_passes() {
        let mut claims: Vec<Claim> = (1..=5)
            .map(|i| {
                let id = format!("cl_{i:064}");
                make_claim(&id, ClaimKind::Assertion, OriginKind::AgentAsserted, TREE)
            })
            .collect();

        // Add residue-acceptance for the 2 excess claims
        let ra = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000006")
                .unwrap(),
            kind: ClaimKind::ResidueAcceptance,
            target: vec![
                ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000004")
                    .unwrap(),
                ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000005")
                    .unwrap(),
            ],
            assertion: AssertionForm::Annotation {
                text: "I accept these gaps".into(),
            },
            origin: Origin {
                kind: OriginKind::HumanAsserted,
                producer: Producer {
                    id: "human-alice".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: TREE.into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_010,
            evidence: vec![],
            depends_on: vec![],
        };
        claims.push(ra);

        let policy = Policy::strict("test", 3);
        let verdict = evaluate(TREE, &policy, &claims).unwrap();
        assert_eq!(verdict.result, VerdictResult::Pass);
    }

    // --- Condition 3: Origin floor ---

    #[test]
    fn scenario_condition3_agent_predicate_fails_strict_policy() {
        let claim = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);
        let log = vec![claim];
        let policy = Policy::strict("test", 100);
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Fail);
        assert!(verdict.reasons.iter().any(|r| r.contains("origin floor")));
    }

    #[test]
    fn scenario_condition3_harness_predicate_passes_strict_policy() {
        let claim = make_predicate_claim(ID_A, OriginKind::HarnessObserved, TREE);
        let log = vec![claim];
        let policy = Policy::strict("test", 100);
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Pass);
    }

    #[test]
    fn scenario_condition3_annotation_not_load_bearing() {
        // Annotations are never load-bearing (INV-09). Even with
        // require_harness_origin, an agent-asserted annotation should
        // not trigger the origin floor.
        let claim = make_claim(ID_A, ClaimKind::Assertion, OriginKind::AgentAsserted, TREE);
        let log = vec![claim];
        let policy = Policy::strict("test", 100);
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        // Annotation is unevaluated (within bounds) and not load-bearing
        assert_eq!(verdict.result, VerdictResult::Pass);
    }

    #[test]
    fn scenario_condition3_permissive_policy_allows_agent() {
        let claim = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);
        let log = vec![claim];
        let policy = Policy::permissive("test");
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Pass);
    }

    // --- Condition 4: Builder agreement ---

    #[test]
    fn scenario_condition4_no_builders_required_passes() {
        let claim = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);
        let log = vec![claim];
        let policy = Policy {
            min_builders: 0,
            ..Policy::permissive("test")
        };
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Pass);
    }

    #[test]
    fn scenario_condition4_insufficient_builders_fails() {
        let claim = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);
        let log = vec![claim];
        let policy = Policy {
            min_builders: 2,
            ..Policy::permissive("test")
        };
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Fail);
        assert!(
            verdict
                .reasons
                .iter()
                .any(|r| r.contains("builder agreement not met: 0 < 2"))
        );
    }

    #[test]
    fn scenario_condition4_sufficient_builders_passes() {
        let target = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);

        let builder1 = Claim {
            id: ClaimId::new(ID_B).unwrap(),
            kind: ClaimKind::Verification,
            target: vec![ClaimId::new(ID_A).unwrap()],
            assertion: AssertionForm::Annotation {
                text: "built by 1".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "builder-1".into(),
                    session_id: None,
                    hermeticity: Some(Hermeticity::Container),
                },
            },
            anchor: Anchor {
                tree: TREE.into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        let builder2 = Claim {
            id: ClaimId::new(ID_C).unwrap(),
            kind: ClaimKind::Verification,
            target: vec![ClaimId::new(ID_A).unwrap()],
            assertion: AssertionForm::Annotation {
                text: "built by 2".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "builder-2".into(),
                    session_id: None,
                    hermeticity: Some(Hermeticity::Container),
                },
            },
            anchor: Anchor {
                tree: TREE.into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_002,
            evidence: vec![],
            depends_on: vec![],
        };

        let log = vec![target, builder1, builder2];
        let policy = Policy {
            min_builders: 2,
            ..Policy::permissive("test")
        };
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Pass);
    }

    // --- Live evaluation (INV-14) ---

    #[test]
    fn scenario_live_evaluation_status_changes_after_falsification() {
        let target = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);

        // First evaluation: passes
        let log1 = vec![target.clone()];
        let policy = Policy::permissive("test");
        let verdict1 = evaluate(TREE, &policy, &log1).unwrap();
        assert_eq!(verdict1.result, VerdictResult::Pass);

        // After falsification: fails
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
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        let log2 = vec![target, falsification];
        let verdict2 = evaluate(TREE, &policy, &log2).unwrap();
        assert_eq!(verdict2.result, VerdictResult::Fail);
        // No byte changed, no re-signing — the verdict is computed
        // live from the current claim log.
    }

    // --- Multiple conditions failing simultaneously ---

    #[test]
    fn scenario_multiple_conditions_fail() {
        let claim = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);
        let log = vec![claim];
        let policy = Policy {
            max_unevaluated: 0,
            require_harness_origin: true,
            min_builders: 1,
            ..Policy::permissive("test")
        };
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Fail);
        // Should have reasons for condition 2 (residue), 3 (origin), and 4 (builder)
        assert!(
            verdict
                .reasons
                .iter()
                .any(|r| r.contains("unbounded residue")),
            "expected residue reason"
        );
        assert!(
            verdict.reasons.iter().any(|r| r.contains("origin floor")),
            "expected origin reason"
        );
        assert!(
            verdict
                .reasons
                .iter()
                .any(|r| r.contains("builder agreement")),
            "expected builder reason"
        );
    }

    // --- Different tree: not affected ---

    #[test]
    fn scenario_different_tree_not_affected() {
        let claim_tree_a = make_predicate_claim(ID_A, OriginKind::AgentAsserted, "tree_a");
        let claim_tree_b = make_predicate_claim(ID_B, OriginKind::HarnessObserved, "tree_b");
        let log = vec![claim_tree_a, claim_tree_b];

        // Evaluate tree_b with strict policy. tree_a's agent-asserted
        // claim should NOT affect tree_b's verdict — tree_b's claim
        // is harness-observed, so it passes.
        let policy = Policy::strict("test", 100);
        let verdict = evaluate("tree_b", &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Pass);
        assert!(verdict.reasons.is_empty());
    }

    // --- Hermeticity floor (GAP-C1) ---

    #[test]
    fn scenario_hermeticity_floor_rejects_below_minimum() {
        let target = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);
        let builder_none = Claim {
            id: ClaimId::new(ID_B).unwrap(),
            kind: ClaimKind::Verification,
            target: vec![ClaimId::new(ID_A).unwrap()],
            assertion: AssertionForm::Annotation {
                text: "built".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "builder-none".into(),
                    session_id: None,
                    hermeticity: Some(Hermeticity::None),
                },
            },
            anchor: Anchor {
                tree: TREE.into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        let log = vec![target, builder_none];
        let policy = Policy {
            min_builders: 1,
            min_hermeticity: HermeticityFloor::Container,
            ..Policy::permissive("test")
        };
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Fail);
        assert!(
            verdict
                .reasons
                .iter()
                .any(|r| r.contains("builder agreement not met: 0 < 1")),
            "expected hermeticity floor rejection"
        );
    }

    #[test]
    fn scenario_hermeticity_floor_accepts_at_minimum() {
        let target = make_predicate_claim(ID_A, OriginKind::AgentAsserted, TREE);
        let builder_container = Claim {
            id: ClaimId::new(ID_B).unwrap(),
            kind: ClaimKind::Verification,
            target: vec![ClaimId::new(ID_A).unwrap()],
            assertion: AssertionForm::Annotation {
                text: "built".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "builder-container".into(),
                    session_id: None,
                    hermeticity: Some(Hermeticity::Container),
                },
            },
            anchor: Anchor {
                tree: TREE.into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        let log = vec![target, builder_container];
        let policy = Policy {
            min_builders: 1,
            min_hermeticity: HermeticityFloor::Container,
            ..Policy::permissive("test")
        };
        let verdict = evaluate(TREE, &policy, &log).unwrap();
        assert_eq!(verdict.result, VerdictResult::Pass);
    }
}
