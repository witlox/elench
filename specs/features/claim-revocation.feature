Feature: Claim revocation without tree mutation
  A claim's status is changed by appending a falsification record. No
  code changes, no history rewrite. The prior status remains visible.
  Blast radius propagates through the dependsOn closure.

  Scenario: A falsification record changes a claim's status
    Given a claim log containing claim cl_a3f... with computed status "passed"
    When a falsification claim is appended targeting cl_a3f...
    Then the claim log contains two records: the original and the falsification
    And cl_a3f...'s computed status is now "falsified"
    And the original claim's content is byte-identical to before
    And no tree state was added or modified

  Scenario: Blast radius propagates through dependsOn
    Given claims cl_a, cl_b, cl_c where cl_b dependsOn [cl_a] and cl_c dependsOn [cl_b]
    When cl_a is falsified
    Then cl_a, cl_b, and cl_c all have computed status "falsified"
    And the blast radius report lists all three claims in dependency order

  Scenario: A claim with no dependsOn has no blast radius
    Given a claim cl_x with an empty dependsOn field
    When cl_x is falsified
    Then the blast radius report contains only cl_x
    And no other claims change status

  Scenario: A previously released artifact's status changes after falsification
    Given an artifact D released from tree T under policy P with gate passing
    When a load-bearing claim in T's claim set is falsified after release
    Then re-evaluating the gate for (T, P) now fails
    And no byte of D was changed
    And no re-signing occurred

  Scenario: Falsifying a verification reverts the target to unevaluated
    Given claim cl_a with computed status "passed" via verification cl_v
    When a falsification cl_fv targets cl_v (the verification, not cl_a)
    Then cl_v's computed status is "falsified"
    And cl_a's computed status reverts to "unevaluated" (no valid verification remains)
    And cl_a's original claim content is byte-identical to before

  Scenario: Falsifying a falsification reverts the target to its previous status
    Given claim cl_a with computed status "falsified" via falsification cl_f1
    When a falsification cl_f2 targets cl_f1 (the falsification, not cl_a)
    Then cl_f1's computed status is "falsified"
    And cl_a's computed status reverts to its previous status ("unevaluated" or "passed")
    And the revert is recursive — cl_a's dependents also revert

  Scenario: A falsification that changes no status is rejected
    Given claim cl_x with computed status "falsified"
    When an agent emits a new falsification targeting cl_x
    Then the new falsification is rejected
    And the rejection reason includes "falsification changes no status — target already falsified"
