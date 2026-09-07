# Contributing to elench

## Dev setup

```sh
git clone https://github.com/witlox/elench.git
cd elench
make                         # fmt-check + lint + Tier 1 tests
```

`rust-toolchain.toml` pins stable Rust with rustfmt and clippy.
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
- All public items have doc comments (`///`). Module-level docs (`//!`).
- Commits: conventional commits (`feat:`, `fix:`, `docs:`, `test:`,
  `refactor:`, `perf:`, `chore:`, `ci:`). One logical change per commit.
- Files under 500 lines where practical. One responsibility per file.
- Imports grouped: stdlib -> external -> internal.

## Dependency policy

`deny.toml` enforces license compatibility (MIT-compatible only) and
advisory scanning. `cargo-deny` runs in CI. Unknown registries and
git sources are denied. Yanked crates are denied.

## Testing

Three tiers, cascading. Each higher tier includes the lower.

| Tier | Command | What | When |
|------|---------|------|------|
| 1 (fast) | `make test` | `cargo test --lib` | Between every edit; pre-commit |
| 2 (slow) | `make test-slow` | Tier 1 + all targets including ignored | Pre-PR |
| 3 (full) | `make test-full` | Tier 2 + dogfooding e2e | Pre-merge / nightly |

`make` (no target) = fmt-check + lint + Tier 1. Run it before every
commit. If it fails, do not commit.

## Workflow

elench follows a greenfield diamond protocol:

```
analyst -> architect -> adversary (gate 1) -> implementer -> auditor -> integrator
```

- The **analyst** writes specs (domain model, invariants, Gherkin).
- The **architect** derives interfaces, contracts, and ADRs.
- The **adversary** gates implementation -- no code until findings are
  resolved.
- The **implementer** builds within architect boundaries (TDD + BDD).
- The **auditor** measures test depth and gates PR.
- The **integrator** verifies cross-context interactions.

See `AGENTS.md` for full role dispatch and escalation paths.
Escalations are filed in `specs/escalations/`.

## Harness contract

Agents working on this repository emit claims following the rules in
`AGENTS.md` section "Harness contract". The key asymmetry: the harness
emits what it observed; the agent emits only what nothing else can
observe. The validator (ADR-0006) is implemented and enforces all
emission rules.

## Experiments

Three binding experiments -- all PASSED:

- **E0** (predicate ratio) -- PASSED 0.72 (threshold >= 0.30). Gates
  ADR-0004 and all implementation. PROCEED AS DESIGNED.
- **E1** (anchor survival) -- PASSED 99.4% correct, 0.6% wrong (all
  strategies USABLE). Gates the `anchor` object in
  `schema/claim.schema.json`. Proceed with multi.
- **E2** (build reproducibility) -- PASSED. Same-triple divergences all
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
6. Update `docs/` if setup, build, or CLI changes.
7. Update `README.md` if the project state table changes.

CI (`.github/workflows/`) runs automatically:
- **Push** -> Tier 1 (fmt-check + clippy + `cargo test --lib`)
- **PR** -> Tier 1 + Tier 2 (`cargo test --all-targets`)
- **Nightly** -> Tier 1 + Tier 2 + Tier 3 (fjall backend + coverage + dogfooding)
- **Feature matrix** -> compiles every feature combination (on Cargo.toml changes + nightly)

## License

Contributions are licensed under the [MIT License](LICENSE-MIT).
