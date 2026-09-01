Feature: Unevaluated is a first-class status
  Brownfield code is overwhelmingly unevaluated. A system that cannot
  say so is unusable on real repositories. Policies must permit bounded
  unevaluated residue with a named signer accepting it (R5).

  Scenario: A new claim is unevaluated by default
    Given a freshly emitted assertion claim with no verification or falsification targeting it
    When the claim's status is computed
    Then the status is "unevaluated"
    And the status is NOT "passed"

  Scenario: Unevaluated residue is bounded by policy
    Given a tree T with 5 unevaluated claims
    And a release policy P that allows at most 3 unevaluated claims
    When the release gate is evaluated for tree T under policy P
    Then the gate fails with reason "unbounded residue: 5 > 3"

  Scenario: Excess residue is covered by a residue-acceptance record
    Given a tree T with 5 unevaluated claims
    And a release policy P that allows at most 3 unevaluated claims
    And a residue-acceptance record signed by a human key naming the 2 excess claims
    When the release gate is evaluated for tree T under policy P
    Then the gate passes
    And the acceptance record has kind = "residue-acceptance"
    And the acceptance record has origin.kind = "human-asserted"

  Scenario: A residue-acceptance record from an agent is rejected
    Given an agent attempting to emit a residue-acceptance record
    When the record is submitted to the validator
    Then the record is rejected
    And the rejection reason includes "only human may emit residue-acceptance"

  Scenario: A residue-acceptance names specific gaps, not a blanket acceptance
    Given a residue-acceptance record signed by a human key
    When the record is validated
    Then it must contain a non-empty target array naming the accepted gaps
    And a record with an empty target array is rejected

  Scenario: A corrupt claim cascades to unevaluated dependents
    Given a claim log with claims cl_a, cl_b, cl_c
    And cl_b dependsOn [cl_a] and cl_c dependsOn [cl_b]
    When cl_a becomes corrupt (unreadable in the store)
    Then cl_a's computed status is "unevaluated" (corrupt — cannot read)
    And cl_b's computed status is "unevaluated" (depends on corrupt claim)
    And cl_c's computed status is "unevaluated" (cascading from cl_b)
    And the status report marks them as "unevaluated: corrupt" not "unevaluated: no one checked"
    And the gate may fail if "unevaluated: corrupt" exceeds the residue bound
