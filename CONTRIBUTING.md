# Contributing to elench

## Dev setup

```sh
git clone https://github.com/witlox/elench.git
cd elench
rustup default stable       # requires Rust 1.85+ (edition 2024)
make                         # fmt-check + lint + Tier 1 tests
```

If `make` passes, the workspace is ready for development. If it does
not, run `make fmt` first (auto-format), then `make` again.

## Coding standards

Follow the global Rust guidelines at
`~/.config/opencode/guidelines/rust.md`. Key points:

- `cargo fmt` + `cargo clippy -- -D warnings`. Run `make` before every
  commit.
- `unwrap()` / `expect()` only in tests, or in production with a
  `// SAFETY:` / `// INVARIANT:` comment explaining why.
- `#[must_use]` on any function returning a wrapper a caller might drop.
- Library code uses `thiserror`; the binary may use `anyhow`. Never
  `Box<dyn Error>` in a public library API.
- Tests: `#[test] fn scenario_<context>_<behavior>()`.
- Slow tests: `#[ignore = "slow: <reason — what makes this expensive>"]`.
- Property-based: `proptest` for invariant testing.

## Testing

Three tiers, cascading. Each higher tier includes the lower.

| Tier | Command | What | When |
|------|---------|------|------|
| 1 (fast) | `make test` | `cargo test --lib` | Between every edit; pre-commit |
| 2 (slow) | `make test-slow` | Tier 1 + all targets including ignored | Pre-PR |
| 3 (full) | `make test-full` | Tier 2 + e2e against real repositories | Pre-merge / nightly |

`make` (no target) = fmt-check + lint + Tier 1. Run it before every
commit. If it fails, do not commit.

## Workflow

elench follows a greenfield diamond protocol:

```
analyst → architect → adversary (gate 1) → implementer → auditor → integrator
```

- The **analyst** writes specs (domain model, invariants, Gherkin).
- The **architect** derives interfaces, contracts, and ADRs.
- The **adversary** gates implementation — no code until findings are
  resolved.
- The **implementer** builds within architect boundaries (TDD + BDD).
- The **auditor** measures test depth and gates PR.
- The **integrator** verifies cross-context interactions.

See `AGENTS.md` for full role dispatch and escalation paths.

## Harness contract

Agents working on this repository emit claims following the rules in
`AGENTS.md` §Harness contract. The key asymmetry: the harness emits
what it observed; the agent emits only what nothing else can observe.
The validator (ADR-0006) is implemented and enforces all emission
rules.

## Experiments

Three binding experiments — all PASSED:

- **E0** (predicate ratio) — PASSED 0.72 (threshold >= 0.30). Gates
  ADR-0004 and all implementation. PROCEED AS DESIGNED.
- **E1** (anchor survival) — PASSED 99.4% correct, 0.6% wrong (all
  strategies USABLE). Gates the `anchor` object in
  `schema/claim.schema.json`. Proceed with multi.
- **E2** (build reproducibility) — PASSED. Same-triple divergences all
  cheap-to-fix. K-of-N available. Gates release-policy condition 4.

See `experiments/` for pre-registered thresholds and results.

## PR process

1. Run `make test-slow` (Tier 2) before opening a PR.
2. Critical + High review findings are resolved before the next feature
   begins. Medium/Low get an issue filed.
3. No `TODO` / `FIXME` left in code.
4. Update `specs/fidelity/INDEX.md` if test coverage changes.
5. Write an ADR (`specs/architecture/adr/`) for significant architectural
   decisions. Number sequentially.

## License

Contributions are licensed under the [MIT License](LICENSE-MIT).
