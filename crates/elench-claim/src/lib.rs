//! # elench-claim
//!
//! Claim data model, status computation, and validation.
//!
//! A claim is a signed assertion about a tree. Its status (passed,
//! failed, unevaluated) is NOT stored — it is computed by folding the
//! append-only claim log. This crate provides the data structures
//! matching `schema/claim.schema.json`, the log-folding status
//! computation, and validation of emission rules per AGENTS.md.
