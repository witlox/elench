//! # elench
//!
//! An evidence layer for repositories — and the substrate that
//! replaces git (ADR-0001).
//!
//! elench records what was checked, to what depth, and what remains
//! unevaluated — as a durable claim log stored in a content-addressed
//! store. Claims are signed, append-only, and revocable. An artifact's
//! acceptability is a live evaluation against the current claim log,
//! not a signature frozen at release time.
//!
//! The git CLI works because elench synthesizes git-compatible objects
//! from the claim log (ADR-0002, ADR-0007). The projection is
//! read-only and deterministic (BC4). Humans use git; elench is
//! invisible.
//!
//! ## Status
//!
//! Pre-implementation. Nothing is built. Three binding experiments
//! (E0, E1, E2) gate whether the project should exist at all.
//! See `specs/architecture/adr/0006-validator-is-unimplemented-debt.md`
//! for why the validator is the first milestone, and `experiments/`
//! for the pre-registered experiments.

fn main() {
    // Pre-implementation. E0 must run before any claim-handling code.
    // See AGENTS.md and experiments/E0-predicate-ratio.md.
    eprintln!("elench: not yet implemented. Run E0 first.");
    std::process::exit(1);
}
