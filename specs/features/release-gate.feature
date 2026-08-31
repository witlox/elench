Feature: Release gate is a predicate over claims
  The gate is cheap and deterministic, evaluable without build
  capability (R3). An artifact's acceptability is a live evaluation
  against the current claim log, not a signature frozen at release
  time (R4). See docs/release-policy.md for the four conditions.

  Scenario: Gate passes when no falsified premises exist
    Given a tree T with claims all having status "passed" or "unevaluated"
    And a release policy P that allows the existing unevaluated residue
    When the release gate is evaluated for tree T under policy P
    Then the gate passes
    And the verdict contains no failure reasons

  Scenario: Gate fails when a falsified premise exists
    Given a tree T with a claim cl_x having status "falsified"
    And a release policy P that does not allow the falsified claim
    When the release gate is evaluated for tree T under policy P
    Then the gate fails with reason "falsified premise: cl_x"

  Scenario: Gate fails when unevaluated residue exceeds policy bounds
    Given a tree T with 5 unevaluated claims
    And a release policy P that allows at most 3 unevaluated claims
    When the release gate is evaluated for tree T under policy P
    Then the gate fails with reason "unbounded residue: 5 > 3"

  Scenario: Excess residue is covered by a human acceptance record
    Given a tree T with 5 unevaluated claims
    And a release policy P that allows at most 3 unevaluated claims
    And a residue-acceptance record signed by a human key naming the 2 excess claims
    When the release gate is evaluated for tree T under policy P
    Then the gate passes

  Scenario: Gate evaluates without build capability
    Given a party with only the git refs and no compute
    When the party evaluates the release gate for tree T under policy P
    Then the party reaches the same verdict as a party with a build farm

  Scenario: A previously released artifact's status changes after falsification
    Given an artifact D released from tree T under policy P with gate passing
    When a load-bearing claim is falsified after release
    And the gate is re-evaluated for the same (T, P)
    Then the gate now fails
    And no byte of D was changed and no re-signing occurred
