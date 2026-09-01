# elench — AGENTS.md

This file serves two purposes: **workflow router** (how agents work on
this project) and **harness contract** (what agents emit into the claim
log). Both are binding. The harness contract is in force from now;
until a validator exists (ADR-0006), its rules are unevaluated, not
passed.

## Project state

| Field | Value |
|-------|-------|
| Mode | Greenfield (no source code; spec scaffold built) |
| Phase | Pre-implementation — E0 not yet run |
| Language | Rust (edition 2024, MSRV 1.85) |
| License | MIT |
| Crate count | 5 (elench, elench-claim, elench-envelope, elench-store, elench-gate) |
| Spec count | 7 feature files, 44 scenarios, 28 invariants (1 removed), 20 assumptions, 11 failure modes |
| ADR count | 7 (0001–0007; 0000 is template) |
| Experiment count | 3 (E0 not run, E1 not run, E2 not run) |
| Fidelity | All invariants NONE. First milestone: validator (ADR-0006). |

**Gate:** Do not begin implementation until E0 has run. E0's result
determines whether the gate layer should exist. See
`experiments/E0-predicate-ratio.md` for thresholds.

## Workflow routing

### Mode detection

- **Greenfield** (current): no source code. Forward direction:
  spec → code. The analyst writes specs (now scaffolded); the
  architect derives interfaces; the implementer builds.
- **Brownfield** (future): when source code exists and
  `specs/fidelity/INDEX.md` is stale. Entry point is `/bootstrap`.
  Analyst works in reverse (code → spec).

### Role dispatch

| Role | When | Output |
|------|------|--------|
| Analyst | Spec gap identified, or after E0 to refine specs | `specs/` artifacts |
| Architect | Specs validated, or interface/contract needed | `specs/architecture/` artifacts, ADRs |
| Adversary | Before implementation (gate 1) and after (findings) | `specs/findings/` |
| Implementer | Architecture approved, feature scoped | Code within architect boundaries |
| Auditor | After implementation, before PR | `specs/fidelity/` updates |
| Integrator | Feature spans 2+ bounded contexts | `specs/integration/` |

**Greenfield diamond:** analyst → architect → adversary (gate 1) →
implementer → auditor → integrator. Each role's output is the next
role's input. The adversary gates implementation; the auditor gates PR.

### Escalation paths

- Implementer → Architect (interface conflict) or Analyst (spec gap)
- Adversary → Architect (structural flaw) or Analyst (spec gap)
- Auditor → Implementer (shallow tests) or Architect (contract divergence)
- Integrator → Architect (cross-cutting issue)

Escalations are filed in `specs/escalations/`. The escalated-to role
addresses the issue, marks it RESOLVED, and the originator resumes from
the escalation point.

### Critical + High findings

Resolved before the next feature begins (may span multiple role
switches within the workflow cycle). Medium/Low get an issue filed.
No `TODO` / `FIXME` left in code.

## Harness contract

Rules for any agent or harness emitting claims into this repository. These are
enforcement targets, not etiquette. Where a rule says MUST, a validator should
reject violations; where no validator exists yet, that is tracked as debt, not
as permission.

### The asymmetry

The agent is the audited party. Any record the agent controls is a record the
agent can shape. So the split is:

- **The harness emits what it observed.** A process exited 0. A file changed.
  A gate transition fired. The agent is not consulted and cannot suppress it.
- **The agent emits only what nothing else can observe.** Stated premises.
  Rejected alternatives and why. Assumptions carried forward.

Default to harness-derived. Reach for agent-asserted only when the information
genuinely does not exist anywhere else.

### Rules

1. An agent MUST NOT emit a record with `origin.kind = "harness-observed"`.
   This is the single rule that keeps R2 meaningful.

2. An agent MUST NOT emit `kind = "verification"`. Only the harness verifies,
   and only from evidence it observed directly. An agent believing something
   passed is an assertion, not a verification.

3. An agent asserting `form = "predicate"` MUST supply an executable
   `expression`. Prose in a predicate slot is rejected at validation. Prose
   gates do not gate.

4. An `annotation` is powerless by construction. It is searchable and it is
   never read by policy. Emit them freely; expect nothing from them.

5. An agent MUST populate `dependsOn` with the claims it relied on. A claim
   with no premises asserts it was reached from nothing. That is occasionally
   true and usually a bug in the emission path.

6. Record a failure only when it changed some claim's status. A failed attempt
   that falsified nothing is noise. This filter is load-bearing; without it
   the log grows without bound and the signal ratio approaches zero.

7. An agent MUST NOT emit `residue-acceptance`. That record requires a human
   key and represents a person accepting named unevaluated gaps (R5).

### What good emission looks like

Bad — prose in a predicate slot, no premises, no evidence:

    form: predicate
    text: "Input validation is now handled correctly."

Better — an annotation honestly typed, and a separate real predicate:

    form: annotation
    text: "Chose rejection over coercion for empty input; coercion would
           have masked the upstream bug in the caller."
    dependsOn: [cl_a3f..., cl_91c...]

    form: predicate
    expression: { language: ..., source: "parse('') == Err(EmptyInput)" }

### Note on this section

If a validator does not exist, these rules are prose, and prose does not gate.
Treat every MUST above as an unimplemented gate. ADR-0006 tracks it.

## Testing tiers

Three tiers, cascading. Each higher tier includes the lower. Run `make`
(no target) before every commit.

| Tier | Command | What | When |
|------|---------|------|------|
| 1 (fast) | `make test` | `cargo test --lib` (fast unit + scenario tests) | Between every edit; pre-commit |
| 2 (slow) | `make test-slow` | Tier 1 + `cargo test --all-targets --include-ignored` (slow-marked + full) | Pre-PR |
| 3 (full) | `make test-full` | Tier 2 + e2e against real git repos (not yet configured) | Pre-merge / nightly |

`make` (no target) = `fmt-check` + `lint` + Tier 1. Run before every
commit.

## Build commands

```
make              # fmt-check + lint + Tier 1 (before every commit)
make fmt          # format all crates
make fmt-check    # check formatting without writing
make lint         # cargo clippy --all-targets -- -D warnings
make test         # Tier 1: cargo test --lib
make test-slow    # Tier 2: Tier 1 + all tests including ignored
make test-full    # Tier 3: Tier 2 + e2e (not yet configured)
make coverage     # cargo-llvm-cov --workspace --fail-under-lines 50
make clean        # cargo clean
```

## Language and guidelines

- **Rust** (edition 2024, MSRV 1.85). See
  `~/.config/opencode/guidelines/rust.md` for coding standards:
  `thiserror` for library errors, `anyhow` for binaries, `#[must_use]`
  on wrappers, `unwrap()`/`expect()` only in tests or with SAFETY
  comments, `cargo-llvm-cov` with 50% floor, `proptest` for property
  testing, `#[ignore = "slow: <reason>"]` for slow tests.
- **CI**: three-tier cascading. See `~/.config/opencode/guidelines/ci.md`.
- **Docs**: see `~/.config/opencode/guidelines/docs.md`. Spec documents
  in `specs/`, ADRs in `specs/architecture/adr/`.

## Reading order

1. `docs/problem.md` — requirements (R1–R7), binding constraints (BC1–BC3)
2. `experiments/E0-predicate-ratio.md` — the go/no-go measurement
3. `schema/claim.schema.json` — draft data model
4. `docs/anchoring.md` — the unsolved problem everything rests on
5. `docs/release-policy.md` — the gate shape
6. `specs/` — ubiquitous language, domain model, invariants, features,
   failure modes, assumptions, fidelity, cross-context, architecture
7. `specs/architecture/adr/` — ADR log (0000–0007)
