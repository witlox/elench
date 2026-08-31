//! # elench-gate
//!
//! Release gate evaluation — a predicate over claims, not a build.
//!
//! The gate is cheap and deterministic: a party with the refs and no
//! compute can evaluate the release predicate and get the same answer
//! as anyone else (R3). The build is a separate, expensive function of
//! a tree. Keeping them apart is what lets an artifact's acceptability
//! be a live evaluation, not a frozen signature (R4).
//!
//! See `docs/release-policy.md` for the gate's four conditions:
//! no falsified premise, bounded residue, origin floor, builder
//! agreement. Condition 4 is unavailable unless E2 passes.
