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

// Claim (aggregate root)
pub struct Claim {
    pub id: ClaimId,
    pub kind: ClaimKind,
    pub target: Vec<ClaimId>,
    pub assertion: Assertion,
    pub origin: Origin,
    pub anchor: Anchor,
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

/// Validate a claim against emission rules (AGENTS.md).
/// INV-06, INV-07, INV-08, INV-12: reject violations.
pub fn validate_claim(claim: &Claim) -> Result<(), ValidationError>;
```

## elench-envelope

### Functions

```rust
/// Sign a claim payload in a DSSE envelope.
pub fn sign(claim: &Claim, signing_key: &SigningKey) -> Envelope;

/// Verify a DSSE envelope's signature and extract the claim.
/// INV-21: same format as build provenance.
pub fn verify(envelope: &Envelope) -> Result<Claim, EnvelopeError>;
```

## elench-store

### Functions

```rust
/// Store a claim in refs/claims/<type>/<id>.
/// INV-01, INV-19: append-only, parallel ref namespace.
pub fn store(claim: &Claim, repo: &Repository) -> Result<(), StoreError>;

/// Read all claims from refs/claims/.
pub fn read_all(repo: &Repository) -> Result<Vec<Claim>, StoreError>;

/// Read claims for a specific tree.
pub fn read_for_tree(tree: &str, repo: &Repository) -> Result<Vec<Claim>, StoreError>;
```

## elench-gate

### Functions

```rust
/// Evaluate the release gate for a tree under a policy.
/// INV-13: evaluable without build capability.
/// INV-14: live evaluation, not frozen.
pub fn evaluate(
    tree: &str,
    policy: &Policy,
    log: &[Claim],
) -> Result<Verdict, GateError>;

/// The four conditions from docs/release-policy.md.
/// Condition 4 (builder agreement) is unavailable unless E2 passes.
pub struct Verdict {
    pub result: VerdictResult,
    pub reasons: Vec<String>,
    pub tree: String,
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

- `elench emit` — create and sign a claim, store in refs/claims/
- `elench verify` — verify an envelope and validate the claim
- `elench status` — compute a claim's status by folding the log
- `elench gate` — evaluate the release gate for a tree
- `elench blast` — compute the blast radius from a claim
