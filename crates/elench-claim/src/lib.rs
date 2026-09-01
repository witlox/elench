//! # elench-claim
//!
//! Claim data model, status computation, and emission-rule validation.
//!
//! A claim is a signed assertion about a tree. Its status (passed,
//! falsified, unevaluated) is NOT stored — it is computed by folding
//! the append-only claim log (INV-04). This crate provides the data
//! structures matching `schema/claim.schema.json`, the log-folding
//! status computation, and validation of emission rules per AGENTS.md.
//!
//! The claim log IS the primary history (ADR-0001). There is no git
//! underneath; claims are stored in the content-addressed store
//! (`elench-store`).

use std::collections::HashSet;

use sha2::{Digest, Sha256};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Claim identifier
// ---------------------------------------------------------------------------

/// Claim identifier. SHA-256 hash with a `cl_` prefix (67 chars total).
/// Pattern: `^cl_[0-9a-f]{64}$`. Never reassigned (R1, INV-03).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClaimId(String);

impl ClaimId {
    /// Create a `ClaimId` from a string, validating the pattern.
    ///
    /// # Errors
    ///
    /// Returns [`ParseClaimIdError`] if the string does not match
    /// `^cl_[0-9a-f]{64}$`.
    pub fn new(s: impl Into<String>) -> Result<Self, ParseClaimIdError> {
        let s = s.into();
        if s.len() != 67 || !s.starts_with("cl_") || !s[3..].chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(ParseClaimIdError(s));
        }
        Ok(Self(s))
    }

    /// Compute the content address of a claim: SHA-256 of its canonical
    /// JSON serialization (all fields except `id`), prefixed with `cl_`.
    /// INV-28: two claims with identical content get the same OID.
    #[must_use]
    pub fn from_content(claim: &Claim) -> Self {
        let canonical = canonical_json(claim);
        let hash = Sha256::digest(canonical.as_bytes());
        Self(format!("cl_{}", hex_encode(&hash)))
    }

    /// Return the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ClaimId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for ClaimId {
    type Err = ParseClaimIdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[derive(Debug, Error)]
#[error("invalid claim id: {0}")]
pub struct ParseClaimIdError(pub String);

// ---------------------------------------------------------------------------
// Claim types
// ---------------------------------------------------------------------------

/// Claim kind (from `schema/claim.schema.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimKind {
    Assertion,
    Falsification,
    Verification,
    Supersession,
    ResidueAcceptance,
}

/// Assertion form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssertionForm {
    /// Machine-checkable, can gate. Must have an executable expression.
    Predicate { expression: Expression },
    /// Prose, searchable, cannot gate. Never read by policy.
    Annotation { text: String },
}

/// Predicate expression (ADR-0004: `elench-predicate-v1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    /// Must be `"elench-predicate-v1"` (or a known successor).
    pub language: String,
    /// DSL source string.
    pub source: String,
    /// SHA-256 of the source, for deduplication (INV-28).
    pub digest: Option<String>,
}

/// Origin: who produced this claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    pub kind: OriginKind,
    pub producer: Producer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginKind {
    HarnessObserved,
    AgentAsserted,
    HumanAsserted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Producer {
    pub id: String,
    pub session_id: Option<String>,
    pub hermeticity: Option<Hermeticity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hermeticity {
    None,
    Container,
    Vm,
    HermeticDerivation,
}

/// Anchor: how a claim points at code within a tree.
/// UNRESOLVED — E1 determines which strategy survives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    /// elench tree OID (ADR-0001). Not a git commit OID.
    pub tree: String,
    pub strategy: AnchorStrategy,
    pub path: Option<String>,
    pub range: Option<[i64; 2]>,
    pub symbol: Option<String>,
    pub content_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorStrategy {
    PathRange,
    Symbol,
    ContentDigest,
    Multi,
}

/// Evidence observed by the harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub digest: Option<String>,
    pub exit_code: Option<i64>,
    pub uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceKind {
    ProcessExit,
    TestReport,
    ArtifactDigest,
    ExternalAttestation,
}

/// A signed assertion about a tree (aggregate root).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub id: ClaimId,
    pub kind: ClaimKind,
    pub target: Vec<ClaimId>,
    pub assertion: AssertionForm,
    pub origin: Origin,
    pub anchor: Anchor,
    /// Unix epoch seconds. Set by producer at emission.
    /// Used by ADR-0007 for deterministic git commit synthesis.
    pub timestamp: i64,
    pub evidence: Vec<Evidence>,
    /// Premises. Transitive closure IS the blast radius.
    pub depends_on: Vec<ClaimId>,
}

/// Computed status (not stored). INV-04.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimStatus {
    /// No verification or falsification record targets this claim.
    Unevaluated,
    /// A verification record exists and no falsification has
    /// invalidated it.
    Passed,
    /// A falsification or supersession record has changed this
    /// claim's status.
    Falsified,
}

// ---------------------------------------------------------------------------
// Signer identity (for validate_claim)
// ---------------------------------------------------------------------------

/// Identity of the key that signed the DSSE envelope, with its known
/// entity type. The validator cross-checks this against
/// `claim.origin.kind` to prevent forgery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerIdentity {
    pub key_id: String,
    pub entity: SignerEntity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignerEntity {
    Harness,
    Agent,
    Human,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("origin.kind is required")]
    MissingOriginKind,

    #[error("agents cannot emit harness-observed records")]
    AgentEmitsHarnessObserved,

    #[error("only the harness may emit verification records")]
    NonHarnessEmitsVerification,

    #[error("predicate claims require an executable expression")]
    PredicateWithoutExpression,

    #[error("only humans may emit residue-acceptance records")]
    NonHumanEmitsResidueAcceptance,

    #[error("residue-acceptance must name specific gaps (non-empty target)")]
    ResidueAcceptanceWithoutTargets,

    #[error("falsification changes no status — target {0} already falsified")]
    FalsificationChangesNoStatus(String),

    #[error("dependsOn is empty — claim asserts it was reached from nothing (warning)")]
    EmptyDependsOn,

    #[error("cyclic dependency detected: {0}")]
    CyclicDependency(String),

    #[error("predicate expression parse error: {0}")]
    PredicateParseError(String),

    #[error("schema validation failed: {0}")]
    SchemaViolation(String),

    #[error("status computation error: {0}")]
    Status(#[from] StatusError),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StatusError {
    #[error("claim {0} not found in log")]
    ClaimNotFound(String),

    #[error("log is corrupt: {0}")]
    CorruptLog(String),

    #[error("cyclic dependency detected in targeting graph: {0}")]
    CyclicDependency(String),
}

// ---------------------------------------------------------------------------
// Validation (INV-05, INV-06, INV-07, INV-08, INV-11, INV-12, INV-29)
// ---------------------------------------------------------------------------

/// Validate a claim against emission rules (AGENTS.md).
///
/// - INV-05: `origin.kind` is present (enforced by type system).
/// - INV-06: agents cannot emit `harness-observed`.
/// - INV-07: only the harness emits `verification`.
/// - INV-08: predicate claims require an executable expression in
///   `elench-predicate-v1`.
/// - INV-11: a falsification that changes no status is rejected.
/// - INV-12: only humans emit `residue-acceptance`.
/// - INV-29: `dependsOn` must be acyclic.
///
/// # Errors
///
/// Returns [`ValidationError`] if any emission rule is violated.
pub fn validate_claim(
    claim: &Claim,
    signer: &SignerIdentity,
    log: &[Claim],
) -> Result<(), ValidationError> {
    // INV-06: agents cannot emit harness-observed records.
    if claim.origin.kind == OriginKind::HarnessObserved && signer.entity == SignerEntity::Agent {
        return Err(ValidationError::AgentEmitsHarnessObserved);
    }

    // INV-07: only the harness emits verification records.
    if claim.kind == ClaimKind::Verification && signer.entity != SignerEntity::Harness {
        return Err(ValidationError::NonHarnessEmitsVerification);
    }

    // INV-12: only humans emit residue-acceptance records.
    if claim.kind == ClaimKind::ResidueAcceptance && signer.entity != SignerEntity::Human {
        return Err(ValidationError::NonHumanEmitsResidueAcceptance);
    }

    // INV-12: residue-acceptance must name specific gaps (non-empty target).
    if claim.kind == ClaimKind::ResidueAcceptance && claim.target.is_empty() {
        return Err(ValidationError::ResidueAcceptanceWithoutTargets);
    }

    // INV-08: predicate claims require an executable expression.
    if let AssertionForm::Predicate { expression } = &claim.assertion {
        if expression.source.is_empty() {
            return Err(ValidationError::PredicateWithoutExpression);
        }
        if expression.language != "elench-predicate-v1" {
            return Err(ValidationError::SchemaViolation(format!(
                "unknown predicate language: {} (expected elench-predicate-v1)",
                expression.language
            )));
        }
        // Parse the expression to ensure it's valid DSL, not prose.
        elench_predicate::parse(&expression.source)
            .map_err(|e| ValidationError::PredicateParseError(e.to_string()))?;
    }

    // INV-11: a falsification that changes no status is rejected.
    // The falsification must change the target's status. If the target
    // is already falsified, the new falsification is noise.
    if claim.kind == ClaimKind::Falsification || claim.kind == ClaimKind::Supersession {
        for target_id in &claim.target {
            let target_status = compute_status(target_id, log)?;
            if target_status == ClaimStatus::Falsified {
                return Err(ValidationError::FalsificationChangesNoStatus(
                    target_id.to_string(),
                ));
            }
        }
    }

    // INV-29: dependsOn must be acyclic.
    // Check that the claim's dependsOn does not create a cycle.
    // For a new claim being validated, we check if any of its
    // dependsOn targets, transitively, depend on the new claim's id.
    if !claim.depends_on.is_empty() {
        for dep in &claim.depends_on {
            if dep == &claim.id {
                return Err(ValidationError::CyclicDependency(format!(
                    "{} depends on itself",
                    claim.id
                )));
            }
            // Check if the dependency transitively depends on the new claim.
            let mut visited = HashSet::new();
            visited.insert(claim.id.clone());
            if dep_transitively_depends_on(dep, &claim.id, log, &mut visited)? {
                return Err(ValidationError::CyclicDependency(format!(
                    "{} -> ... -> {} creates a cycle",
                    claim.id, dep
                )));
            }
        }
    }

    Ok(())
}

/// Check if `dep` transitively depends on `target` in the log.
/// Uses DFS with a visited set for cycle detection.
fn dep_transitively_depends_on(
    dep: &ClaimId,
    target: &ClaimId,
    log: &[Claim],
    visited: &mut HashSet<ClaimId>,
) -> Result<bool, ValidationError> {
    if dep == target {
        return Ok(true);
    }
    if !visited.insert(dep.clone()) {
        // Already visited — cycle in existing log (not caused by new claim)
        return Ok(false);
    }
    // Find dep in log and check its dependsOn
    if let Some(dep_claim) = log.iter().find(|c| &c.id == dep) {
        for grandchild in &dep_claim.depends_on {
            if dep_transitively_depends_on(grandchild, target, log, visited)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

// ---------------------------------------------------------------------------
// Status computation (INV-01, INV-02, INV-03, INV-04)
// ---------------------------------------------------------------------------

/// Compute a claim's status by folding the log.
///
/// INV-04: pure function, no stored status. The status is computed
/// from the claims that target this one.
///
/// - Default: `Unevaluated` (no verification or falsification targets it).
/// - `Passed`: a verification record targets it and no falsification
///   has invalidated that verification.
/// - `Falsified`: a falsification or supersession record targets it,
///   and the falsification/supersession itself has not been falsified.
///
/// If a verification record is itself falsified, the target reverts to
/// `Unevaluated`. If a falsification record is itself falsified, the
/// target reverts to its previous status.
///
/// INV-29: cycle detection via a visited set prevents infinite
/// recursion. If a cycle is detected in the targeting graph, returns
/// [`StatusError::CyclicDependency`].
///
/// # Errors
///
/// Returns [`StatusError`] if a cycle is detected in the targeting graph.
pub fn compute_status(claim_id: &ClaimId, log: &[Claim]) -> Result<ClaimStatus, StatusError> {
    let mut visited = HashSet::new();
    compute_status_inner(claim_id, log, &mut visited)
}

fn compute_status_inner(
    claim_id: &ClaimId,
    log: &[Claim],
    visited: &mut HashSet<ClaimId>,
) -> Result<ClaimStatus, StatusError> {
    // INV-29: cycle detection. If we've already visited this claim
    // in the current recursion, we have a cycle. Return Falsified
    // as a conservative default — a cycle in the targeting graph
    // means we cannot determine the true status.
    if !visited.insert(claim_id.clone()) {
        return Err(StatusError::CyclicDependency(claim_id.to_string()));
    }

    // Find all claims that target this one.
    let targeting: Vec<&Claim> = log.iter().filter(|c| c.target.contains(claim_id)).collect();

    if targeting.is_empty() {
        return Ok(ClaimStatus::Unevaluated);
    }

    // Check for active falsifications (falsification or supersession
    // that has not itself been falsified).
    let mut has_active_falsification = false;
    for c in &targeting {
        if c.kind == ClaimKind::Falsification || c.kind == ClaimKind::Supersession {
            let c_status = compute_status_inner(&c.id, log, visited)?;
            if c_status != ClaimStatus::Falsified {
                has_active_falsification = true;
                break;
            }
        }
    }

    if has_active_falsification {
        return Ok(ClaimStatus::Falsified);
    }

    // Check for active verifications (verification that has not been
    // falsified).
    let mut has_active_verification = false;
    for c in &targeting {
        if c.kind == ClaimKind::Verification {
            let c_status = compute_status_inner(&c.id, log, visited)?;
            if c_status != ClaimStatus::Falsified {
                has_active_verification = true;
                break;
            }
        }
    }

    if has_active_verification {
        return Ok(ClaimStatus::Passed);
    }

    Ok(ClaimStatus::Unevaluated)
}

// ---------------------------------------------------------------------------
// Blast radius (transitive dependsOn closure)
// ---------------------------------------------------------------------------

/// Compute the transitive `dependsOn` closure (blast radius) from a
/// falsified claim. Every claim that transitively depends on the
/// given claim is in the blast radius.
///
/// INV-29: cycle detection prevents infinite loops. If a cycle is
/// detected, it is reported but does not cause a panic.
///
/// # Errors
///
/// Returns [`ValidationError::CyclicDependency`] if a cycle is found
/// in the `dependsOn` graph that prevents termination.
#[must_use]
pub fn blast_radius(claim_id: &ClaimId, log: &[Claim]) -> Vec<ClaimId> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    collect_dependents(claim_id, log, &mut visited, &mut result);
    result
}

/// Collect all claims that transitively depend on `claim_id`.
fn collect_dependents(
    claim_id: &ClaimId,
    log: &[Claim],
    visited: &mut HashSet<ClaimId>,
    result: &mut Vec<ClaimId>,
) {
    if !visited.insert(claim_id.clone()) {
        return; // Already visited — cycle detected, stop.
    }

    // Find all claims that depend on this one.
    for c in log {
        if c.depends_on.contains(claim_id) && !result.contains(&c.id) {
            result.push(c.id.clone());
            collect_dependents(&c.id, log, visited, result);
        }
    }
}

// ---------------------------------------------------------------------------
// Canonical JSON (for INV-28: claim OID = SHA-256 of canonical JSON)
// ---------------------------------------------------------------------------

/// Produce a canonical JSON serialization of a claim, excluding the
/// `id` field. Used by [`ClaimId::from_content`] for content addressing.
#[must_use]
fn canonical_json(claim: &Claim) -> String {
    // Serialize using serde_json with sorted keys (BTreeMap-like).
    // For v1, we use a simple manual serialization to ensure
    // determinism: fields in a fixed order, keys sorted.
    let mut map = serde_json::Map::new();
    map.insert("kind".into(), serde_json::json!(claim.kind_str()));
    map.insert(
        "target".into(),
        serde_json::json!(claim.target.iter().map(ClaimId::as_str).collect::<Vec<_>>()),
    );
    map.insert("assertion".into(), claim.assertion_json());
    map.insert("origin".into(), claim.origin_json());
    map.insert("anchor".into(), claim.anchor_json());
    map.insert("timestamp".into(), serde_json::json!(claim.timestamp));
    map.insert(
        "evidence".into(),
        serde_json::json!(claim.evidence.iter().map(evidence_json).collect::<Vec<_>>()),
    );
    map.insert(
        "dependsOn".into(),
        serde_json::json!(
            claim
                .depends_on
                .iter()
                .map(ClaimId::as_str)
                .collect::<Vec<_>>()
        ),
    );

    // Sort keys for canonical form.
    let mut sorted_keys: Vec<&String> = map.keys().collect();
    sorted_keys.sort();
    let pairs: Vec<String> = sorted_keys
        .iter()
        .map(|k| format!("\"{}\":{}", k, map.get(*k).unwrap()))
        .collect();
    format!("{{{}}}", pairs.join(","))
}

impl Claim {
    #[must_use]
    pub fn kind_str(&self) -> &'static str {
        match self.kind {
            ClaimKind::Assertion => "assertion",
            ClaimKind::Falsification => "falsification",
            ClaimKind::Verification => "verification",
            ClaimKind::Supersession => "supersession",
            ClaimKind::ResidueAcceptance => "residue-acceptance",
        }
    }

    fn assertion_json(&self) -> serde_json::Value {
        match &self.assertion {
            AssertionForm::Predicate { expression } => serde_json::json!({
                "form": "predicate",
                "expression": {
                    "language": expression.language,
                    "source": expression.source,
                    "digest": expression.digest,
                }
            }),
            AssertionForm::Annotation { text } => serde_json::json!({
                "form": "annotation",
                "text": text,
            }),
        }
    }

    fn origin_json(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": match self.origin.kind {
                OriginKind::HarnessObserved => "harness-observed",
                OriginKind::AgentAsserted => "agent-asserted",
                OriginKind::HumanAsserted => "human-asserted",
            },
            "producer": {
                "id": self.origin.producer.id,
                "sessionId": self.origin.producer.session_id,
                "hermeticity": match &self.origin.producer.hermeticity {
                    Some(Hermeticity::None) => Some("none"),
                    Some(Hermeticity::Container) => Some("container"),
                    Some(Hermeticity::Vm) => Some("vm"),
                    Some(Hermeticity::HermeticDerivation) => Some("hermetic-derivation"),
                    None => None,
                },
            }
        })
    }

    fn anchor_json(&self) -> serde_json::Value {
        serde_json::json!({
            "tree": self.anchor.tree,
            "strategy": match self.anchor.strategy {
                AnchorStrategy::PathRange => "path-range",
                AnchorStrategy::Symbol => "symbol",
                AnchorStrategy::ContentDigest => "content-digest",
                AnchorStrategy::Multi => "multi",
            },
            "path": self.anchor.path,
            "range": self.anchor.range,
            "symbol": self.anchor.symbol,
            "contentDigest": self.anchor.content_digest,
        })
    }
}

/// Serialize a single evidence item to canonical JSON.
fn evidence_json(ev: &Evidence) -> serde_json::Value {
    serde_json::json!({
        "kind": match ev.kind {
            EvidenceKind::ProcessExit => "process-exit",
            EvidenceKind::TestReport => "test-report",
            EvidenceKind::ArtifactDigest => "artifact-digest",
            EvidenceKind::ExternalAttestation => "external-attestation",
        },
        "digest": ev.digest,
        "exitCode": ev.exit_code,
        "uri": ev.uri,
    })
}

// ---------------------------------------------------------------------------

#[must_use]
fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Helpers ---

    fn make_claim(id: &str, kind: ClaimKind, origin_kind: OriginKind) -> Claim {
        Claim {
            id: ClaimId::new(id).unwrap(),
            kind,
            target: vec![],
            assertion: AssertionForm::Annotation {
                text: "test claim".into(),
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
                tree: "abc123".into(),
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

    fn make_signer(entity: SignerEntity) -> SignerIdentity {
        SignerIdentity {
            key_id: "test-key".into(),
            entity,
        }
    }

    fn make_predicate_claim(id: &str, source: &str, origin_kind: OriginKind) -> Claim {
        let mut claim = make_claim(id, ClaimKind::Assertion, origin_kind);
        claim.assertion = AssertionForm::Predicate {
            expression: Expression {
                language: "elench-predicate-v1".into(),
                source: source.into(),
                digest: None,
            },
        };
        claim
    }

    // --- ClaimId tests ---

    #[test]
    fn scenario_claimid_valid_pattern() {
        let id =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000000");
        assert!(id.is_ok());
    }

    #[test]
    fn scenario_claimid_invalid_too_short() {
        let id = ClaimId::new("cl_abc");
        assert!(id.is_err());
    }

    #[test]
    fn scenario_claimid_invalid_no_prefix() {
        let id = ClaimId::new("0000000000000000000000000000000000000000000000000000000000000000");
        assert!(id.is_err());
    }

    #[test]
    fn scenario_claimid_invalid_non_hex() {
        let id =
            ClaimId::new("cl_g000000000000000000000000000000000000000000000000000000000000000");
        assert!(id.is_err());
    }

    // --- INV-05: origin.kind required ---

    #[test]
    fn scenario_inv05_origin_kind_present_by_construction() {
        // The type system enforces this: Origin has a `kind` field
        // that cannot be omitted. No runtime check needed — but we
        // test that the validator accepts a claim with origin.kind.
        let claim = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000001",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let signer = make_signer(SignerEntity::Agent);
        let result = validate_claim(&claim, &signer, &[]);
        assert!(result.is_ok());
    }

    // --- INV-06: agents cannot emit harness-observed ---

    #[test]
    fn scenario_inv06_agent_cannot_emit_harness_observed() {
        let claim = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000002",
            ClaimKind::Assertion,
            OriginKind::HarnessObserved,
        );
        let signer = make_signer(SignerEntity::Agent);
        let result = validate_claim(&claim, &signer, &[]);
        assert_eq!(result, Err(ValidationError::AgentEmitsHarnessObserved));
    }

    #[test]
    fn scenario_inv06_harness_can_emit_harness_observed() {
        let claim = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000003",
            ClaimKind::Assertion,
            OriginKind::HarnessObserved,
        );
        let signer = make_signer(SignerEntity::Harness);
        let result = validate_claim(&claim, &signer, &[]);
        assert!(result.is_ok());
    }

    // --- INV-07: only harness emits verification ---

    #[test]
    fn scenario_inv07_agent_cannot_emit_verification() {
        let claim = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000004",
            ClaimKind::Verification,
            OriginKind::AgentAsserted,
        );
        let signer = make_signer(SignerEntity::Agent);
        let result = validate_claim(&claim, &signer, &[]);
        assert_eq!(result, Err(ValidationError::NonHarnessEmitsVerification));
    }

    #[test]
    fn scenario_inv07_human_cannot_emit_verification() {
        let claim = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000005",
            ClaimKind::Verification,
            OriginKind::HumanAsserted,
        );
        let signer = make_signer(SignerEntity::Human);
        let result = validate_claim(&claim, &signer, &[]);
        assert_eq!(result, Err(ValidationError::NonHarnessEmitsVerification));
    }

    #[test]
    fn scenario_inv07_harness_can_emit_verification() {
        let mut claim = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000006",
            ClaimKind::Verification,
            OriginKind::HarnessObserved,
        );
        claim.target = vec![
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000099")
                .unwrap(),
        ];
        let signer = make_signer(SignerEntity::Harness);
        let result = validate_claim(&claim, &signer, &[]);
        assert!(result.is_ok());
    }

    // --- INV-08: predicate requires executable expression ---

    #[test]
    fn scenario_inv08_predicate_with_valid_expression_accepted() {
        let claim = make_predicate_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000007",
            "exists(\"Cargo.toml\")",
            OriginKind::AgentAsserted,
        );
        let signer = make_signer(SignerEntity::Agent);
        let result = validate_claim(&claim, &signer, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn scenario_inv08_predicate_without_expression_rejected() {
        let mut claim = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000008",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        claim.assertion = AssertionForm::Predicate {
            expression: Expression {
                language: "elench-predicate-v1".into(),
                source: String::new(),
                digest: None,
            },
        };
        let signer = make_signer(SignerEntity::Agent);
        let result = validate_claim(&claim, &signer, &[]);
        assert_eq!(result, Err(ValidationError::PredicateWithoutExpression));
    }

    #[test]
    fn scenario_inv08_prose_in_predicate_slot_rejected() {
        let mut claim = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000009",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        claim.assertion = AssertionForm::Predicate {
            expression: Expression {
                language: "elench-predicate-v1".into(),
                source: "Input validation is now handled correctly.".into(),
                digest: None,
            },
        };
        let signer = make_signer(SignerEntity::Agent);
        let result = validate_claim(&claim, &signer, &[]);
        assert!(matches!(
            result,
            Err(ValidationError::PredicateParseError(_))
        ));
    }

    // --- INV-12: only humans emit residue-acceptance ---

    #[test]
    fn scenario_inv12_agent_cannot_emit_residue_acceptance() {
        let mut claim = make_claim(
            "cl_000000000000000000000000000000000000000000000000000000000000000a",
            ClaimKind::ResidueAcceptance,
            OriginKind::AgentAsserted,
        );
        claim.target = vec![
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000099")
                .unwrap(),
        ];
        let signer = make_signer(SignerEntity::Agent);
        let result = validate_claim(&claim, &signer, &[]);
        assert_eq!(result, Err(ValidationError::NonHumanEmitsResidueAcceptance));
    }

    #[test]
    fn scenario_inv12_harness_cannot_emit_residue_acceptance() {
        let mut claim = make_claim(
            "cl_000000000000000000000000000000000000000000000000000000000000000b",
            ClaimKind::ResidueAcceptance,
            OriginKind::HarnessObserved,
        );
        claim.target = vec![
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000099")
                .unwrap(),
        ];
        let signer = make_signer(SignerEntity::Harness);
        let result = validate_claim(&claim, &signer, &[]);
        assert_eq!(result, Err(ValidationError::NonHumanEmitsResidueAcceptance));
    }

    #[test]
    fn scenario_inv12_human_can_emit_residue_acceptance() {
        let mut claim = make_claim(
            "cl_000000000000000000000000000000000000000000000000000000000000000c",
            ClaimKind::ResidueAcceptance,
            OriginKind::HumanAsserted,
        );
        claim.target = vec![
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000099")
                .unwrap(),
        ];
        let signer = make_signer(SignerEntity::Human);
        let result = validate_claim(&claim, &signer, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn scenario_inv12_residue_acceptance_empty_target_rejected() {
        let claim = make_claim(
            "cl_000000000000000000000000000000000000000000000000000000000000000d",
            ClaimKind::ResidueAcceptance,
            OriginKind::HumanAsserted,
        );
        // target is empty by default
        let signer = make_signer(SignerEntity::Human);
        let result = validate_claim(&claim, &signer, &[]);
        assert_eq!(
            result,
            Err(ValidationError::ResidueAcceptanceWithoutTargets)
        );
    }

    // --- INV-11: falsification that changes no status is rejected ---

    #[test]
    fn scenario_inv11_falsification_of_already_falsified_rejected() {
        let target = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000099",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let falsification1 = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000010")
                .unwrap(),
            kind: ClaimKind::Falsification,
            target: vec![target.id.clone()],
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
            anchor: target.anchor.clone(),
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        let log = vec![target, falsification1];

        // Now try to emit a second falsification of the same target
        let falsification2 = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000011")
                .unwrap(),
            kind: ClaimKind::Falsification,
            target: vec![
                ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000099")
                    .unwrap(),
            ],
            assertion: AssertionForm::Annotation {
                text: "also wrong".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "harness".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: make_claim(
                "cl_0000000000000000000000000000000000000000000000000000000000000099",
                ClaimKind::Assertion,
                OriginKind::AgentAsserted,
            )
            .anchor
            .clone(),
            timestamp: 1_700_000_002,
            evidence: vec![],
            depends_on: vec![],
        };
        let signer = make_signer(SignerEntity::Harness);
        let result = validate_claim(&falsification2, &signer, &log);
        assert_eq!(
            result,
            Err(ValidationError::FalsificationChangesNoStatus(
                "cl_0000000000000000000000000000000000000000000000000000000000000099".into()
            ))
        );
    }

    // --- INV-29: dependsOn acyclic ---

    #[test]
    fn scenario_inv29_self_reference_rejected() {
        let id =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000012")
                .unwrap();
        let mut claim = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000012",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        claim.depends_on = vec![id];
        let signer = make_signer(SignerEntity::Agent);
        let result = validate_claim(&claim, &signer, &[]);
        assert!(matches!(result, Err(ValidationError::CyclicDependency(_))));
    }

    #[test]
    fn scenario_inv29_mutual_dependency_rejected() {
        let id_a =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000013")
                .unwrap();
        let id_b =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000014")
                .unwrap();

        let claim_a = Claim {
            id: id_a.clone(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation { text: "a".into() },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "agent".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "t".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_000,
            evidence: vec![],
            depends_on: vec![id_b.clone()],
        };
        let log = vec![claim_a];

        // New claim B depends on A, which depends on B -> cycle
        let claim_b = Claim {
            id: id_b.clone(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation { text: "b".into() },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "agent".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "t".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![id_a.clone()],
        };
        let signer = make_signer(SignerEntity::Agent);
        let result = validate_claim(&claim_b, &signer, &log);
        assert!(matches!(result, Err(ValidationError::CyclicDependency(_))));
    }

    #[test]
    fn scenario_inv29_acyclic_dependency_accepted() {
        let id_a =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000015")
                .unwrap();
        let claim_a = Claim {
            id: id_a.clone(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation { text: "a".into() },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "agent".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "t".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_000,
            evidence: vec![],
            depends_on: vec![],
        };
        let log = vec![claim_a];

        let claim_b = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000016")
                .unwrap(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation { text: "b".into() },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "agent".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "t".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![id_a],
        };
        let signer = make_signer(SignerEntity::Agent);
        let result = validate_claim(&claim_b, &signer, &log);
        assert!(result.is_ok());
    }

    // --- Status computation (INV-01, INV-02, INV-03, INV-04) ---

    #[test]
    fn scenario_status_unevaluated_by_default() {
        let claim = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000017",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let log = vec![claim.clone()];
        assert_eq!(
            compute_status(&claim.id, &log).unwrap(),
            ClaimStatus::Unevaluated
        );
    }

    #[test]
    fn scenario_status_passed_after_verification() {
        let target = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000018",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let verification = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000019")
                .unwrap(),
            kind: ClaimKind::Verification,
            target: vec![target.id.clone()],
            assertion: AssertionForm::Annotation {
                text: "verified".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "harness".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: target.anchor.clone(),
            timestamp: 1_700_000_001,
            evidence: vec![Evidence {
                kind: EvidenceKind::ProcessExit,
                digest: None,
                exit_code: Some(0),
                uri: None,
            }],
            depends_on: vec![],
        };
        let log = vec![target.clone(), verification];
        assert_eq!(
            compute_status(&target.id, &log).unwrap(),
            ClaimStatus::Passed
        );
    }

    #[test]
    fn scenario_status_falsified_after_falsification() {
        let target = make_claim(
            "cl_000000000000000000000000000000000000000000000000000000000000001a",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let falsification = Claim {
            id: ClaimId::new("cl_000000000000000000000000000000000000000000000000000000000000001b")
                .unwrap(),
            kind: ClaimKind::Falsification,
            target: vec![target.id.clone()],
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
            anchor: target.anchor.clone(),
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        let log = vec![target.clone(), falsification];
        assert_eq!(
            compute_status(&target.id, &log).unwrap(),
            ClaimStatus::Falsified
        );
    }

    #[test]
    fn scenario_status_falsified_after_supersession() {
        let target = make_claim(
            "cl_000000000000000000000000000000000000000000000000000000000000001c",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let supersession = Claim {
            id: ClaimId::new("cl_000000000000000000000000000000000000000000000000000000000000001d")
                .unwrap(),
            kind: ClaimKind::Supersession,
            target: vec![target.id.clone()],
            assertion: AssertionForm::Annotation {
                text: "superseded".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "harness".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: target.anchor.clone(),
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        let log = vec![target.clone(), supersession];
        assert_eq!(
            compute_status(&target.id, &log).unwrap(),
            ClaimStatus::Falsified
        );
    }

    #[test]
    fn scenario_status_reverts_to_unevaluated_when_verification_falsified() {
        let target = make_claim(
            "cl_000000000000000000000000000000000000000000000000000000000000001e",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let verification = Claim {
            id: ClaimId::new("cl_000000000000000000000000000000000000000000000000000000000000001f")
                .unwrap(),
            kind: ClaimKind::Verification,
            target: vec![target.id.clone()],
            assertion: AssertionForm::Annotation {
                text: "verified".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "harness".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: target.anchor.clone(),
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        let falsification_of_verification = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000020")
                .unwrap(),
            kind: ClaimKind::Falsification,
            target: vec![verification.id.clone()],
            assertion: AssertionForm::Annotation {
                text: "verification was wrong".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "harness".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: target.anchor.clone(),
            timestamp: 1_700_000_002,
            evidence: vec![],
            depends_on: vec![],
        };
        let log = vec![target.clone(), verification, falsification_of_verification];
        // Target's verification was falsified -> reverts to unevaluated
        assert_eq!(
            compute_status(&target.id, &log).unwrap(),
            ClaimStatus::Unevaluated
        );
    }

    #[test]
    fn scenario_status_reverts_when_falsification_falsified() {
        let target = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000021",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let falsification = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000022")
                .unwrap(),
            kind: ClaimKind::Falsification,
            target: vec![target.id.clone()],
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
            anchor: target.anchor.clone(),
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        let falsification_of_falsification = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000023")
                .unwrap(),
            kind: ClaimKind::Falsification,
            target: vec![falsification.id.clone()],
            assertion: AssertionForm::Annotation {
                text: "falsification was wrong".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "harness".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: target.anchor.clone(),
            timestamp: 1_700_000_002,
            evidence: vec![],
            depends_on: vec![],
        };
        let log = vec![
            target.clone(),
            falsification,
            falsification_of_falsification,
        ];
        // The falsification was itself falsified -> target reverts to unevaluated
        assert_eq!(
            compute_status(&target.id, &log).unwrap(),
            ClaimStatus::Unevaluated
        );
    }

    #[test]
    fn scenario_status_reverts_to_passed_when_falsification_falsified() {
        // GAP-6: test revert to Passed (not just Unevaluated).
        // Target is verified (Passed). Falsification falsifies the target.
        // Then a second falsification falsifies the first one.
        // The target should revert to Passed (its previous status).
        let target = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000030",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let verification = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000031")
                .unwrap(),
            kind: ClaimKind::Verification,
            target: vec![target.id.clone()],
            assertion: AssertionForm::Annotation {
                text: "verified".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "harness".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: target.anchor.clone(),
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        let falsification = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000032")
                .unwrap(),
            kind: ClaimKind::Falsification,
            target: vec![target.id.clone()],
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
            anchor: target.anchor.clone(),
            timestamp: 1_700_000_002,
            evidence: vec![],
            depends_on: vec![],
        };
        let falsification_of_falsification = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000033")
                .unwrap(),
            kind: ClaimKind::Falsification,
            target: vec![falsification.id.clone()],
            assertion: AssertionForm::Annotation {
                text: "falsification was wrong".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "harness".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: target.anchor.clone(),
            timestamp: 1_700_000_003,
            evidence: vec![],
            depends_on: vec![],
        };
        let log = vec![
            target.clone(),
            verification,
            falsification,
            falsification_of_falsification,
        ];
        // Target was Passed -> falsified -> falsification falsified -> revert to Passed
        assert_eq!(
            compute_status(&target.id, &log).unwrap(),
            ClaimStatus::Passed
        );
    }

    #[test]
    fn scenario_compute_status_cycle_returns_error() {
        // GAP-1: compute_status with cyclic targeting returns error, not panic.
        let id_a =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000034")
                .unwrap();
        let id_b =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000035")
                .unwrap();

        let claim_a = Claim {
            id: id_a.clone(),
            kind: ClaimKind::Verification,
            target: vec![id_b.clone()],
            assertion: AssertionForm::Annotation {
                text: "a verifies b".into(),
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
                tree: "t".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_000,
            evidence: vec![],
            depends_on: vec![],
        };
        let claim_b = Claim {
            id: id_b.clone(),
            kind: ClaimKind::Verification,
            target: vec![id_a.clone()],
            assertion: AssertionForm::Annotation {
                text: "b verifies a".into(),
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
                tree: "t".into(),
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
        let log = vec![claim_a, claim_b];
        // Should return error, not panic
        let result = compute_status(&id_a, &log);
        assert!(
            matches!(result, Err(StatusError::CyclicDependency(_))),
            "expected CyclicDependency error, got {result:?}"
        );
    }

    #[test]
    fn scenario_compute_status_empty_log() {
        // GAP-7: empty log returns Unevaluated.
        let id =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000036")
                .unwrap();
        assert_eq!(compute_status(&id, &[]).unwrap(), ClaimStatus::Unevaluated);
    }

    // --- Blast radius ---

    #[test]
    fn scenario_blast_radius_no_dependents() {
        let claim = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000024",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let log = vec![claim.clone()];
        let radius = blast_radius(&claim.id, &log);
        assert!(radius.is_empty());
    }

    #[test]
    fn scenario_blast_radius_propagates_through_depends_on() {
        let id_a =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000025")
                .unwrap();
        let id_b =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000026")
                .unwrap();
        let id_c =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000027")
                .unwrap();

        let claim_a = Claim {
            id: id_a.clone(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation { text: "a".into() },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "x".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "t".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1,
            evidence: vec![],
            depends_on: vec![],
        };
        let claim_b = Claim {
            id: id_b.clone(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation { text: "b".into() },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "x".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "t".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 2,
            evidence: vec![],
            depends_on: vec![id_a.clone()],
        };
        let claim_c = Claim {
            id: id_c.clone(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation { text: "c".into() },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "x".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "t".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 3,
            evidence: vec![],
            depends_on: vec![id_b.clone()],
        };

        let log = vec![claim_a, claim_b, claim_c];
        let radius = blast_radius(&id_a, &log);
        assert!(radius.contains(&id_b));
        assert!(radius.contains(&id_c));
        assert_eq!(radius.len(), 2);
    }

    #[test]
    fn scenario_blast_radius_cycle_detected_no_panic() {
        let id_a =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000028")
                .unwrap();
        let id_b =
            ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000029")
                .unwrap();

        let claim_a = Claim {
            id: id_a.clone(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation { text: "a".into() },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "x".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "t".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1,
            evidence: vec![],
            depends_on: vec![id_b.clone()],
        };
        let claim_b = Claim {
            id: id_b.clone(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation { text: "b".into() },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "x".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "t".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 2,
            evidence: vec![],
            depends_on: vec![id_a.clone()],
        };

        let log = vec![claim_a, claim_b];
        // Should not panic — cycle detected by visited set
        let radius = blast_radius(&id_a, &log);
        assert!(radius.contains(&id_b));
    }

    // --- INV-28: content addressing ---

    #[test]
    fn scenario_inv28_identical_claims_same_oid() {
        let claim1 = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000000",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let claim2 = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000000",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let oid1 = ClaimId::from_content(&claim1);
        let oid2 = ClaimId::from_content(&claim2);
        assert_eq!(oid1, oid2, "identical claims must have the same OID");
    }

    #[test]
    fn scenario_inv28_different_claims_different_oid() {
        let claim1 = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000000",
            ClaimKind::Assertion,
            OriginKind::AgentAsserted,
        );
        let claim2 = make_claim(
            "cl_0000000000000000000000000000000000000000000000000000000000000000",
            ClaimKind::Assertion,
            OriginKind::HumanAsserted,
        );
        let oid1 = ClaimId::from_content(&claim1);
        let oid2 = ClaimId::from_content(&claim2);
        assert_ne!(oid1, oid2, "different claims must have different OIDs");
    }
}
