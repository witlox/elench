# ADR-0005 — Implementation substrate: Rust, content-addressed store

**Status:** proposed
**Serves:** R6, BC4
**Supersedes:** the original ADR-0005 ("implementation substrate
undecided")

## Context

elench owns its own content-addressed store (ADR-0001). The tool is a
client-side binary that reads and writes the store, verifies DSSE
envelopes, evaluates predicates, computes transitive closures, and
synthesizes a git projection (ADR-0002). No daemon (per problem.md
anti-goals). Deterministic synthesis is a binding constraint (BC4).

## Decision

**Rust.** elench is implemented in Rust (edition 2024, MSRV 1.85) for
the following reasons:

1. **Single static binary.** The tool must run inside an agent sandbox
   on both macOS and Linux. Rust produces a single static binary with
   no runtime dependency.
2. **Deterministic by construction.** Rust has no GC pause, no
   non-deterministic finalization, and a well-defined memory model.
   The content-addressed store and git synthesis require bit-exact
   determinism (BC4).
3. **Sandboxing.** Rust's `unsafe` discipline and absence of a runtime
   make the audit surface small — important for a tool that must be
   trusted as evidence infrastructure.
4. **No libgit2 C surface.** elench owns its own store; it does not
   link against libgit2. The git projection synthesizes git objects
   directly (OIDs, tree entries, commit objects) without a git
   library.

## Rejected alternatives

- **Go.** Faster to write, easier contribution curve. But GC
  introduces non-determinism in finalization order, which is a risk
  for BC4. Go's binary is not statically linked by default on all
  platforms.
- **Shell out to `git` plumbing.** Zero library risk, but elench owns
  the store — there is no git repo to shell out to. The projection
  synthesizes objects; it does not delegate to git.
- **Rust + gitoxide.** gitoxide is excellent for git operations, but
  elench is not a git tool. It would pull in a large dependency for
  operations elench must own (content addressing, tree manipulation).
  The git projection writes raw git objects; it does not need a git
  library.

## Consequences

Rust's contribution curve is steeper than Go's. The workspace is
already scaffolded (5 crates). The content-addressed store must be
implemented from scratch — no libgit2, no gitoxide. This is more work
but eliminates a large dependency and keeps the audit surface small.
