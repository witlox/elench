# How It Works

elench has no server. It is a single-process CLI that agents call and
humans project from. The claim log is the primary history; git is a
read-only projection derived from it.

## The chain

```
 Agent ──elench emit──► Claim Log (primary history)
   │                         │
   │                         ├──elench gate──► Verdict (live evaluation)
   │                         │
   │                         └──elench git init──► .git/ (projection)
   │                                                   │
 Human ◄──git log/blame/checkout── .git/
```

1. **Agent emits a claim** after each change. A claim is a signed
   assertion about a tree: what was checked, to what depth, what
   remains unevaluated. The agent signs with an Ed25519 key and emits
   a DSSE envelope (same format as build provenance — INV-22).

2. **The harness emits what it observed.** A CI run, a test suite, a
   lint check — these are harness-observed claims with
   `origin.kind = harness-observed`. The agent cannot forge them
   (INV-06). The agent emits only what nothing else can observe:
   stated premises, rejected alternatives, assumptions carried forward.

3. **The claim log IS the history.** Not git. Not a database. An
   append-only, content-addressed, revocable record. Status is
   computed by folding the log — never stored (INV-04). A claim with
   no falsification or verification against it is `unevaluated`, not
   `passed` (R5).

4. **The gate evaluates live.** `elench gate <tree> <claims.json>`
   is a predicate over claims, not a build (R3). Any party with the
   claim log and no compute can evaluate and get the same answer as
   one with a build farm. The verdict is not cached; it is recomputed
   on demand (R4, INV-14).

5. **Git is a projection.** `elench git init <path> <claims.json>`
   synthesizes real zlib-compressed git objects (blobs, trees,
   commits) from the claim log. Two parties with the same log produce
   byte-identical objects (BC4, INV-20). After materialization,
   `git log`, `git blame`, and `git checkout` work — humans use git
   without knowing elench exists.

6. **Retroactive invalidation.** A finding late in a session can
   falsify a claim that a shipped artifact depended on. The blast
   radius is the transitive `dependsOn` closure — every claim that
   depended on the falsified one is also falsified. No byte of code
   changes. No re-signing. The gate re-evaluates and the verdict
   flips. This is the property that justifies the project.

## Why no server

Three reasons, all in `docs/problem.md`:

1. **No daemon to maintain.** Everything is derivable from the
   content-addressed store by a client-side binary (problem.md
   anti-goal). No single point of failure, no service to keep alive.

2. **Independent evaluation.** Two parties with the same claim log
   but different policies can evaluate independently. A server would
   either centralize evaluation (defeating R3) or require a
   consensus protocol (explicitly out of scope).

3. **The asymmetry is enforceable without a server.** The validator
   (`elench-claim::validate_claim`) enforces all emission rules
   client-side: agents cannot emit harness-observed records (INV-06),
   only the harness emits verifications (INV-07), only humans emit
   residue-acceptance (INV-12). No server needed — the claim itself
   carries `origin.kind`, and the validator cross-checks it against
   the signer's entity.

## Agent integration

Agents integrate by calling `elench emit` after each change:

```sh
# Agent writes code, then emits a claim about what was checked
elench --store fjall /path/to/store emit claim.json

# CI (the harness) runs the build and emits provenance
elench --store fjall /path/to/store build <tree> --artifact dist.tar.gz -- make

# CI emits verification claims (test results, lint results)
elench --store fjall /path/to/store emit test-results.json
elench --store fjall /path/to/store emit lint-results.json

# Human evaluates the gate before release
elench --store fjall /path/to/store gate <tree> claims.json

# Human materializes git to review the history
elench --store fjall /path/to/store git init /tmp/review claims.json
cd /tmp/review && git log
```

The dogfooding pipeline (`dogfooding/emit-continuous.sh`) demonstrates
this: it runs `cargo build`, `cargo test --lib`, `cargo clippy`, and
`cargo fmt --check`, emitting a harness-observed claim for each. The
accumulated claims are then gated and reconciled.

## What's not built yet

- **Continuous agent integration.** The dogfooding pipeline emits 4
  claims per CI run. A full regime would have agents emit claims on
  every change to a shared fjall store. This is plumbing (call
  `elench emit` from agent hooks), not architecture.
- **Multi-party distribution.** Two parties with the same claim log
  can already evaluate independently. What's missing is a protocol
  for distributing the claim log between parties — but that's out of
  scope by design (problem.md: "Not a consensus mechanism").
- **K-of-N builder agreement.** The gate checks for it (condition 4),
  E2 passed (same-triple divergences are cheap-to-fix), but no
  K-of-N agreement has been run against real builds with K>1.
