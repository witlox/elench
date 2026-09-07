# CLI Reference

All commands accept an optional `--store memory|fjall <path>` flag
before the command name to select the storage backend. Default is
`--store memory` (in-memory, no persistence).

## emit

Create, sign, and store a claim.

```sh
elench [--store ...] emit <claim.json>
```

The JSON file must contain a `Claim` with fields matching
`schema/claim.schema.json`. The claim's `id` is computed from content
(INV-28); the provided `id` is ignored. The claim is signed in a DSSE
envelope (INV-22) and stored in the content-addressed store.

## verify

Verify a DSSE envelope and validate the enclosed claim.

```sh
elench verify <envelope.json>
```

Extracts the claim and signer from the envelope, verifies the Ed25519
signature, and runs the emission-rule validator (ADR-0006).

## status

Compute a claim's status by folding the log.

```sh
elench status <claim_id> [<claims.json>]
```

Status is `unevaluated`, `passed`, or `falsified` — computed from the
log, never stored (INV-04). If no claims file is given, the status is
computed from an empty log (live evaluation, INV-14).

## gate

Evaluate the release gate for a tree.

```sh
elench gate <tree_oid> [<claims.json>]
```

The gate is a predicate over claims (R3). It checks four conditions
(see `docs/release-policy.md`): no falsified premise, bounded residue,
origin floor, builder agreement. The verdict is computed live, not
cached (INV-14).

## blast

Compute the blast radius from a claim.

```sh
elench blast <claim_id> [<claims.json>]
```

The blast radius is the transitive `dependsOn` closure — all claims
that depend on the given claim, directly or indirectly. If the given
claim is falsified, every claim in its blast radius is also
falsified.

## git

Materialize the git projection.

```sh
elench [--store ...] git <claims.json>           # oneline (default)
elench [--store ...] git oneline <claims.json>    # git log --oneline
elench [--store ...] git full <claims.json>       # full git log
elench [--store ...] git init <output_path> <claims.json>  # real .git/
```

The projection synthesizes git-compatible objects (blobs, trees,
commits) from the claim log (ADR-0002). Synthesis is deterministic
(BC4, INV-20): two parties with the same claim log produce
byte-identical objects.

`git init` writes real zlib-compressed git objects to
`<output_path>/.git/objects/`. After materialization, `git log`,
`git blame`, and `git checkout` work in the output directory.

## store

Store a blob or tree in the content-addressed store.

```sh
elench [--store ...] store blob <file>
elench [--store ...] store tree <file1> <file2> ...
```

Blobs are SHA-256 content-addressed (INV-25). Trees are canonical
(mode space name null oid) and SHA-256 content-addressed. With
`--store fjall <path>`, objects persist across processes.

## log

Log statistics: count, status distribution, conflicts.

```sh
elench log <claims.json>
```

Reports total count, kind distribution (assertion, verification,
falsification, supersession, residue-acceptance), status distribution
(unevaluated, passed, falsified), noise ratio, and `dependsOn`
density.

## review

Review unevaluated claims for a tree.

```sh
elench review <tree_oid> <claims.json>
```

Lists all unevaluated claims anchored to the given tree, showing
form, language, source, and producer. The human must name each gap
before `elench accept` issues a residue-acceptance.

## accept

Accept named unevaluated gaps (residue-acceptance).

```sh
elench accept <tree_oid> <claims.json> --claim <id> [<id>...]
```

Only humans may emit residue-acceptance records (INV-12). Each named
claim must be unevaluated; the acceptance covers the excess residue
above the policy's allowance (R5).

## conflicts

List active predicate conflicts for a tree.

```sh
elench conflicts <tree_oid> <claims.json>
```

Two predicates are a conflict if and only if they anchor to the
**same code location** (same path+range, or same symbol) but assert
**different expressions**. Two predicates about different files are
not a conflict. Last-writer-wins by timestamp; the older claim is
flagged for resolution.

## compact

Compact the claim log (manual, destructive).

```sh
elench compact <claims.json> [--before <timestamp>]
```

Retires all claims before the cut-off timestamp, freezing their
statuses. The compaction record carries the frozen status snapshot
forward. Active claims continue to be revocable (R1 preserved for
active claims, deliberately violated for retired ones).

## reconcile

Detect anchors that no longer resolve.

```sh
elench reconcile <tree_oid> <claims.json>
```

For each claim anchored to the given tree, resolves the anchor using
the multi strategy (path-range, symbol, content-digest). Claims that
fail to resolve or are degraded are reported as drifted. The
reconciliation pass is read-only — it does not auto-fix.

## artifact

Create or verify a release artifact (INV-15).

```sh
elench artifact create <tree_oid> <policy_name> <sha256_digest>
elench artifact verify <artifact.json> <claims.json>
```

The artifact carries `(version, tree, policy, digest, released_at)` —
NOT a verdict (INV-15). Consumers re-evaluate the gate at consumption
time (R4). See `schema/artifact.schema.json` for the format.

## build

Run a build, capture exit code + digest, emit provenance.

```sh
elench build <tree_oid> [--artifact <path>] -- <command...>
```

Runs the build command, captures the exit code, and computes a
SHA-256 digest. With `--artifact <path>`, the digest is SHA-256 of
that file (the real build output). Without it, the digest falls back
to SHA-256 of stdout. Emits a build provenance claim with
`origin.kind = harness-observed` (INV-22).
