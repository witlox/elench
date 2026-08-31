# E2 — Build reproducibility

**Gates: BC3. Independent of E0/E1; can run in parallel.**

## Question

Can the target build be made bit-reproducible across independent machines at
acceptable cost? K-of-N builder agreement is meaningless without it, and
without K-of-N the release gate degrades to a single signature.

## Method

1. Take the current build as-is. Build the same commit on three machines
   differing in OS version, CPU, filesystem ordering, and locale.
2. Compare output digests. For each divergence, identify the cause:
   embedded timestamp, path leakage, non-deterministic ordering, toolchain
   version drift, network-fetched dependency.
3. Classify each cause: cheap to fix, expensive to fix, or structural.
4. Estimate the cost of reaching bit-reproducibility under a hermetic
   derivation model, and separately under the existing build with fixes
   applied.

## Pre-registered thresholds

- Divergences all cheap-to-fix: proceed with K-of-N in the release policy.
- Any structural divergence: K-of-N is unavailable. The release gate must be
  specified for a single trusted builder, and `docs/release-policy.md` needs
  rewriting before implementation, not after.

## Note

This is the one binding constraint that is not novel. Reproducible-builds
work, `guix challenge`, and rebuilderd have prior art and tooling. Reuse it
rather than measuring from scratch — the contribution here is the claim log,
not the rebuild comparison.
