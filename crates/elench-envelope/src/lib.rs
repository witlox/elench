//! # elench-envelope
//!
//! DSSE envelopes carrying in-toto statements.
//!
//! Agent claims and build provenance share the same envelope format,
//! signing path, and verification library (ADR-0003). This crate
//! handles envelope signing, verification, and the distinction between
//! signer identity (from the envelope) and producer identity (from the
//! claim payload), which is a different thing (R2).
