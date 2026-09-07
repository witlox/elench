# Error Taxonomy

Error types by crate, with the invariant or failure mode each addresses.
Library code uses `thiserror`; the binary may use `anyhow`.

## Error categories

| Category | Meaning | Caller action |
|---|---|---|
| **Permanent** | Cannot succeed; data loss, corruption, or configuration error | Report to user/admin; do not retry |
| **Retriable** | Transient failure (I/O, network); may succeed on retry | Retry with backoff |
| **Security** | Authentication or authorization failure | Deny and audit |
| **Validation** | Claim or envelope rejected by emission rules | Agent reformulates and re-emits |

## elench-claim

```rust
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("origin.kind is required")]
    MissingOriginKind,                           // INV-05 — Validation

    #[error("agents cannot emit harness-observed records")]
    AgentEmitsHarnessObserved,                   // INV-06 — Security

    #[error("only the harness may emit verification records")]
    NonHarnessEmitsVerification,                 // INV-07 — Security

    #[error("predicate claims require an executable expression")]
    PredicateWithoutExpression,                  // INV-08 — Validation

    #[error("only humans may emit residue-acceptance records")]
    NonHumanEmitsResidueAcceptance,              // INV-12 — Security

    #[error("residue-acceptance must name specific gaps (non-empty target)")]
    ResidueAcceptanceWithoutTargets,             // INV-12 — Validation

    #[error("dependsOn is empty — claim asserts it was reached from nothing (warning)")]
    EmptyDependsOn,                              // GUIDELINE (was INV-10, downgraded) — Validation

    #[error("cyclic dependency detected: {0}")]
    CyclicDependency(String),                    // INV-29 — Permanent

    #[error("schema validation failed: {0}")]
    SchemaViolation(String),                     // general — Validation
}

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("claim {0} not found in log")]
    ClaimNotFound(String),                       // Permanent

    #[error("log is corrupt: {0}")]
    CorruptLog(String),                          // Permanent
}
```

## elench-envelope

```rust
#[derive(Debug, thiserror::Error)]
pub enum EnvelopeError {
    #[error("invalid signature")]
    InvalidSignature,                            // Security

    #[error("unknown signer: {0}")]
    UnknownSigner(String),                       // Security

    #[error("no signature present")]
    NoSignature,                                 // Validation

    #[error("invalid payload: {0}")]
    InvalidPayload(String),                      // Validation

    #[error("unsupported predicateType: {0}")]
    UnsupportedPredicateType(String),            // Validation

    #[error("malformed envelope: {0}")]
    MalformedEnvelope(String),                   // Validation

    #[error("invalid key: {0}")]
    InvalidKey(String),                          // Validation
}
```

## elench-store

```rust
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("invalid OID: {0}")]
    InvalidOid(String),                          // Validation

    #[error("object already exists with different content: {oid} ({reason})")]
    ObjectExists { oid: String, reason: String }, // INV-01 — Permanent

    #[error("content addressing violation: {oid} ({reason})")]
    ContentAddressingViolation { oid: String, reason: String }, // INV-25 — Permanent

    #[error("object not found: {0}")]
    ObjectNotFound(String),                      // Permanent

    #[error("store is corrupt: {0}")]
    CorruptStore(String),                        // Permanent

    #[error("I/O error")]
    Io,                                          // Retriable (wraps std::io::Error)
}
```

## elench-gate

```rust
#[derive(Debug, thiserror::Error)]
pub enum GateError {
    #[error("falsified premise in blast radius: {0}")]
    FalsifiedPremise(String),                    // release-policy condition 1 — Permanent

    #[error("unbounded residue: {0} > {1}")]
    UnboundedResidue(usize, usize),              // release-policy condition 2 — Permanent

    #[error("origin floor not met for claim: {0}")]
    OriginFloorNotMet(String),                   // release-policy condition 3 — Security

    #[error("builder agreement not met: {0} < {1}")]
    BuilderAgreementNotMet(usize, usize),        // release-policy condition 4 — Permanent

    #[error("policy evaluation failed: {0}")]
    PolicyEvaluationFailed(String),              // general — Permanent
}
```

## elench-anchor

```rust
#[derive(Debug, thiserror::Error)]
pub enum AnchorError {
    #[error("anchor has no path (required for path-range strategy)")]
    NoPath,                                      // Validation

    #[error("anchor has no symbol (required for symbol strategy)")]
    NoSymbol,                                    // Validation

    #[error("anchor has no content digest (required for content-digest strategy)")]
    NoContentDigest,                             // Validation

    #[error("anchor strategy is not multi: {0:?}")]
    NotMulti(AnchorStrategy),                    // Validation

    #[error("invalid tree OID: {0}")]
    InvalidTreeOid(String),                      // Validation
}
```

## elench-projection

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    #[error("claim log is empty — nothing to project")]
    EmptyLog,                                    // Permanent

    #[error("claim {0} has no anchor — cannot determine tree")]
    NoAnchor(String),                            // Permanent

    #[error("store error: {0}")]
    Store(String),                               // Retriable (wraps StoreError)
}

#[derive(Debug, thiserror::Error)]
pub enum MaterializeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),                  // Retriable

    #[error("store error: {0}")]
    Store(String),                               // Retriable

    #[error("blob {0} not found in store")]
    BlobNotFound(String),                        // Permanent

    #[error("tree {0} not found in store")]
    TreeNotFound(String),                        // Permanent

    #[error("projection has no commits — nothing to materialize")]
    NoCommits,                                   // Permanent
}
```

## elench-predicate

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("unexpected end of input")]
    UnexpectedEof,                               // Validation

    #[error("unexpected token: {0}")]
    UnexpectedToken(String),                     // Validation

    #[error("invalid regex: {0}")]
    InvalidRegex(#[from] regex::Error),          // Validation

    #[error("expected {expected}, found {found}")]
    Expected { expected: String, found: String }, // Validation

    #[error("empty expression")]
    Empty,                                       // Validation
}

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("expected bool, got {0:?}")]
    ExpectedBool(Value),                         // Validation

    #[error("expected int, got {0:?}")]
    ExpectedInt(Value),                          // Validation

    #[error("expected str, got {0:?}")]
    ExpectedStr(Value),                          // Validation

    #[error("field '{0}' not available on this expression")]
    InvalidField(String),                        // Validation

    #[error("I/O error reading {path}: {err}")]
    Io { path: String, err: std::io::Error },    // Retriable

    #[error("command failed to execute: {0}")]
    CommandExec(String),                         // Permanent

    #[error("unknown field: {0}")]
    UnknownField(String),                        // Validation

    #[error("invalid regex: {0}")]
    InvalidRegex(#[from] regex::Error),          // Validation
}
```

## Error handling principles

1. **Fail closed.** If the gate cannot evaluate, the verdict is `fail`,
   not `pass`. An unevaluated claim is NOT a passing claim.

2. **No silent corruption.** A corrupt store yields `StoreError::CorruptStore`
   and marks all unreadable claims as `unevaluated`. It does not guess.

3. **Append-only is a store invariant, not a recovery mechanism.**
   `StoreError::ContentAddressingViolation` prevents overwriting; it does
   not provide conflict resolution.

4. **Agent emission errors are rejection, not retry.** The validator
   rejects; the agent reformulates and re-emits. There is no partial
   acceptance.

5. **Builder agreement unavailability is explicit.** The gate reports
   it as a named condition failure, not a silent degradation to single
   signature.

6. **Projection errors are loud.** Non-deterministic synthesis
   (ProjectionError) is a P1 failure that breaks R6. Write-through-git
   is an explicit rejection, not a silent no-op.

7. **Materialization errors are categorised.** I/O errors are
   retriable; missing blobs/trees are permanent. The caller knows
   which to retry and which to report.
