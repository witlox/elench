# E0 — Pre-registration: session selection criteria

**Status:** pre-registered before extraction. Do not adjust after seeing
results.

## What counts as a session

A session is a contiguous block of agent work on a single repository,
bounded by one of:

1. **Release boundary** — all commits between two releases (tags) on the
   default branch. Each release's notes describe the work done; the
   commits in that range are the session.
2. **PR boundary** — all commits in a single pull request (merged). The
   PR description (if any) and commit messages are the session.
3. **Task cluster** — for repos without releases or PRs, a contiguous
   run of commits with a shared theme (identified by commit message
   prefix or content), bounded by the next different theme.

A session MUST contain at least one tree-changing commit. A session
with only docs, CI config, or dependency bumps is excluded — these are
not representative of the claim density E0 measures.

## How sessions are sampled

**Stratified by repository, not random.** The goal is coverage of
different work types (features, bug fixes, refactors, performance), not
statistical representativeness.

1. From each repository, take the 5 most recent releases (or
   equivalent session boundaries) that contain tree-changing commits.
2. If fewer than 5 releases exist, take PR-boundary sessions or
   task-cluster sessions to reach 5.
3. Target: 20-30 sessions total across 4-6 repositories.
4. Exclude sessions that are purely dependency bumps, CI config, or
   formatting — these carry no claims to classify.

## What makes a session "real work"

- At least one source file (`.rs`, `.py`, `.go`, `.ts`) was added or
  modified.
- The commit messages or release notes describe what was done, not just
  "bump X" or "format Y".
- The work is non-trivial: a bug fix, a feature, a refactor, or a
  performance change — not a typo or a one-line config change.

## Repositories

| Repository | Source | Sessions | Language | Notes |
|------------|--------|----------|----------|-------|
| kiseki | local (/home/witlox/src/kiseki) | 5 | Rust, Go, Python | User's most advanced; release-boundary sessions |
| yoyo-evolve | external (github.com/yologdev/yoyo-evolve) | 5 | Rust | All agent-written; release-boundary sessions |
| scree | local (/home/witlox/src/scree) | 3 | Python, TypeScript | Release-boundary sessions |
| ghyll | local (/home/witlox/src/ghyll) | 3 | Go | PR-boundary or task-cluster sessions |
| pact | local (/home/witlox/src/pact) | 3 | Rust | Release-boundary sessions |
| lattice | local (/home/witlox/src/lattice) | 3 | Python, Rust | Release-boundary or task-cluster sessions |

**Total: 22 sessions** (within the 20-30 target).

## Extraction method

For each session:
1. Read all commit messages in the session range.
2. Read the release notes (if a release-boundary session).
3. Extract candidate claims: assertions the agent makes about its own
   work. Each claim is a single sentence or clause that asserts
   something about what was done, why, or what is true as a result.
4. For each claim, classify:
   - **predicate**: an executable expression can be written NOW, by the
     person doing the classification, in under five minutes, that would
     pass if the claim is true and fail if it is false.
   - **annotation**: prose only. Cannot be expressed as an executable
     expression in under five minutes.

## Threat to validity (additional)

Session selection is stratified by repository and limited to recent
work. This biases toward the agent's current capabilities (better than
its past) and toward repos with active development. The ratio is an
optimistic upper bound, consistent with the threat already noted in
E0-predicate-ratio.md.
