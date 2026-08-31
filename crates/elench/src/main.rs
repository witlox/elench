//! # elench
//!
//! An evidence layer for repositories worked on by agents.
//!
//! elench records what was checked, to what depth, and what remains
//! unevaluated — as a durable claim log stored alongside the code in a
//! parallel git ref namespace. Claims are signed, append-only, and
//! revocable. An artifact's acceptability is a live evaluation against
//! the current claim log, not a signature frozen at release time.
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
