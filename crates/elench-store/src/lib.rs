//! # elench-store
//!
//! Git ref namespace operations for the claim log.
//!
//! Claims live in `refs/claims/<type>/<id>`, in the same object
//! database as the code (ADR-0001). Fetched by whoever wants them,
//! ignored by everyone else. No tree mutation, no synthesised commits,
//! no daemon. Everything is derivable from refs by a client-side binary.
