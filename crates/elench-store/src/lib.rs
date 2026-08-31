//! # elench-store
//!
//! Content-addressed store: the substrate (ADR-0001).
//!
//! elench owns its own storage — blobs, trees, and claims, all
//! content-addressed. There is no git repository underneath, no
//! parallel ref namespace, no daemon. The store IS the history.
//!
//! The git projection (ADR-0002) synthesizes git-compatible objects
//! from the store on demand. It reads from this store and generates
//! git objects; it never writes back.
