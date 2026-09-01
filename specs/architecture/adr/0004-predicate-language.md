# ADR-0004 — Predicate expression language: a small DSL of check primitives

**Status:** accepted
**Gated by:** E0 (COMPLETE — ratio 0.72, well above 0.30 threshold)
**Serves:** R3 (gate evaluable without build), INV-23 (expressions executable/deterministic/sandboxable)
**Supersedes:** ADR-0004 "Predicate expression language (UNDECIDED)"

## Context

`assertion.expression` must be executable, deterministic, sandboxable, and
writable by an agent under time pressure (INV-23). ADR-0004 was deferred to
E0 on the grounds that deciding a predicate language before seeing what
predicates agents actually write is designing against a guess.

E0 is complete. It extracted ~242 predicate expressions across two
repositories (kiseki, ~142 predicates; yoyo-evolve, ~100 predicates from
5 sessions). The extraction files are in
`tool_05bbec962001JHgFFYKtqYweNJ` (kiseki) and
`tool_05bad7ae3001kBm0OymVdX7fGK` (yoyo-evolve); the result summary is
in `experiments/E0-predicate-ratio-result.md`.

### What E0 found about predicate shapes

Every predicate in the corpus is a **flat expression**: a small number of
check primitives combined with comparison operators and boolean logic. No
predicate uses loops, recursion, data structure construction, or function
definitions. The shapes, classified by primary surface:

| Shape | Count | % | Representative expression |
|-------|-------|---|---------------------------|
| Code structure (grep source) | ~80 | ~33% | `grep -r "KISEKI_INTENT_EPOCH_ROTATE" crates/kiseki-log/src \| grep -E "8192\|default"` |
| Behavioral assertion (func→result) | ~75 | ~31% | `safe_truncate("héllo", 4) == "hél"` (no panic) |
| Test execution (test passes/count) | ~43 | ~18% | `cargo test -p kiseki-log` count == 255 and clippy clean |
| CLI output (cmd exit/output) | ~28 | ~12% | `yoyo("status").matches(r"elapsed\|time") && yoyo("status").matches(r"turn")` |
| File/content (grep non-source) | ~11 | ~5% | `grep -cE "IBM Storage Scale\|WEKA\|DAOS\|BeeGFS" docs/performance/competitive-targets.md >= 4` |
| Build/lint (clippy/fmt/build) | ~5 | ~2% | `cargo clippy -- -D warnings` exit 0 |

The shapes reduce to **four primitives**:

1. **`grep(pattern, path) → int`** — count of regex matches in a file.
   Covers code structure (~33%) and file/content (~5%) = ~38% of
   predicates. The dominant primitive in kiseki, where claims are mostly
   "env var X has default Y" or "function Z exists in source."

2. **`test(name) → {passed: bool}`** — a named test passes or fails.
   Covers behavioral assertions (~31%) and test execution (~18%) = ~49%
   of predicates. The dominant primitive in yoyo-evolve, where claims
   are mostly "function(args) == expected" and the agent already wrote
   the test as part of TDD.

3. **`run(cmd) → {exit: int, stdout: string, stderr: string}`** — run a
   command and inspect output or exit code. Covers CLI output (~12%)
   and build/lint (~2%) = ~14% of predicates. Examples: `yoyo changelog
   exits 0`, `cargo clippy -D warnings exits 0`, `gh issue view 253
   --json state contains "closed"`.

4. **`exists(path) → bool`** — a file exists. Covers ~1% of predicates.
   A special case of `grep(.+, path) >= 1` or `run("test -f path").exit
   == 0`, but common enough and semantically distinct enough to warrant
   its own primitive.

~10–15% of predicates combine two or more primitives with boolean
logic. No predicate combines more than three.

### Language features needed (deduplicated)

From all ~242 predicate expressions:

- **String literals** — file paths, command strings, pattern strings,
  test names, expected values.
- **Regex** — for `grep` and string matching (e.g., `IBM Storage
  Scale|WEKA|DAOS|BeeGFS`, `\d+.*tokens?/s`, `error\[E0308\]`).
- **File I/O (read-only)** — `grep` and `exists` read files; neither
  writes.
- **Process execution** — `test` runs `cargo test <name>`; `run` runs
  an arbitrary command. The validator controls the sandbox.
- **Integer** — match counts, test counts, exit codes.
- **Boolean** — pass/fail, exists/not-exists.
- **Comparison operators** — `==`, `!=`, `>=`, `<=`, `>`, `<`.
- **Boolean logic** — `&&` (and), `||` (or), `!` (not).
- **String matching** — `.contains(str)` (substring), `.matches(regex)`
  (full match).

**Not needed** — and present in zero predicates from the corpus: loops,
recursion, data structure construction (arrays, maps, records beyond
primitive results), function definitions, floating point, I/O writes,
exception handling, class/type systems, modules, imports.

## Decision

**A small, versioned DSL of check primitives — `elench-predicate-v1`.**

The DSL has four primitives — `grep`, `test`, `run`, `exists` — with
comparison operators, boolean logic, and string/regex matching. It is
not Turing-complete. It is not a general-purpose programming language.
It is the smallest language that expresses 100% of the predicate shapes
E0 found, and it is the smallest language the validator (ADR-0006) must
implement.

### Primitives

```
grep(pattern: regex, path: string) → int          // count of regex matches in file
test(name: string) → {passed: bool}               // cargo test <name> passes
run(cmd: string) → {exit: int, stdout: string}    // run command, inspect output
exists(path: string) → bool                       // file exists
```

### Operators

```
==  !=  >=  <=  >  <        // integer and boolean comparison
&&  ||  !                   // boolean logic
.contains(str) → bool       // substring match on string
.matches(regex) → bool      // regex match on string
```

### Grammar (informal)

An expression is one or more predicate checks combined with boolean
logic. Each check applies a primitive, accesses a field if needed, and
compares to an expected value.

```
expr        := or_expr
or_expr     := and_expr ('||' and_expr)*
and_expr    := not_expr ('&&' not_expr)*
not_expr    := '!' not_expr | comparison
comparison  := term (op term)?
term        := primitive ('.' field)*
primitive   := grep_call | test_call | run_call | exists_call | int_lit | str_lit | bool_lit
```

The `source` field of `assertion.expression` contains the DSL
expression as a string. The `language` field is `"elench-predicate-v1"`.
The `digest` field is the SHA-256 of the source, for deduplication: two
agents independently asserting the same predicate produce one claim
(INV-28).

### How the shapes map to the DSL

| Shape from E0 | DSL expression |
|---------------|----------------|
| `grep -cE "IBM Storage Scale\|WEKA\|DAOS\|BeeGFS" docs/performance/competitive-targets.md >= 4` | `grep(/IBM Storage Scale\|WEKA\|DAOS\|BeeGFS/, "docs/performance/competitive-targets.md") >= 4` |
| `safe_truncate("héllo", 4) == "hél"` (no panic) | `test("safe_truncate_utf8_boundary").passed` |
| `cargo test -p kiseki-log` count == 255 | `run("cargo test -p kiseki-log").stdout.contains("255 passed")` |
| `yoyo("status").matches(r"elapsed\|time")` | `run("yoyo status").stdout.matches(/elapsed\|time/)` |
| `cargo clippy -- -D warnings` exit 0 | `run("cargo clippy -- -D warnings").exit == 0` |
| `scripts/perf-gate.sh` exists | `exists("scripts/perf-gate.sh")` |
| `rg("fn set_model", "src/").len() >= 1` | `grep(/fn set_model/, "src/") >= 1` |
| `rg("auto_downgrade\|editor_model_map", "src/").len() == 0` | `grep(/auto_downgrade\|editor_model_map/, "src/") == 0` |

The behavioral-assertion shape (func(args) == expected) maps to
`test(name).passed` — the agent writes the test as part of its TDD
workflow, and the predicate references the test by name. The test
itself contains the function call and assertion. This is the
"existing test frameworks as the predicate" candidate, now a primitive
within the DSL rather than the entire language.

### Crate

`elench-predicate` (already anticipated in `module-graph.md` and
`enforcement-map.md`). The crate provides:

- **Parser**: `fn parse(source: &str) -> Result<Expression, ParseError>`
- **Evaluator**: `fn evaluate(expr: &Expression, ctx: &EvalContext) -> Result<EvalResult, EvalError>`
  where `EvalContext` carries the workspace root, sandbox configuration,
  and allowed-command list.
- **Validator integration**: `elench-claim::validate_claim` calls
  `elench-predicate::parse` to enforce INV-08 (predicate requires
  expression) and rejects expressions in unknown languages.

### Versioning

The DSL is versioned (`elench-predicate-v1`). Future changes (new
primitives, operator changes) produce `elench-predicate-v2` and a
new ADR. Existing claims reference `v1` and are evaluated by the `v1`
evaluator indefinitely. This is the same append-only discipline the
claim log follows (R1).

## Rejected alternatives

### Rego (OPA)

Mature, purpose-built for policy, sandboxed by design. **Lost because:**
(1) E0 found zero predicates that require policy-over-data evaluation —
Rego's strength. Every predicate is "check a file," "run a command," or
"run a test," none of which is Rego's native shape. (2) Rego has no
native primitives for process execution or file I/O; these would need
custom built-ins, at which point Rego is a wrapper around the DSL's
`run` and `grep` primitives. (3) Rego's evaluation model (partial sets,
default values, negation semantics) adds cognitive load for an agent
emitting predicates under time pressure — the corpus shows flat
comparisons, not set operations. (4) Rego's sandbox is designed for
data evaluation, not for running untrusted commands; the `run` primitive
requires a different sandboxing strategy.

### CUE

Strong at constraint expression and schema. **Lost because:** (1) E0
found zero predicates that require constraint solving — CUE's strength.
Predicates are flat checks, not constraints over data graphs. (2) CUE's
ecosystem is small, and its evaluation model (closed vs. open, defaults,
required) is more complex than the four primitives needed. (3) Like
Rego, CUE has no native primitives for process execution or file I/O.

### Starlark

Deterministic Python subset, well-sandboxed, easy for a model to emit.
**Lost because:** (1) Starlark is a general-purpose language
(Turing-complete, has loops, functions, data structures), and E0 found
zero predicates that need these features. (2) A general-purpose
language is harder to sandbox than a fixed set of four primitives — the
validator must reason about the full execution model, not just four
known operations. (3) Starlark's evaluator is larger and more complex
than the DSL's parser + four evaluators. (4) An agent emitting
Starlark has more ways to make mistakes (syntax errors, type errors,
runtime errors) than an agent emitting four primitives with a simple
grammar. Starlark is the strongest general-purpose candidate, but E0's
data shows the predicates don't need a general-purpose language.

### Existing test frameworks as the predicate

The predicate IS a named test; the expression is a test identifier.
Cheapest by far and reuses all existing infrastructure. **Lost because:**
(1) E0 found that only ~49% of predicates are test-runnable (behavioral
assertions where the agent writes a test, plus test-execution counts).
The remaining ~51% — grep checks on source files (~33%), CLI output
checks (~12%), file existence (~5%), build/lint checks (~2%) — are not
naturally test names. "grep a doc file for 4 terms" is not a test
name; "cargo clippy -D warnings exits 0" is not a test name; "gh issue
view 253 --json state is closed" is not a test name. (2) Wrapping these
in tests (writing a `fn doc_contains_ibm_weka_daos_beegfs()` that
greps a markdown file) is possible but defeats the purpose: the
predicate becomes "test passes" with no visibility into what the test
checks, and the ADR-0004 (undecided) weakness — "shallow tests asserting
almost nothing are structurally equivalent to no tests, and this route
makes that failure invisible rather than visible" — is realized. (3)
Build/lint checks (cargo clippy, cargo fmt --check) cannot be expressed
as in-project tests without circular execution (running `cargo test` to
check if `cargo clippy` passes). (4) The "test as predicate" approach
is a *subset* of the DSL — `test(name)` is one of four primitives. The
DSL adds `grep`, `run`, and `exists` to cover the remaining ~51% of
predicates, while keeping `test` as the primary primitive for
behavioral assertions.

### Two languages — one for policy over claims, one for assertions about code

Honest about the split between "check something" (grep, run, exists)
and "assert behavior" (test). **Lost because:** (1) The split is
between primitives, not between languages. Both "check" and "assert"
primitives live in one DSL with four primitives; there is no need for
two parsers, two evaluators, two sandbox configurations, or two
languages for the agent to learn. (2) ~10–15% of predicates combine
both kinds ("grep for X AND test Y passes"), which would require
cross-language composition — a significant implementation burden. (3)
E0 found no predicate that requires a policy language (claim-level
rules like "all claims in this tree must have origin.kind =
harness-observed"). Policy over claims is the gate's job (INV-09,
INV-13), and the gate evaluates claim *status* (computed from the log),
not predicate expressions.

## Consequences

### For the validator (ADR-0006)

The validator must implement a parser and evaluator for
`elench-predicate-v1`. This is small: four primitive evaluators
(`grep`, `test`, `run`, `exists`), a comparison layer, and a boolean
logic layer. The parser is a simple recursive-descent parser for the
grammar above — no operator precedence beyond `!` > comparison > `&&`
> `||`. The `run` primitive is the most dangerous: the validator must
sandbox process execution (whitelist allowed commands, restrict working
directory, capture stdout/stderr). The `grep` and `exists` primitives
are read-only and need only filesystem path restrictions (workspace
root). The `test` primitive runs `cargo test <name>`, which is
deterministic but may be slow — the validator may need a timeout.

The validator enforces INV-08 by rejecting any `form = "predicate"`
claim whose `expression.language` is not `elench-predicate-v1` (or a
known successor version). Prose in a predicate slot is rejected at
parse time: `parse("Input validation is now handled correctly.")` →
`ParseError`.

### For the schema

`schema/claim.schema.json` field `expression.language` is no longer a
free string. Validated claims must set `language =
"elench-predicate-v1"`. The `source` field contains the DSL
expression. The `digest` field is the SHA-256 of the source, enabling
deduplication (INV-28): two agents independently asserting the same
predicate produce one claim.

The schema itself does not need to change — `language` is already a
string, and the validator enforces the value. A future schema revision
may constrain `language` to an enum, but that is not required now.

### For the agent

The agent emits DSL expressions instead of free-form predicate text.
The grammar is small enough that an agent under time pressure can emit
it reliably: `grep(/pattern/, "path") >= N`, `test("name").passed`,
`run("cmd").exit == 0`, `exists("path")`, combined with `&&`, `||`,
`!`.

For behavioral assertions — the dominant shape in yoyo-evolve (~31% of
predicates) — the agent's TDD workflow already produces the test. The
predicate is the test name: `test("safe_truncate_utf8_boundary").passed`.
The agent does not need to write a separate assertion; the test IS the
assertion.

For code structure checks — the dominant shape in kiseki (~33% of
predicates) — the agent emits `grep(/pattern/, "src/") >= 1` or
`grep(/banned_pattern/, "src/") == 0`. These are one-liners that
require no test scaffolding.

### For the build phases

Phase 0 (validator) is unblocked. The build-phases.md entry
"Predicate evaluation | gated by E0 (ratio) + ADR-0004 (language)" can
now proceed: E0 is COMPLETE (0.72), ADR-0004 is accepted. The
`elench-predicate` crate (already listed in module-graph.md as
not-yet-created) is the first implementation target for the
predicate-evaluation portion of Phase 0.

The module graph should be updated: `elench-predicate` is no longer
"deferred" — it is the next crate to create, depended on by
`elench-claim` (for validation) and `elench-gate` (for predicate
evaluation, if the gate needs to re-evaluate predicates; per R3/INV-13
the gate evaluates claim *status*, not predicates, so `elench-gate` may
not need to depend on `elench-predicate` directly).

### What this decision does NOT do

- **Does not make predicates deep.** A predicate like
  `test("my_test").passed` has the same shallowness risk as the
  "existing test frameworks" candidate: the test might assert almost
  nothing. The DSL makes this *visible* — the predicate expression is
  `test("my_test").passed`, not a magic string — but it does not make
  it *deep*. Depth is the auditor's job (ADR-0006 + fidelity index),
  not the language's.

- **Does not handle non-deterministic predicates.** Some `run` commands
  (e.g., `gh issue view` — network-dependent) produce results that
  change over time. The DSL can *express* them, but the validator must
  decide whether to evaluate them live (result may differ between
  evaluations) or reject them as non-deterministic. The initial
  implementation should restrict `run` to deterministic, local
  commands (cargo, git, file utilities) and flag network-dependent
  predicates as `unevaluated` (INV-16). This is a validator policy
  decision, not a language decision.

- **Does not preclude future extensions.** If E1 (anchor survival) or
  later experiments reveal predicate shapes that the four primitives
  cannot express (e.g., "parse a tree and check that every path under
  `src/` has a claim"), the DSL gains a new primitive in
  `elench-predicate-v2`, with a new ADR. Existing `v1` claims are
  evaluated by the `v1` evaluator indefinitely (append-only
  discipline, R1). But E0's data gives no reason to expect this: every
  predicate found is a flat check.
