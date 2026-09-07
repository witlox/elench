Feature: Property-based tests (proptest)
  Invariants that hold for ALL inputs, not just the examples in unit
  tests. Each property is tagged with the invariant it covers and
  classified as PROPERTY depth in the fidelity index.

  Scenario: INV-25 — content addressing is deterministic
    Given any two byte arrays with the same content
    When Oid::from_blob_data is called on each
    Then both produce the same OID
    And given any two byte arrays with different content
    Then both produce different OIDs

  Scenario: INV-28 — claim OID is content hash
    Given any claim C
    When ClaimId::from_content is called on C
    And the same claim C is constructed again with identical fields
    Then both produce the same ClaimId

  Scenario: INV-13 — compute_status is a pure function
    Given any claim ID K and any log L
    When compute_status(K, L) is called twice
    Then both calls return the same result

  Scenario: INV-29 — dependsOn is acyclic
    Given any log L that passes validate_claim for each claim
    When compute_status is called for any claim in L
    Then it does not loop forever (returns Ok or Err, not a hang)

  Scenario: INV-20 — git synthesis is deterministic
    Given any claim log L and any store S
    When synthesize(L, S) is called twice
    Then both calls produce byte-identical commit OIDs and tree OIDs
