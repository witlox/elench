Feature: Git projection
  The git CLI works because elench synthesizes git-compatible objects
  from the claim log. Synthesis is deterministic (BC4): two parties
  with the same claim log produce byte-identical git objects. The
  projection is read-only — writes go through elench, never git.

  Scenario: git log shows synthesized commits
    Given an elench store with a claim log containing N tree-changing claims
    When the git projection is materialized
    Then `git log` shows N commits, one per tree-changing claim
    And each commit's tree OID is a deterministic function of the elench tree
    And each commit's author is derived from the claim's producer.id

  Scenario: Two parties produce identical git objects
    Given an elench store with claim log L
    When party A synthesizes git objects from L on machine A
    And party B synthesizes git objects from L on machine B
    Then A and B produce byte-identical commit OIDs
    And A and B produce byte-identical tree OIDs
    And `git log` output is identical on both machines

  Scenario: git blame maps to claims
    Given an elench store with claims that introduced specific lines
    When the git projection is materialized and `git blame` is run
    Then each line maps to the commit synthesized from the claim that introduced it
    And the commit's author is the claim's producer, not a user-configured name

  Scenario: Write through git is rejected
    Given a materialized git projection of an elench store
    When a user runs `git commit` in the projected repository
    Then the commit is rejected or has no effect on the elench store
    And the elench claim log is unchanged
