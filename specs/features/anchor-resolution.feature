Feature: Anchor resolution
  Anchors point at code within a tree. If anchors rot, revocation
  targets the wrong code and blast radius is fiction. E1 measures
  survival rate for each strategy over real refactor sequences.

  Scenario: A path-range anchor resolves at the same commit
    Given a claim anchored to src/lib.rs lines 10-20 at commit C0
    When the anchor is resolved at commit C0
    Then it resolves to src/lib.rs lines 10-20

  Scenario: A path-range anchor may misresolve after a reformat
    Given a claim anchored to src/lib.rs lines 10-20 at commit C0
    And a formatter was applied in commit C1 that moved the anchored code to lines 15-25
    When the anchor is resolved at commit C1 using path-range strategy
    Then it resolves to lines 10-20 (the wrong code) OR fails to resolve
    And if using multi-strategy, the result is reported as "degraded"

  Scenario: A symbol anchor survives reformatting
    Given a claim anchored to symbol "parse_input" at commit C0
    And a formatter was applied in commit C1 that moved the function
    When the anchor is resolved at commit C1 using symbol strategy
    Then it resolves to the function "parse_input" at its new location

  Scenario: A symbol anchor dies on rename
    Given a claim anchored to symbol "parse_input" at commit C0
    And the function was renamed to "parse" in commit C1
    When the anchor is resolved at commit C1 using symbol strategy
    Then it fails to resolve

  Scenario: Wrong-resolution is reported distinctly from failure
    Given a claim anchored at commit C0 using path-range strategy
    When the anchor is resolved at commit C1 and resolves to the wrong code
    Then the result is "wrong-resolution", not "unresolved"
    And wrong-resolution rate > 2% disqualifies the strategy
