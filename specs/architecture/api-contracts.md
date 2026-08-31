# API Contracts

Interfaces between crates. Each contract is the minimal surface that
satisfies the spec. No implementation exists yet; these are the targets
the implementer builds toward.

## elench-claim

### Types

```rust
// Claim kind (from schema/claim.schema.json)
pub enum ClaimKind {
    Assertion,
    Falsification,
    Verification,
    Supersession,
    ResidueAcceptance,
}

// Assertion form
pub enum AssertionForm {
    Predicate { expression: Expression },
    Annotation { text: String },
}

// Expression (language undecided — ADR-0004)
pub struct Expression {
    pub language: String,
    pub source: String,
    pub digest: Option<String>,
}

// Origin
pub struct Origin {
    pub kind: OriginKind,
    pub producer: Producer,
}

pub enum OriginKind {
    HarnessObserved,
    AgentAsserted,
    HumanAsserted,
}

pub struct Producer {
    pub id: String,
    pub session_id: Option<String>,
    pub hermeticity: Option<Hermeticity>,
}

pub enum Hermeticity {
    None,
    Container,
    Vm,
    HermeticDerivation,
}

// Claim identifier. SHA-256 hash with `cl_` prefix (67 chars total).
// Pattern: ^cl_[0-9a-f]{64}$. Never reassigned (R1, INV-03).
pub struct ClaimId(String);

// Claim (aggregate root)
pub struct Claim {
    pub id: ClaimId,
    pub kind: ClaimKind,
    pub target: Vec<ClaimId>,
    pub assertion: Assertion,
    pub origin: Origin,
    pub anchor: Anchor,
    pub timestamp: i64,
    pub evidence: Vec<Evidence>,
    pub depends_on: Vec<ClaimId>,
}

// Computed status (not stored)
pub enum ClaimStatus {
    Unevaluated,
    Passed,
    Falsified,
}
```

### Functions

```rust
/// Compute a claim's status by folding the log.
/// INV-04: pure function, no stored status.
pub fn compute_status(claim_id: &ClaimId, log: &[Claim]) -> ClaimStatus;

/// Compute the transitive dependsOn closure (blast radius).
pub fn blast_radius(claim_id: &ClaimId, log: &[Claim]) -> Vec<ClaimId>;

/// Identity of the key that signed the DSSE envelope, with its known entity type.
/// The validator cross-checks this against claim.origin.kind to prevent forgery.
pub struct SignerIdentity {
    pub key_id: String,
    pub entity: SignerEntity,
}

pub enum SignerEntity {
    Harness,
    Agent,
    Human,
}

/// Validate a claim against emission rules (AGENTS.md).
/// INV-05: origin.kind required.
/// INV-06: reject if signer is agent but origin.kind = harness-observed.
/// INV-07: reject if signer is not harness but kind = verification.
/// INV-08: reject form=predicate without expression.
/// INV-11: reject falsification that changes no status (requires log).
/// INV-12: reject if signer is not human but kind = residue-acceptance.
pub fn validate_claim(claim: &Claim, signer: &SignerIdentity, log: &[Claim]) -> Result<(), ValidationError>;
```

## elench-envelope

### Functions

```rust
/// Sign a claim payload in a DSSE envelope.
pub fn sign(claim: &Claim, signing_key: &SigningKey) -> Envelope;

/// Verify a DSSE envelope's signature and extract the claim.
/// INV-22: same format as build provenance.
pub fn verify(envelope: &Envelope) -> Result<Claim, EnvelopeError>;
```

## elench-store

### Types

```rust
/// Content address of a blob, tree, or claim.
/// Blobs and trees: SHA-256 hash (64 hex chars, no prefix), identical to git SHA-256 OIDs.
/// Claims: `cl_` prefix + SHA-256 hash (see ClaimId).
/// The git projection is a passthrough for blobs and trees; only commits are synthesized.
pub struct Oid(String);

/// A blob: content-addressed byte array.
pub struct Blob {
    pub oid: Oid,
    pub data: Vec<u8>,
}

/// A tree entry: name + mode + OID, exactly like git.
pub struct TreeEntry {
    pub name: String,
    pub mode: u32,
    pub oid: Oid,
    pub kind: TreeEntryKind,
}

pub enum TreeEntryKind {
    Blob,
    Tree,
}

/// A tree: sorted entries, content-addressed. Hierarchical — each
/// directory is a separate tree object. Serialization matches git's
/// tree object format, making OIDs identical.
pub struct Tree {
    pub oid: Oid,
    pub entries: Vec<TreeEntry>,
}
```

### Functions

```rust
/// Store a claim in the content-addressed store.
/// INV-01: append-only, never updates.
/// INV-18: elench owns the store, no git underneath.
pub fn store_claim(claim: &Claim, store: &Store) -> Result<Oid, StoreError>;

/// Store a blob.
pub fn store_blob(data: &[u8], store: &Store) -> Result<Oid, StoreError>;

/// Store a tree.
pub fn store_tree(tree: &Tree, store: &Store) -> Result<Oid, StoreError>;

/// Read all claims from the store.
pub fn read_all_claims(store: &Store) -> Result<Vec<Claim>, StoreError>;

/// Read claims for a specific tree (by elench tree OID).
pub fn read_claims_for_tree(tree: &Oid, store: &Store) -> Result<Vec<Claim>, StoreError>;

/// Read a tree by OID.
pub fn read_tree(tree: &Oid, store: &Store) -> Result<Tree, StoreError>;

/// Read a blob by OID.
pub fn read_blob(blob: &Oid, store: &Store) -> Result<Vec<u8>, StoreError>;
```

## elench-gate

### Functions

```rust
/// Evaluate the release gate for a tree under a policy.
/// INV-13: evaluable without build capability.
/// INV-14: live evaluation, not frozen.
/// `tree` is an elench tree OID, not a git commit.
pub fn evaluate(
    tree: &Oid,
    policy: &Policy,
    log: &[Claim],
) -> Result<Verdict, GateError>;

/// The four conditions from docs/release-policy.md.
/// Condition 4 (builder agreement) is unavailable unless E2 passes.
pub struct Verdict {
    pub result: VerdictResult,
    pub reasons: Vec<String>,
    pub tree: Oid,
    pub policy: String,
}

pub enum VerdictResult {
    Pass,
    Fail,
}
```

## elench (binary)

The CLI wires the libraries together. No public API — it is a binary.
Commands (not yet implemented):

- `elench emit` — create and sign a claim, store in the content-addressed store
- `elench verify` — verify an envelope and validate the claim
- `elench status` — compute a claim's status by folding the log
- `elench gate` — evaluate the release gate for a tree
- `elench blast` — compute the blast radius from a claim
- `elench git` — materialize the git projection (ADR-0002, ADR-0007)
