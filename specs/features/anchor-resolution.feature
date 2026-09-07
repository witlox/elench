Feature: Anchor resolution
  Anchors point at code within a tree. If anchors rot, revocation
  targets the wrong code and blast radius is fiction. E1 measures
  survival rate for each strategy over real refactor sequences.

  Scenario: A path-range anchor resolves at the same tree
    Given a claim anchored to src/lib.rs lines 1-10 at tree T0
    And tree T0 contains a blob at src/lib.rs with at least 10 lines
    When the anchor is resolved at tree T0 using the multi strategy
    Then the path-range strategy resolves to src/lib.rs
    And the result is Correct

  Scenario: A path-range anchor fails when the path is not in the tree
    Given a claim anchored to nonexistent.rs at tree T0
    When the anchor is resolved at tree T0
    Then the path-range strategy fails
    And if all strategies fail, the result is Failed

  Scenario: A path-range anchor fails when the range exceeds the blob
    Given a claim anchored to src/lib.rs lines 1-100 at tree T0
    And tree T0 contains a blob at src/lib.rs with only 5 lines
    When the anchor is resolved at tree T0
    Then the path-range strategy fails

  Scenario: A symbol anchor finds a function definition in the tree
    Given a claim anchored to symbol "parse_input" at tree T0
    And tree T0 contains a blob with "fn parse_input" in src/parser.rs
    When the anchor is resolved at tree T0 using the multi strategy
    Then the symbol strategy resolves to src/parser.rs
    And the result is Correct

  Scenario: A symbol anchor dies on rename
    Given a claim anchored to symbol "parse_input" at tree T0
    And tree T0 contains no definition of "parse_input"
    When the anchor is resolved at tree T0
    Then the symbol strategy fails

  Scenario: A content-digest anchor finds matching content in the tree
    Given a claim anchored with content_digest D at tree T0
    And tree T0 contains a blob whose SHA-256 equals D
    When the anchor is resolved at tree T0 using the multi strategy
    Then the content-digest strategy resolves to that blob's path
    And the result is Correct

  Scenario: A content-digest anchor fails when no blob matches
    Given a claim anchored with content_digest D at tree T0
    And no blob in tree T0 has SHA-256 equal to D
    When the anchor is resolved at tree T0
    Then the content-digest strategy fails

  Scenario: Strategies disagree when path and symbol point to different files
    Given a claim anchored to path src/main.rs and symbol "parse_input"
    And tree T0 has src/main.rs (path-range resolves) but parse_input is in src/parser.rs
    When the anchor is resolved at tree T0
    Then the result is Degraded
    And the disagreements include path-range → src/main.rs and symbol → src/parser.rs

  Scenario: Wrong-resolution is reported distinctly from failure
    Given a claim anchored at tree T0 using path-range strategy
    When the anchor is resolved at tree T1 and resolves to the wrong code
    Then the result is "wrong-resolution", not "unresolved"
    And wrong-resolution rate > 2% disqualifies the strategy

  Scenario: Reconciliation reports intact and drifted claims
    Given a tree T0 with two claims: one anchored correctly, one with a missing path
    When reconciliation is run for tree T0
    Then the report lists 1 intact claim and 1 drifted claim
    And the drifted claim's resolution is Failed

  Scenario: Reconciliation CLI command
    Given a claims file with claims anchored to tree T0
    When elench reconcile <tree_oid> <claims.json> runs
    Then the output shows intact and drifted counts
    And the exit code is 0
