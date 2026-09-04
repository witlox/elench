Feature: Store backend selection
  elench ships two storage backends: an in-memory store (default, no
  dependencies) and a persistent fjall-backed store (optional
  `fjall-backend` feature, ADR-0008). Both implement `StoreBackend`,
  so any command that reads or writes the store can target either.

  INV-18: elench owns the store. INV-25: content addressing. INV-26:
  the store is the sole source of truth — all views derive from it
  alone, so a persistent backend must round-trip trees and blobs
  across processes.

  Scenario: Default backend is in-memory
    Given no --store flag is supplied
    When a command runs
    Then it uses the in-memory backend
    And nothing is written to disk

  Scenario: Explicit in-memory backend
    Given the --store memory flag is supplied
    When a command runs
    Then it uses the in-memory backend

  Scenario: Persistent fjall backend is selected by --store fjall <path>
    Given the fjall-backend feature is enabled
    When a blob and a tree are stored with --store fjall <path>
    Then a separate process opening the store at <path> can read the blob back
    And it can read the tree back with the same entries and OID (INV-25)

  Scenario: read_tree round-trips the canonical serialization
    Given a tree with entries is stored in any backend
    When the tree is read back by OID
    Then the entries equal the original entries
    And the tree OID equals the SHA-256 of the canonical serialization

  Scenario: fjall backend without the feature is rejected
    Given the fjall-backend feature is not enabled
    When --store fjall <path> is supplied
    Then the command exits non-zero
    And the error names the missing feature
