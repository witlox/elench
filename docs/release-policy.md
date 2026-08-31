# Release policy

The gate is a predicate over claims. The build is a separate, expensive
function of a tree. Conflating them is what makes a green check mean "some
machine you don't control ran something." Keeping them apart is what lets a
party with no compute reach the same verdict as one with a build farm (R3).

## Shape

Release of artifact digest `D` from tree `T` under policy `P` holds when all
of the following are true at evaluation time:

1. **No falsified premise.** No claim in the transitive `dependsOn` closure
   rooted at `T`'s claim set has status `falsified`.
2. **Bounded residue.** Claims with status `unevaluated` are within `P`'s
   allowance, and each excess is covered by a `residue-acceptance` record
   signed by a key `P` names.
3. **Origin floor.** Claims that `P` designates load-bearing have
   `origin.kind = harness-observed`. Agent-asserted claims may inform, but
   `P` should not let them alone carry a release.
4. **Builder agreement.** K independent producers have signed statements with
   subject `D` for tree `T`, each meeting `P`'s `hermeticity` floor.

Condition 4 is where "true artifact" actually lives, and it is the expensive
one. It is unavailable unless E2 passes.

## Evaluation is live, not frozen

The artifact carries a pointer to `(T, P)`, not a verdict. Consumers
re-evaluate. If a load-bearing claim is falsified after release, the artifact's
status changes with no byte moving and no re-signing — the property from
README that justifies the project. `T` is an elench tree OID
(ADR-0001), not a git commit. The git projection (ADR-0002) is not
consulted at evaluation time.

This is certificate-revocation shaped and inherits its problems: consumers who
never re-check are unprotected, and there is no push path. Do not pretend
otherwise in the docs.

## Deliberately not specified

- **What K should be.** Depends on how many independent builders exist, which
  is an organisational fact, not a design one.
- **Cross-party reconciliation.** Two evaluators with different `P` reaching
  different verdicts is correct behaviour, not a bug. Out of scope.
- **Emergency override.** Every real system grows one. Specifying it now
  would be guessing; leaving it unspecified means it will be added badly under
  pressure. Flagged, not resolved.
