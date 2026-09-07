# elench dogfooding

elench eats its own dog food. This directory contains claims about
elench's own code — assertions that specific invariants are enforced
by specific code locations, backed by the build provenance and test
results that the CI pipeline produces.

## What this is

Each claim in `claims.json` asserts that one of elench's invariants
(INV-01 through INV-29, see `specs/fidelity/INDEX.md`) is enforced by
a specific file + line range. The claims are signed, stored, and
gated using elench's own CLI. This is the closed loop: elench records
what was checked about itself, and the gate evaluates those records
live.

## How to run

```sh
# 1. Emit all dogfooding claims (signs + stores)
make dogfood-emit

# 2. Gate the dogfooding tree (evaluates against the claim log)
make dogfood-gate

# 3. Reconcile (check which claims still resolve)
make dogfood-reconcile

# 4. Materialize a git projection from the claims
make dogfood-project
```

Or run the full pipeline:

```sh
make dogfood
```

## Claim structure

Each claim is an `assertion` with:
- `origin.kind = agent-asserted` (an agent checked, not the harness)
- `anchor.strategy = multi` (path-range + symbol + content-digest)
- `anchor.path` pointing at the file that enforces the invariant
- `anchor.symbol` naming the function or module
- `assertion.form = predicate` with an executable expression
- `depends_on` listing prior claims the assertion relies on

The claims are NOT harness-observed verifications — they are
agent-asserted. Only the harness can emit verifications (INV-07).
The dogfooding claims represent an agent's belief that specific code
enforces specific invariants; the gate evaluates whether those
beliefs have been falsified.
