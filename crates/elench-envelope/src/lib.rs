//! # elench-envelope
//!
//! DSSE envelopes carrying in-toto statements, signed with Ed25519.
//!
//! Agent claims and build provenance share the same envelope format,
//! signing path, and verification library (ADR-0003). This crate
//! handles envelope signing, verification, and the distinction between
//! signer identity (from the envelope) and producer identity (from the
//! claim payload), which is a different thing (R2).
//!
//! ## Crypto
//!
//! Signing uses Ed25519 via `ed25519-dalek`. Keys are:
//! - `SigningKey`: 32-byte private key, generates 64-byte signatures
//! - `VerifyingKey`: 32-byte public key, verifies 64-byte signatures
//!
//! Key IDs are SHA-256 of the public key (16 hex chars, configurable).
//! The PAE (pre-authentication encoding) is the DSSE v1 format:
//! `DSSEv1 <payload_type_len> <payload_type> <payload_len> <payload>`

use ed25519_dalek::{Signer, Verifier as EdVerifier};
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
// Signing key — Ed25519 via ed25519-dalek
// ---------------------------------------------------------------------------

/// An Ed25519 signing key. Carries the private key and the entity
/// type it belongs to. Used to sign DSSE envelopes.
///
/// INV-22: same format as build provenance. Agent claims and build
/// provenance use the same signing path.
#[derive(Debug, Clone)]
pub struct SigningKey {
    /// Key identifier (SHA-256 of public key, first 16 hex chars).
    pub key_id: String,
    /// The entity type this key belongs to.
    pub entity: SignerEntity,
    /// Ed25519 signing key (private).
    pub signing_key: ed25519_dalek::SigningKey,
}

impl SigningKey {
    /// Generate a new Ed25519 keypair with the given entity type.
    #[must_use]
    pub fn generate(entity: SignerEntity) -> Self {
        let mut rng = rand::rng();
        let signing_key = ed25519_dalek::SigningKey::generate(&mut rng);
        let public = signing_key.verifying_key();
        let key_id = key_id_from_public(&public);
        Self {
            key_id,
            entity,
            signing_key,
        }
    }

    /// Create a signing key from raw bytes (32 bytes).
    ///
    /// # Errors
    ///
    /// Returns [`EnvelopeError`] if the bytes are not 32 bytes.
    pub fn from_bytes(
        key_id: impl Into<String>,
        entity: SignerEntity,
        bytes: &[u8],
    ) -> Result<Self, EnvelopeError> {
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| EnvelopeError::InvalidKey("expected 32 bytes".into()))?;
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&arr);
        Ok(Self {
            key_id: key_id.into(),
            entity,
            signing_key,
        })
    }

    /// Serialize the private key to hex (64 chars).
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex_encode(&self.signing_key.to_bytes())
    }

    /// Deserialize a signing key from hex (64 chars).
    ///
    /// # Errors
    ///
    /// Returns [`EnvelopeError`] if the hex is invalid or not 32 bytes.
    pub fn from_hex(
        key_id: impl Into<String>,
        entity: SignerEntity,
        hex: &str,
    ) -> Result<Self, EnvelopeError> {
        let bytes =
            hex_decode(hex).ok_or_else(|| EnvelopeError::InvalidKey("invalid hex".into()))?;
        Self::from_bytes(key_id, entity, &bytes)
    }

    /// Derive the corresponding verifier (public key).
    #[must_use]
    pub fn verifier(&self) -> VerifyingKey {
        VerifyingKey {
            key_id: self.key_id.clone(),
            entity: self.entity.clone(),
            verifying_key: self.signing_key.verifying_key(),
        }
    }
}

impl PartialEq for SigningKey {
    fn eq(&self, other: &Self) -> bool {
        self.key_id == other.key_id
            && self.entity == other.entity
            && self.signing_key.to_bytes() == other.signing_key.to_bytes()
    }
}

impl Eq for SigningKey {}

/// A verifying key (public key). Used to verify DSSE envelope
/// signatures without needing the private key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyingKey {
    /// Key identifier (SHA-256 of public key, first 16 hex chars).
    pub key_id: String,
    /// The entity type this key belongs to.
    pub entity: SignerEntity,
    /// Ed25519 verifying key (public).
    pub verifying_key: ed25519_dalek::VerifyingKey,
}

impl VerifyingKey {
    /// Serialize the public key to hex (64 chars).
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex_encode(&self.verifying_key.to_bytes())
    }

    /// Deserialize a verifying key from hex (64 chars).
    ///
    /// # Errors
    ///
    /// Returns [`EnvelopeError`] if the hex is invalid or not 32 bytes.
    pub fn from_hex(
        key_id: impl Into<String>,
        entity: SignerEntity,
        hex: &str,
    ) -> Result<Self, EnvelopeError> {
        let bytes =
            hex_decode(hex).ok_or_else(|| EnvelopeError::InvalidKey("invalid hex".into()))?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| EnvelopeError::InvalidKey("expected 32 bytes".into()))?;
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&arr)
            .map_err(|e| EnvelopeError::InvalidKey(format!("invalid public key: {e}")))?;
        Ok(Self {
            key_id: key_id.into(),
            entity,
            verifying_key,
        })
    }
}

/// Compute a key ID from a public key (SHA-256, first 16 hex chars).
#[must_use]
fn key_id_from_public(public: &ed25519_dalek::VerifyingKey) -> String {
    let hash = Sha256::digest(public.to_bytes());
    hex_encode(&hash[..8])
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
// Sign and Verify — Ed25519 over DSSE PAE
// ---------------------------------------------------------------------------

/// Compute the DSSE v1 Pre-Authentication Encoding (PAE).
///
/// PAE = `DSSEv1 <payload_type_len> <payload_type> <payload_len> <payload>`
///
/// The payload is the hex-encoded in-toto Statement JSON.
#[must_use]
fn compute_pae(payload_type: &str, payload: &str) -> Vec<u8> {
    format!(
        "DSSEv1 {} {} {} {}",
        payload_type.len(),
        payload_type,
        payload.len(),
        payload
    )
    .into_bytes()
}

/// Sign a claim payload in a DSSE envelope using Ed25519.
///
/// The claim is wrapped in an in-toto Statement, hex-encoded, and
/// signed over the DSSE PAE using Ed25519.
///
/// INV-22: same format as build provenance. Agent claims and build
/// provenance use the same signing path and envelope.
#[must_use]
pub fn sign(claim: &Claim, signing_key: &SigningKey) -> Envelope {
    let statement = Statement {
        statement_type: "https://in-toto.io/Statement/v1".into(),
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

    let pae = compute_pae(PREDICATE_TYPE_AGENT, &payload_hex);
    let sig = signing_key.signing_key.sign(&pae);
    let sig_hex = hex_encode(&sig.to_bytes());

    Envelope {
        payload_type: PREDICATE_TYPE_AGENT.into(),
        payload: payload_hex,
        signatures: vec![Signature {
            keyid: signing_key.key_id.clone(),
            sig: sig_hex,
        }],
    }
}

/// Verify a DSSE envelope's Ed25519 signature and extract the claim.
///
/// Verification uses only the public key (`VerifyingKey`). No private
/// keys or secrets are needed — the verifier does not trust the
/// signer, only the math (INV-22, R2).
///
/// # Errors
///
/// Returns [`EnvelopeError`] if the signature is invalid, the signer
/// is unknown, the payload is corrupt, or the predicateType is not
/// recognised.
pub fn verify(
    envelope: &Envelope,
    keys: &[VerifyingKey],
) -> Result<(Claim, SignerIdentity), EnvelopeError> {
    // 1. Check predicateType is recognised
    if envelope.payload_type != PREDICATE_TYPE_AGENT
        && envelope.payload_type != PREDICATE_TYPE_BUILD
    {
        return Err(EnvelopeError::UnsupportedPredicateType(
            envelope.payload_type.clone(),
        ));
    }

    // 2. Extract signer identity from the key registry
    let sig = envelope
        .signatures
        .first()
        .ok_or(EnvelopeError::NoSignature)?;

    let key = keys
        .iter()
        .find(|k| k.key_id == sig.keyid)
        .ok_or_else(|| EnvelopeError::UnknownSigner(sig.keyid.clone()))?;

    let signer = SignerIdentity {
        key_id: key.key_id.clone(),
        entity: key.entity.clone(),
    };

    // 3. Verify Ed25519 signature over the PAE
    let sig_bytes = hex_decode(&sig.sig).ok_or(EnvelopeError::InvalidSignature)?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| EnvelopeError::InvalidSignature)?;
    let signature = ed25519_dalek::Signature::from_bytes(&sig_arr);

    let pae = compute_pae(&envelope.payload_type, &envelope.payload);
    key.verifying_key
        .verify(&pae, &signature)
        .map_err(|_| EnvelopeError::InvalidSignature)?;

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

    #[error("invalid key: {0}")]
    InvalidKey(String),
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
        SigningKey::generate(SignerEntity::Agent)
    }

    // --- Sign and verify round-trip ---

    #[test]
    fn scenario_sign_produces_envelope() {
        let claim = make_test_claim();
        let key = make_test_key();
        let envelope = sign(&claim, &key);
        assert_eq!(envelope.payload_type, PREDICATE_TYPE_AGENT);
        assert_eq!(envelope.signatures.len(), 1);
        // Key ID is derived from the public key (16 hex chars)
        assert_eq!(envelope.signatures[0].keyid.len(), 16);
        // Signature is 128 hex chars (64 bytes Ed25519)
        assert_eq!(envelope.signatures[0].sig.len(), 128);
    }

    #[test]
    fn scenario_verify_extracts_claim_and_signer() {
        let claim = make_test_claim();
        let key = make_test_key();
        let envelope = sign(&claim, &key);

        let keys = vec![key.verifier()];
        let (extracted_claim, signer) = verify(&envelope, &keys).unwrap();
        assert_eq!(extracted_claim.id, claim.id);
        assert_eq!(extracted_claim.kind, claim.kind);
        assert_eq!(signer.entity, SignerEntity::Agent);
    }

    #[test]
    fn scenario_verify_rejects_invalid_signature() {
        let claim = make_test_claim();
        let key = make_test_key();
        let mut envelope = sign(&claim, &key);
        // Tamper with the signature
        envelope.signatures[0].sig = "0".repeat(128);

        let keys = vec![key.verifier()];
        let result = verify(&envelope, &keys);
        assert_eq!(result, Err(EnvelopeError::InvalidSignature));
    }

    #[test]
    fn scenario_verify_rejects_unknown_signer() {
        let claim = make_test_claim();
        let key = make_test_key();
        let envelope = sign(&claim, &key);

        // No keys registered
        let keys: Vec<VerifyingKey> = vec![];
        let result = verify(&envelope, &keys);
        assert_eq!(
            result,
            Err(EnvelopeError::UnknownSigner(
                envelope.signatures[0].keyid.clone()
            ))
        );
    }

    #[test]
    fn scenario_verify_rejects_no_signature() {
        let claim = make_test_claim();
        let key = make_test_key();
        let mut envelope = sign(&claim, &key);
        envelope.signatures.clear();

        let keys = vec![key.verifier()];
        let result = verify(&envelope, &keys);
        assert_eq!(result, Err(EnvelopeError::NoSignature));
    }

    #[test]
    fn scenario_verify_rejects_wrong_key() {
        let claim = make_test_claim();
        let key = make_test_key();
        let envelope = sign(&claim, &key);

        // Different key (wrong public key for the signature)
        let wrong_key = SigningKey::generate(SignerEntity::Agent);
        let keys = vec![wrong_key.verifier()];
        // The key_id won't match, so it's UnknownSigner
        let result = verify(&envelope, &keys);
        assert_eq!(
            result,
            Err(EnvelopeError::UnknownSigner(
                envelope.signatures[0].keyid.clone()
            ))
        );
    }

    #[test]
    fn scenario_verify_rejects_unsupported_predicatetype() {
        let claim = make_test_claim();
        let key = make_test_key();
        let mut envelope = sign(&claim, &key);
        envelope.payload_type = "https://example.com/unknown".into();

        let keys = vec![key.verifier()];
        let result = verify(&envelope, &keys);
        assert_eq!(
            result,
            Err(EnvelopeError::UnsupportedPredicateType(
                "https://example.com/unknown".into()
            ))
        );
    }

    #[test]
    fn scenario_inv22_agent_and_build_same_envelope_format() {
        let claim = make_test_claim();

        let agent_key = SigningKey::generate(SignerEntity::Agent);
        let build_key = SigningKey::generate(SignerEntity::Harness);

        let agent_env = sign(&claim, &agent_key);
        let build_env = sign(&claim, &build_key);

        // Both use the same envelope structure (payloadType, payload, signatures)
        assert_eq!(agent_env.payload_type, PREDICATE_TYPE_AGENT);
        assert_eq!(build_env.payload_type, PREDICATE_TYPE_AGENT);
        assert_eq!(agent_env.signatures.len(), 1);
        assert_eq!(build_env.signatures.len(), 1);

        // Both can be verified with the same verify function
        let keys = vec![agent_key.verifier(), build_key.verifier()];
        let (agent_claim, agent_signer) = verify(&agent_env, &keys).unwrap();
        let (build_claim, build_signer) = verify(&build_env, &keys).unwrap();

        assert_eq!(agent_claim.id, build_claim.id);
        assert_eq!(agent_signer.entity, SignerEntity::Agent);
        assert_eq!(build_signer.entity, SignerEntity::Harness);
    }

    #[test]
    fn scenario_r2_signer_distinct_from_producer() {
        let claim = make_test_claim();
        let key = make_test_key();
        let envelope = sign(&claim, &key);

        let keys = vec![key.verifier()];
        let (extracted_claim, signer) = verify(&envelope, &keys).unwrap();

        // Signer (from envelope) != producer (from claim payload)
        assert_ne!(signer.key_id, extracted_claim.origin.producer.id);
    }

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
        let (extracted, _) = verify(&envelope, &keys).unwrap();
        match extracted.assertion {
            AssertionForm::Predicate { expression } => {
                assert_eq!(expression.language, "elench-predicate-v1");
                assert_eq!(expression.source, "exists(\"Cargo.toml\")");
            }
            AssertionForm::Annotation { .. } => panic!("expected predicate"),
        }
    }

    #[test]
    fn scenario_key_generation_produces_valid_keypair() {
        let key = SigningKey::generate(SignerEntity::Agent);
        let verifier = key.verifier();

        // Key ID should be 16 hex chars
        assert_eq!(key.key_id.len(), 16);
        assert!(key.key_id.chars().all(|c| c.is_ascii_hexdigit()));

        // Public key hex should be 64 chars (32 bytes)
        let pub_hex = verifier.to_hex();
        assert_eq!(pub_hex.len(), 64);

        // Private key hex should be 64 chars (32 bytes)
        let priv_hex = key.to_hex();
        assert_eq!(priv_hex.len(), 64);

        // Round-trip: from_hex should produce same key
        let restored = SigningKey::from_hex(&key.key_id, SignerEntity::Agent, &priv_hex).unwrap();
        assert_eq!(restored, key);
    }

    #[test]
    fn scenario_harness_signs_agent_claims_externally() {
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

        let harness_key = SigningKey::generate(SignerEntity::Harness);
        let envelope = sign(&claim, &harness_key);

        let keys = vec![harness_key.verifier()];
        let (extracted, signer) = verify(&envelope, &keys).unwrap();
        assert_eq!(signer.entity, SignerEntity::Harness);
        assert_eq!(extracted.origin.kind, OriginKind::AgentAsserted);
    }
}
