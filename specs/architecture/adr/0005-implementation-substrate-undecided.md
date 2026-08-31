# ADR-0005 — Implementation substrate (UNDECIDED)

**Status:** proposed — no decision made

## Context
The tool is a client-side binary reading and writing git refs, verifying
DSSE envelopes, evaluating predicates, and computing transitive closures.
No daemon (per problem.md anti-goals).

## Candidates
- **Rust + gitoxide.** Native ref manipulation without libgit2's C surface,
  good sandboxing story, single static binary. Strongest fit for a tool that
  must run inside an agent sandbox on both macOS and Linux.
- **Go.** Faster to write, go-git is serviceable, easier contribution curve.
- **Shell out to `git` plumbing.** Zero library risk, painful for the
  closure computation and unpleasant to test.

## Note
This must not be decided by habit. The forcing requirements are: runs inside
an existing agent sandbox, no daemon, deterministic evaluation, and small
enough to audit. Whichever candidate best serves *those* wins, and the
reasoning goes here before any code is written.
