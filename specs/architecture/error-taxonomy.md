# Error Taxonomy

Error types by crate, with the invariant or failure mode each addresses.
Library code uses `thiserror`; the binary may use `anyhow`.

## elench-claim

```rust
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("origin.kind is required")]
    MissingOriginKind,                           // INV-05

    #[error("agents cannot emit harness-observed records")]
    AgentEmitsHarnessObserved,                   // INV-06

    #[error("only the harness may emit verification records")]
    NonHarnessEmitsVerification,                 // INV-07

    #[error("predicate claims require an executable expression")]
    PredicateWithoutExpression,                  // INV-08

    #[error("only humans may emit residue-acceptance records")]
    NonHumanEmitsResidueAcceptance,              // INV-12

    #[error("residue-acceptance must name specific gaps (non-empty target)")]
    ResidueAcceptanceWithoutTargets,             // INV-12

    #[error("dependsOn is empty — claim asserts it was reached from nothing")]
    EmptyDependsOn,                              // INV-10 (warn, not reject)

    #[error("schema validation failed: {0}")]
    SchemaViolation(String),                     // general
}

#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("claim {0} not found in log")]
    ClaimNotFound(String),

    #[error("log is corrupt: {0}")]
    CorruptLog(String),
}
```

## elench-envelope

```rust
#[derive(Debug, thiserror::Error)]
pub enum EnvelopeError {
    #[error("invalid signature")]
    InvalidSignature,

    #[error("unknown signer: {0}")]
    UnknownSigner(String),

    #[error("malformed DSSE envelope: {0}")]
    MalformedEnvelope(String),

    #[error("unsupported predicateType: {0}")]
    UnsupportedPredicateType(String),
}
```

## elench-store

```rust
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("ref already exists: {0}")]
    RefExists(String),                           // INV-01 (append-only)

    #[error("git operation failed: {0}")]
    GitOperation(String),

    #[error("repository not found: {0}")]
    RepositoryNotFound(String),
}
```

## elench-gate

```rust
#[derive(Debug, thiserror::Error)]
pub enum GateError {
    #[error("falsified premise in blast radius: {0}")]
    FalsifiedPremise(String),                    // release-policy condition 1

    #[error("unbounded residue: {0} > {1}")]
    UnboundedResidue(usize, usize),              // release-policy condition 2

    #[error("origin floor not met for claim: {0}")]
    OriginFloorNotMet(String),                   // release-policy condition 3

    #[error("builder agreement unavailable (E2 not passed)")]
    BuilderAgreementUnavailable,                 // release-policy condition 4

    #[error("policy evaluation failed: {0}")]
    PolicyEvaluationFailed(String),
}
```

## Error handling principles

1. **Fail closed.** If the gate cannot evaluate, the verdict is `fail`,
   not `pass`. An unevaluated claim is NOT a passing claim.

2. **No silent corruption.** A corrupt log yields `StatusError::CorruptLog`
   and marks all unreadable claims as `unevaluated`. It does not guess.

3. **Append-only is a store invariant, not a recovery mechanism.**
   `StoreError::RefExists` prevents overwriting; it does not provide
   conflict resolution.

4. **Agent emission errors are rejection, not retry.** The validator
   rejects; the agent reformulates and re-emits. There is no partial
   acceptance.

5. **Builder agreement unavailability is explicit.** The gate reports
   it as a named condition failure, not a silent degradation to single
   signature.
