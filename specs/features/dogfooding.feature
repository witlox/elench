Feature: Dogfooding
  elench eats its own dog food. Agents working on elench emit claims
  about elench's own code — assertions that specific invariants are
  enforced by specific file locations. The claims are gated, reconciled,
  and projected using elench's own CLI. This is the closed loop.

  Scenario: Dogfooding claims are emitted and stored
    Given dogfooding/claims.json with 10 claims about elench's invariants
    When elench emit dogfooding/claims.json runs
    Then each claim is signed and stored
    And the output shows the computed claim ID

  Scenario: Gate evaluates dogfooding claims
    Given dogfooding/claims.json with 10 unevaluated assertions
    When elench gate <tree> dogfooding/claims.json runs
    Then the gate passes (no falsified premises)
    And the verdict contains no failure reasons

  Scenario: Log statistics show all unevaluated
    Given dogfooding/claims.json with 10 assertions, 0 verifications
    When elench log dogfooding/claims.json runs
    Then total is 10, assertions is 10, unevaluated is 10
    And noise ratio is 0.00

  Scenario: Git projection from dogfooding claims
    Given dogfooding/claims.json with 10 tree-changing claims
    When elench git dogfooding/claims.json runs
    Then the projection shows commits, blobs, and trees
    And git log oneline output contains "elench claim:"

  Scenario: Reconciliation reports drift when trees are not in the store
    Given dogfooding/claims.json with anchors pointing at trees not in the store
    When elench reconcile <tree> dogfooding/claims.json runs
    Then the report shows drifted claims (anchors cannot resolve)
    And the exit code is 0

  Scenario: Continuous dogfooding emits harness-observed claims
    Given elench's own source tree
    When make dogfood-continuous runs
    Then 4 claims are emitted (build, test, lint, fmt)
    And each claim has origin.kind = harness-observed
    And the gate passes (all verifications, no falsifications)
    And the claims are accumulated in a single JSON file

  Scenario: Continuous dogfooding runs in CI nightly
    Given the nightly CI job triggers
    When the tier-3 job completes
    Then make dogfood-continuous runs
    And the resulting claims are uploaded as a CI artifact
