Feature: Claim emission
  Agents emit signed claims about a tree, stored in the elench
  content-addressed store. Claims are append-only; no tree mutation
  occurs. The validator enforces emission rules before a claim is
  accepted.

  Scenario: An agent-asserted predicate claim is stored
    Given an elench store with at least one tree
    When an agent emits a predicate claim asserting "parse('') == Err(EmptyInput)" about tree <oid>
    Then a new claim object exists in the store with a DSSE envelope
    And the statement's predicate has form = "predicate"
    And the statement's predicate has origin.kind = "agent-asserted"
    And the tree is unchanged — no blobs or tree entries were added or modified

  Scenario: An annotation claim is stored but cannot gate
    Given an elench store with a claim log containing one annotation claim
    When the release gate is evaluated against the log
    Then the annotation claim does not contribute to the gate verdict
    And the annotation's status is reported as "unevaluated"

  Scenario: An agent cannot emit a harness-observed claim
    Given an agent attempting to emit a claim
    When the agent sets origin.kind = "harness-observed"
    Then the claim is rejected by the validator
    And no object is written to the store

  Scenario: A predicate without an expression is rejected
    Given an agent attempting to emit a predicate claim
    When the claim's assertion has form = "predicate" but no expression field
    Then the claim is rejected by the validator
    And the rejection reason includes "expression required for predicate"

  Scenario: A claim with dependsOn lists its premises
    Given an agent emits a claim that relies on premises cl_a3f... and cl_91c...
    When the claim is stored
    Then the claim's dependsOn field contains [cl_a3f..., cl_91c...]
    And the blast radius from any falsification of cl_a3f... includes this claim
