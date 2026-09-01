Feature: Origin is a type, not a label
  Evidence the harness observed and evidence an agent asserted must be
  structurally distinguishable and must never be merged into one status.
  Policies may and generally should discriminate on origin.kind.

  Scenario: Harness-observed and agent-asserted claims are distinct
    Given a claim log with one harness-observed claim and one agent-asserted claim
    When the log is queried for claims with origin.kind = "harness-observed"
    Then only the harness-observed claim is returned
    And the agent-asserted claim is excluded

  Scenario: Policy requires origin floor for load-bearing claims
    Given a release policy P requiring origin.kind = "harness-observed" for load-bearing claims
    And a claim log where a load-bearing claim has origin.kind = "agent-asserted"
    When the release gate is evaluated against P
    Then the gate fails with reason "origin floor not met for claim <id>"

  Scenario: Only the harness emits verification records
    Given an agent attempting to emit a claim with kind = "verification"
    When the claim is submitted to the validator
    Then the claim is rejected
    And the rejection reason includes "only harness may emit verification"

  Scenario: A human cannot emit verification records
    Given a human attempting to emit a claim with kind = "verification"
    When the claim is submitted to the validator
    Then the claim is rejected
    And the rejection reason includes "only harness may emit verification"

  Scenario: A harness cannot emit residue-acceptance records
    Given a harness attempting to emit a claim with kind = "residue-acceptance"
    When the claim is submitted to the validator
    Then the claim is rejected
    And the rejection reason includes "only human may emit residue-acceptance"

  Scenario: A human-asserted claim is distinct from an agent-asserted claim
    Given a claim log with one human-asserted claim and one agent-asserted claim
    When the log is queried for claims with origin.kind = "human-asserted"
    Then only the human-asserted claim is returned
