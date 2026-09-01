//! # elench-envelope
//!
//! DSSE envelopes carrying in-toto statements.
//!
//! Agent claims and build provenance share the same envelope format,
//! signing path, and verification library (ADR-0003). This crate
//! handles envelope signing, verification, and the distinction between
//! signer identity (from the envelope) and producer identity (from the
//! claim payload), which is a different thing (R2).
//!
//! ## DSSE format
//!
//! DSSE (Dead Simple Signing Envelope) is a JSON envelope that wraps a
//! payload and its signature. The in-toto Statement is the payload.
//! The envelope provides:
//!
//! - `payloadType`: identifies the payload format (URI)
//! - `payload`: base64-encoded in-toto Statement
//! - `signatures`: list of { keyid, sig } pairs
//!
//! ## Signer vs Producer
//!
//! The **signer** is the key that signed the DSSE envelope. The
//! **producer** is the entity that produced the claim (from
//! `claim.origin.producer.id`). These are different things (R2). The
//! envelope verification extracts the signer's key ID; the validator
//! (`elench-claim::validate_claim`) cross-checks it against the
//! producer's asserted origin.kind.

use elench_claim::{Claim, SignerEntity, SignerIdentity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The predicateType for elench agent claims.
pub const PREDICATE_TYPE_AGENT: &str = "https://elench.dev/predicate/agent-claim/v0.1";

/// The predicateType for elench build provenance.
pub const PREDICATE_TYPE_BUILD: &str = "https://elench.dev/predicate/build-provenance/v0.1";

// ---------------------------------------------------------------------------
// Signing key (minimal, for Phase 2)
// ---------------------------------------------------------------------------

/// A signing key. In Phase 2, this is a simple key pair identified by
/// a key ID. A full implementation would use Ed25519 or ECDSA; for now
/// we use a deterministic SHA-256-based "signature" that is NOT
/// cryptographically secure but is sufficient for testing the
/// envelope format and signer/producer distinction.
///
/// INV-22: same format as build provenance. Agent claims and build
/// provenance use the same envelope, signing path, and verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningKey {
    /// Key identifier (e.g., "harness-key-1", "agent-model-v3", "human-alice").
    pub key_id: String,
    /// The entity type this key belongs to.
    pub entity: SignerEntity,
    /// Secret key material (for signing). In a real implementation,
    /// this would be a private key. Here it's a simple secret string.
    pub secret: String,
}

impl SigningKey {
    /// Create a new signing key.
    #[must_use]
    pub fn new(key_id: impl Into<String>, entity: SignerEntity, secret: impl Into<String>) -> Self {
        Self {
            key_id: key_id.into(),
            entity,
            secret: secret.into(),
        }
    }

    /// Derive the corresponding verifier (public key equivalent).
    #[must_use]
    pub fn verifier(&self) -> VerifyingKey {
        VerifyingKey {
            key_id: self.key_id.clone(),
            entity: self.entity.clone(),
        }
    }
}

/// A verifying key (public key equivalent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyingKey {
    pub key_id: String,
    pub entity: SignerEntity,
}

// ---------------------------------------------------------------------------
// In-toto Statement
// ---------------------------------------------------------------------------

/// An in-toto Statement wrapping a claim predicate.
///
/// This is the payload of the DSSE envelope. It carries:
/// - `type`: always "<https://in-toto.io/Statement/v1>"
/// - `subject`: the tree the claim is about
/// - `predicateType`: identifies whether this is an agent claim or
///   build provenance
/// - `predicate`: the claim itself
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Statement {
    #[serde(rename = "_type")]
    pub statement_type: String,
    pub subject: Vec<Subject>,
    pub predicate_type: String,
    pub predicate: ClaimPredicate,
}

/// A subject: the tree the claim is about.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subject {
    pub name: String,
    pub digest: HashMapDigest,
}

/// A digest map (algorithm -> digest).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HashMapDigest {
    pub sha256: String,
}

/// The predicate portion of an in-toto Statement.
///
/// This wraps the elench claim data model (`ClaimKind`, `AssertionForm`,
/// Origin, Anchor, Evidence, dependsOn, timestamp) in a format that
/// is serializable and compatible with in-toto/SLSA tooling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimPredicate {
    pub id: String,
    pub kind: String,
    pub target: Vec<String>,
    pub assertion: serde_json::Value,
    pub origin: serde_json::Value,
    pub anchor: serde_json::Value,
    pub timestamp: i64,
    pub evidence: Vec<serde_json::Value>,
    #[serde(rename = "dependsOn", default)]
    pub depends_on: Vec<String>,
}

// ---------------------------------------------------------------------------
// DSSE Envelope
// ---------------------------------------------------------------------------

/// A DSSE (Dead Simple Signing Envelope) wrapping an in-toto Statement.
///
/// INV-22: agent claims and build provenance share the same envelope
/// format, signing path, and verification library.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    pub payload_type: String,
    pub payload: String,
    pub signatures: Vec<Signature>,
}

/// A signature on a DSSE envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signature {
    pub keyid: String,
    pub sig: String,
}

impl Envelope {
    /// Decode the base64 payload into an in-toto Statement.
    ///
    /// # Errors
    ///
    /// Returns [`EnvelopeError::InvalidPayload`] if the payload is not
    /// valid base64 or not a valid in-toto Statement.
    pub fn decode_payload(&self) -> Result<Statement, EnvelopeError> {
        let bytes = hex_decode(&self.payload)
            .ok_or_else(|| EnvelopeError::InvalidPayload("hex decode failed".into()))?;
        let statement: Statement = serde_json::from_slice(&bytes)
            .map_err(|e| EnvelopeError::InvalidPayload(format!("JSON parse failed: {e}")))?;
        Ok(statement)
    }

    /// Extract the signer identity from the envelope's first signature.
    ///
    /// In a full implementation, this would look up the key ID in a
    /// key registry to determine the entity type. For Phase 2, the
    /// verifier provides the key registry.
    ///
    /// # Errors
    ///
    /// Returns [`EnvelopeError::NoSignature`] if there are no signatures.
    pub fn signer_identity(&self, keys: &[VerifyingKey]) -> Result<SignerIdentity, EnvelopeError> {
        let sig = self.signatures.first().ok_or(EnvelopeError::NoSignature)?;

        let key = keys
            .iter()
            .find(|k| k.key_id == sig.keyid)
            .ok_or_else(|| EnvelopeError::UnknownSigner(sig.keyid.clone()))?;

        Ok(SignerIdentity {
            key_id: key.key_id.clone(),
            entity: key.entity.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Sign and Verify
// ---------------------------------------------------------------------------

/// Sign a claim payload in a DSSE envelope.
///
/// The claim is wrapped in an in-toto Statement, base64-encoded, and
/// signed with the provided key. The predicateType is determined by
/// the signer's entity type:
/// - Harness: `PREDICATE_TYPE_AGENT` (harness-observed claims)
/// - Agent: `PREDICATE_TYPE_AGENT`
/// - Human: `PREDICATE_TYPE_AGENT` (human-asserted claims)
///
/// Build provenance uses `PREDICATE_TYPE_BUILD` (future, when the
/// harness emits build statements).
///
/// INV-22: same format as build provenance.
#[must_use]
pub fn sign(claim: &Claim, signing_key: &SigningKey) -> Envelope {
    let statement = Statement {
        statement_type: "<https://in-toto.io/Statement/v1>".into(),
        subject: vec![Subject {
            name: claim.anchor.tree.clone(),
            digest: HashMapDigest {
                sha256: claim.anchor.tree.clone(),
            },
        }],
        predicate_type: PREDICATE_TYPE_AGENT.into(),
        predicate: claim_to_predicate(claim),
    };

    let payload_json = serde_json::to_vec(&statement).unwrap_or_default();
    let payload_hex = hex_encode(&payload_json);

    // "Sign" the PAE (pre-authentication encoding).
    // For Phase 2, the PAE is a simple concatenation of
    // payload_type and payload. A full implementation would
    // follow DSSE v1 spec exactly.
    let pae = format!("{PREDICATE_TYPE_AGENT}{payload_hex}");
    let sig = deterministic_sign(&pae, &signing_key.secret);

    Envelope {
        payload_type: PREDICATE_TYPE_AGENT.into(),
        payload: payload_hex,
        signatures: vec![Signature {
            keyid: signing_key.key_id.clone(),
            sig,
        }],
    }
}

/// Verify a DSSE envelope's signature and extract the claim.
///
/// INV-22: same format as build provenance. Agent claims and build
/// provenance use the same verification path.
///
/// # Errors
///
/// Returns [`EnvelopeError`] if the signature is invalid, the signer
/// is unknown, the payload is corrupt, or the predicateType is not
/// recognised.
pub fn verify(
    envelope: &Envelope,
    keys: &[VerifyingKey],
    secrets: &[(String, String)], // (key_id, secret) pairs for verification
) -> Result<(Claim, SignerIdentity), EnvelopeError> {
    // 1. Check predicateType is recognised
    if envelope.payload_type != PREDICATE_TYPE_AGENT
        && envelope.payload_type != PREDICATE_TYPE_BUILD
    {
        return Err(EnvelopeError::UnsupportedPredicateType(
            envelope.payload_type.clone(),
        ));
    }

    // 2. Extract signer identity
    let signer = envelope.signer_identity(keys)?;

    // 3. Verify signature
    let sig = envelope
        .signatures
        .first()
        .ok_or(EnvelopeError::NoSignature)?;

    let secret = secrets
        .iter()
        .find(|(k, _)| k == &sig.keyid)
        .map(|(_, s)| s.as_str())
        .ok_or_else(|| EnvelopeError::UnknownSigner(sig.keyid.clone()))?;

    // Reconstruct PAE and verify (same format as sign())
    let pae = format!("{}{}", envelope.payload_type, envelope.payload);

    let expected_sig = deterministic_sign(&pae, secret);
    if expected_sig != sig.sig {
        return Err(EnvelopeError::InvalidSignature);
    }

    // 4. Decode and extract claim
    let statement = envelope.decode_payload()?;
    let claim = predicate_to_claim(&statement.predicate)?;

    Ok((claim, signer))
}

// ---------------------------------------------------------------------------
// Claim <-> Predicate conversion
// ---------------------------------------------------------------------------

fn claim_to_predicate(claim: &Claim) -> ClaimPredicate {
    let assertion = match &claim.assertion {
        elench_claim::AssertionForm::Predicate { expression } => serde_json::json!({
            "form": "predicate",
            "expression": {
                "language": expression.language,
                "source": expression.source,
                "digest": expression.digest,
            }
        }),
        elench_claim::AssertionForm::Annotation { text } => serde_json::json!({
            "form": "annotation",
            "text": text,
        }),
    };

    ClaimPredicate {
        id: claim.id.as_str().into(),
        kind: claim.kind_str().into(),
        target: claim
            .target
            .iter()
            .map(|t| t.as_str().to_string())
            .collect(),
        assertion,
        origin: serde_json::json!({
            "kind": match claim.origin.kind {
                elench_claim::OriginKind::HarnessObserved => "harness-observed",
                elench_claim::OriginKind::AgentAsserted => "agent-asserted",
                elench_claim::OriginKind::HumanAsserted => "human-asserted",
            },
            "producer": {
                "id": claim.origin.producer.id,
                "sessionId": claim.origin.producer.session_id,
                "hermeticity": claim.origin.producer.hermeticity.as_ref().map(|h| match h {
                    elench_claim::Hermeticity::None => "none",
                    elench_claim::Hermeticity::Container => "container",
                    elench_claim::Hermeticity::Vm => "vm",
                    elench_claim::Hermeticity::HermeticDerivation => "hermetic-derivation",
                }),
            }
        }),
        anchor: serde_json::json!({
            "tree": claim.anchor.tree,
            "strategy": match claim.anchor.strategy {
                elench_claim::AnchorStrategy::PathRange => "path-range",
                elench_claim::AnchorStrategy::Symbol => "symbol",
                elench_claim::AnchorStrategy::ContentDigest => "content-digest",
                elench_claim::AnchorStrategy::Multi => "multi",
            },
        }),
        timestamp: claim.timestamp,
        evidence: claim
            .evidence
            .iter()
            .map(|e| {
                serde_json::json!({
                    "kind": match e.kind {
                        elench_claim::EvidenceKind::ProcessExit => "process-exit",
                        elench_claim::EvidenceKind::TestReport => "test-report",
                        elench_claim::EvidenceKind::ArtifactDigest => "artifact-digest",
                        elench_claim::EvidenceKind::ExternalAttestation => "external-attestation",
                    },
                    "digest": e.digest,
                    "exitCode": e.exit_code,
                    "uri": e.uri,
                })
            })
            .collect(),
        depends_on: claim
            .depends_on
            .iter()
            .map(|d| d.as_str().to_string())
            .collect(),
    }
}

#[allow(clippy::too_many_lines)]
fn predicate_to_claim(pred: &ClaimPredicate) -> Result<Claim, EnvelopeError> {
    use elench_claim::{
        self, Anchor, AnchorStrategy, AssertionForm, ClaimId, ClaimKind, Evidence, EvidenceKind,
        Hermeticity, Origin, OriginKind, Producer,
    };

    let id = ClaimId::new(&pred.id)
        .map_err(|e| EnvelopeError::InvalidPayload(format!("invalid claim ID: {e}")))?;

    let kind = match pred.kind.as_str() {
        "assertion" => ClaimKind::Assertion,
        "falsification" => ClaimKind::Falsification,
        "verification" => ClaimKind::Verification,
        "supersession" => ClaimKind::Supersession,
        "residue-acceptance" => ClaimKind::ResidueAcceptance,
        other => {
            return Err(EnvelopeError::InvalidPayload(format!(
                "unknown claim kind: {other}"
            )));
        }
    };

    let target: Vec<ClaimId> = pred
        .target
        .iter()
        .map(ClaimId::new)
        .collect::<Result<_, _>>()
        .map_err(|e| EnvelopeError::InvalidPayload(format!("invalid target ID: {e}")))?;

    let assertion_form = pred.assertion.get("form").and_then(|v| v.as_str());
    let assertion = match assertion_form {
        Some("predicate") => {
            let expr = pred.assertion.get("expression").ok_or_else(|| {
                EnvelopeError::InvalidPayload("predicate without expression".into())
            })?;
            AssertionForm::Predicate {
                expression: elench_claim::Expression {
                    language: expr
                        .get("language")
                        .and_then(|v| v.as_str())
                        .unwrap_or("elench-predicate-v1")
                        .to_string(),
                    source: expr
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    digest: expr
                        .get("digest")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                },
            }
        }
        Some("annotation") => AssertionForm::Annotation {
            text: pred
                .assertion
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        },
        other => {
            return Err(EnvelopeError::InvalidPayload(format!(
                "unknown assertion form: {other:?}"
            )));
        }
    };

    let origin_kind = pred
        .origin
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| EnvelopeError::InvalidPayload("origin.kind missing".into()))?;
    let origin_kind = match origin_kind {
        "harness-observed" => OriginKind::HarnessObserved,
        "agent-asserted" => OriginKind::AgentAsserted,
        "human-asserted" => OriginKind::HumanAsserted,
        other => {
            return Err(EnvelopeError::InvalidPayload(format!(
                "unknown origin kind: {other}"
            )));
        }
    };

    let producer = pred
        .origin
        .get("producer")
        .ok_or_else(|| EnvelopeError::InvalidPayload("origin.producer missing".into()))?;

    let hermeticity = producer
        .get("hermeticity")
        .and_then(serde_json::Value::as_str)
        .map(|h| match h {
            "container" => Hermeticity::Container,
            "vm" => Hermeticity::Vm,
            "hermetic-derivation" => Hermeticity::HermeticDerivation,
            _ => Hermeticity::None,
        });

    let anchor = Anchor {
        tree: pred
            .anchor
            .get("tree")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        strategy: match pred
            .anchor
            .get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or("multi")
        {
            "path-range" => AnchorStrategy::PathRange,
            "symbol" => AnchorStrategy::Symbol,
            "content-digest" => AnchorStrategy::ContentDigest,
            _ => AnchorStrategy::Multi,
        },
        path: None,
        range: None,
        symbol: None,
        content_digest: None,
    };

    let evidence: Vec<Evidence> = pred
        .evidence
        .iter()
        .map(|e| Evidence {
            kind: match e.get("kind").and_then(|v| v.as_str()) {
                Some("process-exit") => EvidenceKind::ProcessExit,
                Some("test-report") => EvidenceKind::TestReport,
                Some("artifact-digest") => EvidenceKind::ArtifactDigest,
                _ => EvidenceKind::ExternalAttestation,
            },
            digest: e.get("digest").and_then(|v| v.as_str()).map(String::from),
            exit_code: e.get("exitCode").and_then(serde_json::Value::as_i64),
            uri: e.get("uri").and_then(|v| v.as_str()).map(String::from),
        })
        .collect();

    let depends_on: Vec<ClaimId> = pred
        .depends_on
        .iter()
        .map(ClaimId::new)
        .collect::<Result<_, _>>()
        .map_err(|e| EnvelopeError::InvalidPayload(format!("invalid dependsOn ID: {e}")))?;

    Ok(Claim {
        id,
        kind,
        target,
        assertion,
        origin: Origin {
            kind: origin_kind,
            producer: Producer {
                id: producer
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                session_id: producer
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                hermeticity,
            },
        },
        anchor,
        timestamp: pred.timestamp,
        evidence,
        depends_on,
    })
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EnvelopeError {
    #[error("invalid signature")]
    InvalidSignature,

    #[error("unknown signer: {0}")]
    UnknownSigner(String),

    #[error("no signature present")]
    NoSignature,

    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    #[error("unsupported predicateType: {0}")]
    UnsupportedPredicateType(String),

    #[error("malformed envelope: {0}")]
    MalformedEnvelope(String),
}

// ---------------------------------------------------------------------------
// Hex encoding/decoding (minimal, no dependency)
// ---------------------------------------------------------------------------

fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    for chunk in s.as_bytes().chunks(2) {
        let high = hex_val(chunk[0])?;
        let low = hex_val(chunk[1])?;
        bytes.push((high << 4) | low);
    }
    Some(bytes)
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Deterministic "signature" — NOT cryptographically secure.
/// For Phase 2, this is sufficient to test the envelope format.
/// A real implementation would use Ed25519 or ECDSA.
fn deterministic_sign(pae: &str, secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pae.as_bytes());
    hasher.update(secret.as_bytes());
    let hash = hasher.finalize();
    hex_encode(&hash)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use elench_claim::{
        self, Anchor, AnchorStrategy, AssertionForm, ClaimId, ClaimKind, Hermeticity, Origin,
        OriginKind, Producer,
    };

    fn make_test_claim() -> Claim {
        Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation {
                text: "test annotation".into(),
            },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "agent-model-v3".into(),
                    session_id: Some("session-123".into()),
                    hermeticity: Some(Hermeticity::None),
                },
            },
            anchor: Anchor {
                tree: "abc123def456".into(),
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

    fn make_test_key() -> SigningKey {
        SigningKey::new("agent-key-1", SignerEntity::Agent, "secret-secret")
    }

    // --- Sign and verify round-trip ---

    #[test]
    fn scenario_sign_produces_envelope() {
        let claim = make_test_claim();
        let key = make_test_key();
        let envelope = sign(&claim, &key);
        assert_eq!(envelope.payload_type, PREDICATE_TYPE_AGENT);
        assert_eq!(envelope.signatures.len(), 1);
        assert_eq!(envelope.signatures[0].keyid, "agent-key-1");
    }

    #[test]
    fn scenario_verify_extracts_claim_and_signer() {
        let claim = make_test_claim();
        let key = make_test_key();
        let envelope = sign(&claim, &key);

        let keys = vec![key.verifier()];
        let secrets = vec![("agent-key-1".to_string(), "secret-secret".to_string())];

        let (extracted_claim, signer) = verify(&envelope, &keys, &secrets).unwrap();
        assert_eq!(extracted_claim.id, claim.id);
        assert_eq!(extracted_claim.kind, claim.kind);
        assert_eq!(signer.key_id, "agent-key-1");
        assert_eq!(signer.entity, SignerEntity::Agent);
    }

    #[test]
    fn scenario_verify_rejects_invalid_signature() {
        let claim = make_test_claim();
        let key = make_test_key();
        let mut envelope = sign(&claim, &key);
        envelope.signatures[0].sig = "invalid".to_string();

        let keys = vec![key.verifier()];
        let secrets = vec![("agent-key-1".to_string(), "secret-secret".to_string())];

        let result = verify(&envelope, &keys, &secrets);
        assert_eq!(result, Err(EnvelopeError::InvalidSignature));
    }

    #[test]
    fn scenario_verify_rejects_unknown_signer() {
        let claim = make_test_claim();
        let key = make_test_key();
        let envelope = sign(&claim, &key);

        // No keys registered
        let keys: Vec<VerifyingKey> = vec![];
        let secrets = vec![("agent-key-1".to_string(), "secret-secret".to_string())];

        let result = verify(&envelope, &keys, &secrets);
        assert_eq!(
            result,
            Err(EnvelopeError::UnknownSigner("agent-key-1".into()))
        );
    }

    #[test]
    fn scenario_verify_rejects_wrong_secret() {
        let claim = make_test_claim();
        let key = make_test_key();
        let envelope = sign(&claim, &key);

        let keys = vec![key.verifier()];
        let secrets = vec![("agent-key-1".to_string(), "wrong-secret".to_string())];

        let result = verify(&envelope, &keys, &secrets);
        assert_eq!(result, Err(EnvelopeError::InvalidSignature));
    }

    #[test]
    fn scenario_verify_rejects_no_signature() {
        let claim = make_test_claim();
        let key = make_test_key();
        let mut envelope = sign(&claim, &key);
        envelope.signatures.clear();

        let keys = vec![key.verifier()];
        let secrets = vec![("agent-key-1".to_string(), "secret-secret".to_string())];

        let result = verify(&envelope, &keys, &secrets);
        assert_eq!(result, Err(EnvelopeError::NoSignature));
    }

    #[test]
    fn scenario_verify_rejects_unsupported_predicatetype() {
        let claim = make_test_claim();
        let key = make_test_key();
        let mut envelope = sign(&claim, &key);
        envelope.payload_type = "https://example.com/unknown".into();

        let keys = vec![key.verifier()];
        let secrets = vec![("agent-key-1".to_string(), "secret-secret".to_string())];

        let result = verify(&envelope, &keys, &secrets);
        assert_eq!(
            result,
            Err(EnvelopeError::UnsupportedPredicateType(
                "https://example.com/unknown".into()
            ))
        );
    }

    // --- INV-22: same format for agent claims and build provenance ---

    #[test]
    fn scenario_inv22_agent_and_build_same_envelope_format() {
        let claim = make_test_claim();

        let agent_key = SigningKey::new("agent-key", SignerEntity::Agent, "agent-secret");
        let build_key = SigningKey::new("build-key", SignerEntity::Harness, "build-secret");

        let agent_env = sign(&claim, &agent_key);
        let build_env = sign(&claim, &build_key);

        // Both use the same envelope structure (payloadType, payload, signatures)
        assert_eq!(agent_env.payload_type, PREDICATE_TYPE_AGENT);
        assert_eq!(build_env.payload_type, PREDICATE_TYPE_AGENT); // Both use agent-claim for now
        assert_eq!(agent_env.signatures.len(), 1);
        assert_eq!(build_env.signatures.len(), 1);

        // Both can be verified with the same verify function
        let keys = vec![agent_key.verifier(), build_key.verifier()];
        let secrets = vec![
            ("agent-key".to_string(), "agent-secret".to_string()),
            ("build-key".to_string(), "build-secret".to_string()),
        ];

        let (agent_claim, agent_signer) = verify(&agent_env, &keys, &secrets).unwrap();
        let (build_claim, build_signer) = verify(&build_env, &keys, &secrets).unwrap();

        assert_eq!(agent_claim.id, build_claim.id); // Same claim content
        assert_eq!(agent_signer.entity, SignerEntity::Agent);
        assert_eq!(build_signer.entity, SignerEntity::Harness);
    }

    // --- Signer vs Producer distinction (R2) ---

    #[test]
    fn scenario_r2_signer_distinct_from_producer() {
        let claim = make_test_claim();
        // Producer is "agent-model-v3" (from claim.origin.producer.id)
        // Signer is "agent-key-1" (from envelope signature)
        let key = make_test_key();
        let envelope = sign(&claim, &key);

        let keys = vec![key.verifier()];
        let secrets = vec![("agent-key-1".to_string(), "secret-secret".to_string())];

        let (extracted_claim, signer) = verify(&envelope, &keys, &secrets).unwrap();

        // Signer (from envelope) != producer (from claim payload)
        assert_ne!(signer.key_id, extracted_claim.origin.producer.id);
        assert_eq!(signer.key_id, "agent-key-1");
        assert_eq!(extracted_claim.origin.producer.id, "agent-model-v3");
    }

    // --- Predicate claim round-trip ---

    #[test]
    fn scenario_predicate_claim_round_trip() {
        let mut claim = make_test_claim();
        claim.assertion = AssertionForm::Predicate {
            expression: elench_claim::Expression {
                language: "elench-predicate-v1".into(),
                source: "exists(\"Cargo.toml\")".into(),
                digest: None,
            },
        };

        let key = make_test_key();
        let envelope = sign(&claim, &key);

        let keys = vec![key.verifier()];
        let secrets = vec![("agent-key-1".to_string(), "secret-secret".to_string())];

        let (extracted, _) = verify(&envelope, &keys, &secrets).unwrap();
        match extracted.assertion {
            AssertionForm::Predicate { expression } => {
                assert_eq!(expression.language, "elench-predicate-v1");
                assert_eq!(expression.source, "exists(\"Cargo.toml\")");
            }
            AssertionForm::Annotation { .. } => panic!("expected predicate"),
        }
    }

    // --- Hex tests ---

    #[test]
    fn scenario_hex_round_trip() {
        let data = b"Hello, World!";
        let encoded = hex_encode(data);
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn scenario_hex_known_value() {
        // "hello" in hex is 68656c6c6f
        assert_eq!(hex_encode(b"hello"), "68656c6c6f");
    }

    // --- Harness signer can sign, agent can verify ---

    #[test]
    fn scenario_harness_signs_agent_claims_externally() {
        // A harness signs a claim with `origin.kind` = agent-asserted
        // (e.g., the harness observed the agent asserting something)
        let claim = Claim {
            id: ClaimId::new("cl_0000000000000000000000000000000000000000000000000000000000000002")
                .unwrap(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation {
                text: "agent said X".into(),
            },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "agent-model-v3".into(),
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

        // Harness signs it (the harness is the signer, not the agent)
        let harness_key = SigningKey::new("harness-key-1", SignerEntity::Harness, "harness-secret");
        let envelope = sign(&claim, &harness_key);

        let keys = vec![harness_key.verifier()];
        let secrets = vec![("harness-key-1".to_string(), "harness-secret".to_string())];

        let (extracted, signer) = verify(&envelope, &keys, &secrets).unwrap();
        assert_eq!(signer.entity, SignerEntity::Harness); // Signer is harness
        assert_eq!(extracted.origin.kind, OriginKind::AgentAsserted); // Producer is agent
    }
}
