Feature: Build provenance digest
  `elench build <tree> [--artifact <path>] -- <command...>` runs a build,
  captures the exit code, and emits a harness-observed provenance claim
  (ADR-0003, INV-22). The digest is SHA-256 of the build artifact when
  `--artifact <path>` names an existing file; otherwise it falls back to
  SHA-256 of stdout. The same producer cannot forge a digest it did not
  observe — the harness reads the file, not the agent.

  Scenario: digest is SHA-256 of the artifact file when --artifact is present
    Given a build that writes 42 bytes to /tmp/artifact.bin
    And a known content for /tmp/artifact.bin
    When elench build <tree> --artifact /tmp/artifact.bin -- <command> runs
    Then the emitted claim's content_digest equals SHA-256(/tmp/artifact.bin)
    And the claim's evidence[0].digest equals SHA-256(/tmp/artifact.bin)
    And the claim's evidence[0].exit_code equals the build's exit code

  Scenario: digest falls back to stdout when --artifact is absent
    Given a build that writes "hello" to stdout and exits 0
    When elench build <tree> -- <command> runs
    Then the emitted claim's content_digest equals SHA-256("hello")
    And the claim's evidence[0].digest equals SHA-256("hello")

  Scenario: missing artifact file is rejected
    Given no file exists at /tmp/nonexistent.bin
    When elench build <tree> --artifact /tmp/nonexistent.bin -- <command> runs
    Then the command exits non-zero
    And the error names the missing artifact

  Scenario: --artifact is parsed before --, not passed to the build
    Given a build command echo --artifact foo
    When elench build <tree> --artifact /tmp/real.bin -- echo --artifact foo runs
    Then the digest is SHA-256(/tmp/real.bin), not influenced by echo's args
    And echo receives "--artifact foo" as its own argument
