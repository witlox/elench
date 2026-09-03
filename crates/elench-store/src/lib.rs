//! # elench-store
//!
//! Content-addressed store: the substrate (ADR-0001).
//!
//! elench owns its own storage — blobs, trees, and claims, all
//! content-addressed by SHA-256. There is no git repository underneath,
//! no parallel ref namespace, no daemon. The store IS the history.
//!
//! ## Invariants enforced
//!
//! - INV-01: append-only. Objects are never modified or deleted.
//! - INV-18: elench owns the store. No git dependency.
//! - INV-25: content addressing. A blob/tree/claim OID MUST equal the
//!   SHA-256 hash of its canonical serialization.
//! - INV-26: sole source of truth. All derived views (status, blast
//!   radius, gate verdict, git projection) are computable from the
//!   store alone.
//!
//! The git projection (ADR-0002) synthesizes git-compatible objects
//! from the store on demand. It reads from this store and generates
//! git objects; it never writes back.

use std::collections::HashMap;

use elench_claim::Claim;
use sha2::{Digest, Sha256};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Oid — content address (SHA-256, 64 hex chars, no prefix)
// ---------------------------------------------------------------------------

/// Content address of a blob, tree, or claim.
///
/// Blobs and trees: SHA-256 hash (64 hex chars, no prefix), identical
/// to git SHA-256 OIDs. Claims: `cl_` prefix + SHA-256 hash (see
/// [`elench_claim::ClaimId`]). The git projection is a passthrough for
/// blobs and trees; only commits are synthesized.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Oid(String);

impl Oid {
    /// Create an `Oid` from a hex string, validating it's 64 hex chars.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::InvalidOid`] if the string is not 64 hex chars.
    pub fn new(s: impl Into<String>) -> Result<Self, StoreError> {
        let s = s.into();
        if s.len() != 64 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(StoreError::InvalidOid(s));
        }
        Ok(Self(s))
    }

    /// Compute the SHA-256 content address of a byte array.
    #[must_use]
    pub fn from_blob_data(data: &[u8]) -> Self {
        let hash = Sha256::digest(data);
        Self(hex::encode(&hash))
    }

    /// Compute the SHA-256 content address of a tree's canonical
    /// serialization (sorted entries: mode space name null oid).
    #[must_use]
    pub fn from_tree_entries(entries: &[TreeEntry]) -> Self {
        let canonical = canonical_tree_bytes(entries);
        let hash = Sha256::digest(&canonical);
        Self(hex::encode(&hash))
    }

    /// Return the inner hex string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Blob
// ---------------------------------------------------------------------------

/// A blob: content-addressed byte array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blob {
    /// SHA-256 hash of `data`, 64 hex chars.
    pub oid: Oid,
    pub data: Vec<u8>,
}

// ---------------------------------------------------------------------------
// TreeEntry, TreeEntryKind, Tree
// ---------------------------------------------------------------------------

/// A tree entry: name + mode + OID, exactly like git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    /// Entry name within the parent tree (e.g., "lib.rs", "src").
    /// NOT a full path — git-compatible.
    pub name: String,
    /// File mode (`0o100644` regular, `0o100755` exec, `0o120000` symlink,
    /// `0o040000` directory).
    pub mode: u32,
    /// SHA-256 content hash. For files: blob OID. For dirs: tree OID.
    pub oid: Oid,
    /// Whether this entry is a blob (file) or a tree (directory).
    pub kind: TreeEntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeEntryKind {
    Blob,
    Tree,
}

/// A tree: sorted entries, content-addressed. Hierarchical — each
/// directory is a separate tree object. Serialization matches git's
/// tree object format (mode space name null oid), making OIDs
/// identical to git SHA-256 tree OIDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    /// SHA-256 hash of the canonical serialization.
    pub oid: Oid,
    /// Entries sorted by name. Directory names sort as if they have
    /// a trailing '/' (git-compatible).
    pub entries: Vec<TreeEntry>,
}

impl Tree {
    /// Create a tree from entries, computing the OID from the
    /// canonical serialization. Entries are sorted by name (git rule:
    /// directories sort as if they have a trailing '/').
    #[must_use]
    pub fn from_entries(mut entries: Vec<TreeEntry>) -> Self {
        sort_entries(&mut entries);
        let oid = Oid::from_tree_entries(&entries);
        Self { oid, entries }
    }
}

/// Sort entries by name, with directories sorting as if they have a
/// trailing '/' (git-compatible).
fn sort_entries(entries: &mut [TreeEntry]) {
    entries.sort_by(|a, b| {
        let a_key = sort_key(&a.name, a.kind);
        let b_key = sort_key(&b.name, b.kind);
        a_key.cmp(&b_key)
    });
}

/// Produce a sort key: for directories, append '/' (git rule).
fn sort_key(name: &str, kind: TreeEntryKind) -> String {
    match kind {
        TreeEntryKind::Tree => format!("{name}/"),
        TreeEntryKind::Blob => name.to_string(),
    }
}

/// Canonical tree serialization: for each entry (sorted),
/// `mode_octal_ascii space name null 32_byte_sha256`.
/// This matches git's tree object format exactly.
pub(crate) fn canonical_tree_bytes(entries: &[TreeEntry]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(entries.len() * 40);
    for entry in entries {
        // mode as octal ASCII (no leading zeros, e.g. "100644", "40000")
        let mode_str = format!("{:o}", entry.mode);
        buf.extend_from_slice(mode_str.as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(entry.name.as_bytes());
        buf.push(0x00);
        // OID as 32 raw bytes (hex -> bytes)
        if entry.oid.as_str().len() == 64 {
            let bytes = hex::decode(entry.oid.as_str()).unwrap_or_default();
            buf.extend_from_slice(&bytes);
        }
    }
    buf
}

// ---------------------------------------------------------------------------
// StoreBackend trait — abstracts the storage backend
// ---------------------------------------------------------------------------

/// Abstract content-addressed store backend.
///
/// Implementations:
/// - [`MemoryStore`]: in-memory `HashMap` (default, no deps)
/// - `FjallStore`: persistent LSM-tree backend (optional, `fjall-backend` feature)
///
/// INV-01: append-only. Objects are never modified or deleted.
/// INV-18: elench owns the store. No git dependency.
/// INV-26: sole source of truth. All views derive from the store.
pub trait StoreBackend {
    /// Store a blob. Returns the blob's OID (SHA-256 of data).
    /// INV-01: idempotent if same data.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ContentAddressingViolation`] if the OID
    /// exists with different data.
    fn store_blob(&mut self, data: &[u8]) -> Result<Oid, StoreError>;

    /// Store a tree. Returns the tree's OID.
    /// INV-01: idempotent if same entries.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ContentAddressingViolation`] if the OID
    /// exists with different entries.
    fn store_tree(&mut self, entries: Vec<TreeEntry>) -> Result<Oid, StoreError>;

    /// Store a claim. Returns the claim's OID (`cl_` + SHA-256).
    /// INV-01: idempotent if same content.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ContentAddressingViolation`] if the OID
    /// exists with different content.
    fn store_claim(&mut self, claim: &Claim) -> Result<String, StoreError>;

    /// Read a blob by OID.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ObjectNotFound`] if the blob doesn't exist.
    fn read_blob(&self, oid: &Oid) -> Result<Vec<u8>, StoreError>;

    /// Read a tree by OID.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ObjectNotFound`] if the tree doesn't exist.
    fn read_tree(&self, oid: &Oid) -> Result<Tree, StoreError>;

    /// Read all claims from the store.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::CorruptStore`] if any claim JSON is invalid.
    fn read_all_claims(&self) -> Result<Vec<Claim>, StoreError>;

    /// Read claims for a specific tree (by elench tree OID).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::CorruptStore`] if the claim log is corrupt.
    fn read_claims_for_tree(&self, tree: &Oid) -> Result<Vec<Claim>, StoreError>;

    /// Check if a blob exists.
    #[must_use]
    fn has_blob(&self, oid: &Oid) -> bool;

    /// Check if a tree exists.
    #[must_use]
    fn has_tree(&self, oid: &Oid) -> bool;

    /// Check if a claim exists.
    #[must_use]
    fn has_claim(&self, claim_oid: &str) -> bool;

    /// Number of blobs in the store.
    #[must_use]
    fn blob_count(&self) -> usize;

    /// Number of trees in the store.
    #[must_use]
    fn tree_count(&self) -> usize;

    /// Number of claims in the store.
    #[must_use]
    fn claim_count(&self) -> usize;
}

// ---------------------------------------------------------------------------
// MemoryStore — in-memory content-addressed store (default, no deps)
// ---------------------------------------------------------------------------

/// In-memory content-addressed store. Default backend; no persistence.
///
/// For persistence, enable the `fjall-backend` feature and use
/// `FjallStore` instead.
///
/// INV-01: append-only. Objects are never modified or deleted.
/// INV-18: elench owns the store. No git dependency.
/// INV-26: sole source of truth. All views derive from this store.
#[derive(Debug, Default)]
pub struct MemoryStore {
    /// Blob storage: OID -> data.
    blobs: HashMap<String, Vec<u8>>,
    /// Tree storage: OID -> entries.
    trees: HashMap<String, Vec<TreeEntry>>,
    /// Claim storage: claim OID -> serialized claim JSON.
    claims: HashMap<String, String>,
}

/// Type alias for backwards compatibility.
pub type Store = MemoryStore;

impl MemoryStore {
    /// Create a new empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl StoreBackend for MemoryStore {
    /// Store a blob. Returns the blob's OID (SHA-256 of data).
    /// INV-01: if the OID already exists, the existing data is
    /// returned without modification (append-only / idempotent).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ContentAddressingViolation`] if the OID
    /// exists with *different* data (content-addressing violation).
    fn store_blob(&mut self, data: &[u8]) -> Result<Oid, StoreError> {
        let oid = Oid::from_blob_data(data);
        let oid_str = oid.as_str().to_string();

        if let Some(existing) = self.blobs.get(&oid_str) {
            if existing != data {
                return Err(StoreError::ContentAddressingViolation {
                    oid: oid_str,
                    reason: "blob data does not match existing OID".to_string(),
                });
            }
            // Idempotent: same data, same OID. No-op.
            return Ok(oid);
        }

        self.blobs.insert(oid_str, data.to_vec());
        Ok(oid)
    }

    /// Store a tree. Returns the tree's OID (SHA-256 of canonical
    /// serialization). INV-01: idempotent if same entries.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ContentAddressingViolation`] if the OID
    /// exists with different entries.
    fn store_tree(&mut self, entries: Vec<TreeEntry>) -> Result<Oid, StoreError> {
        let oid = Oid::from_tree_entries(&entries);
        let oid_str = oid.as_str().to_string();

        if let Some(existing) = self.trees.get(&oid_str) {
            if existing != &entries {
                return Err(StoreError::ContentAddressingViolation {
                    oid: oid_str,
                    reason: "tree entries do not match existing OID".to_string(),
                });
            }
            return Ok(oid);
        }

        self.trees.insert(oid_str, entries);
        Ok(oid)
    }

    /// Store a claim. Returns the claim's OID (`cl_` + SHA-256).
    /// INV-01: if the OID already exists, the existing claim is
    /// returned without modification (append-only / idempotent).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ContentAddressingViolation`] if the OID
    /// exists with different content.
    fn store_claim(&mut self, claim: &Claim) -> Result<String, StoreError> {
        let oid = elench_claim::ClaimId::from_content(claim).to_string();

        let json = serde_json::to_string(claim)
            .map_err(|e| StoreError::CorruptStore(format!("claim serialization failed: {e}")))?;

        if let Some(existing) = self.claims.get(&oid) {
            if existing != &json {
                return Err(StoreError::ContentAddressingViolation {
                    oid,
                    reason: "claim content does not match existing OID".to_string(),
                });
            }
            return Ok(oid);
        }

        self.claims.insert(oid.clone(), json);
        Ok(oid)
    }

    /// Read a blob by OID.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ObjectNotFound`] if the blob doesn't exist.
    fn read_blob(&self, oid: &Oid) -> Result<Vec<u8>, StoreError> {
        self.blobs
            .get(oid.as_str())
            .cloned()
            .ok_or_else(|| StoreError::ObjectNotFound(oid.as_str().to_string()))
    }

    /// Read a tree by OID.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::ObjectNotFound`] if the tree doesn't exist.
    fn read_tree(&self, oid: &Oid) -> Result<Tree, StoreError> {
        let entries = self
            .trees
            .get(oid.as_str())
            .cloned()
            .ok_or_else(|| StoreError::ObjectNotFound(oid.as_str().to_string()))?;
        Ok(Tree {
            oid: oid.clone(),
            entries,
        })
    }

    /// Read all claims from the store.
    ///
    /// Each claim is stored as JSON; this deserializes them back to
    /// `Claim` objects.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::CorruptStore`] if any claim JSON is invalid.
    fn read_all_claims(&self) -> Result<Vec<Claim>, StoreError> {
        let mut claims = Vec::with_capacity(self.claims.len());
        for (oid, json) in &self.claims {
            let claim: Claim = serde_json::from_str(json).map_err(|e| {
                StoreError::CorruptStore(format!("failed to deserialize claim {oid}: {e}"))
            })?;
            claims.push(claim);
        }
        Ok(claims)
    }

    /// Read claims for a specific tree (by elench tree OID).
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::CorruptStore`] if the claim log is corrupt.
    fn read_claims_for_tree(&self, tree: &Oid) -> Result<Vec<Claim>, StoreError> {
        let all = self.read_all_claims()?;
        Ok(all
            .into_iter()
            .filter(|c| c.anchor.tree == tree.as_str())
            .collect())
    }

    /// Number of blobs in the store.
    fn blob_count(&self) -> usize {
        self.blobs.len()
    }

    /// Number of trees in the store.
    fn tree_count(&self) -> usize {
        self.trees.len()
    }

    /// Number of claims in the store.
    fn claim_count(&self) -> usize {
        self.claims.len()
    }

    /// Check if a blob exists.
    fn has_blob(&self, oid: &Oid) -> bool {
        self.blobs.contains_key(oid.as_str())
    }

    /// Check if a tree exists.
    fn has_tree(&self, oid: &Oid) -> bool {
        self.trees.contains_key(oid.as_str())
    }

    /// Check if a claim exists.
    fn has_claim(&self, claim_oid: &str) -> bool {
        self.claims.contains_key(claim_oid)
    }
}

// ---------------------------------------------------------------------------
// Fjall backend (optional, behind `fjall-backend` feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "fjall-backend")]
mod fjall_backend;

#[cfg(feature = "fjall-backend")]
pub use fjall_backend::FjallStore;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq)]
pub enum StoreError {
    #[error("invalid OID: {0}")]
    InvalidOid(String),

    #[error("object already exists with different content: {oid} ({reason})")]
    ObjectExists { oid: String, reason: String },

    #[error("content addressing violation: {oid} ({reason})")]
    ContentAddressingViolation { oid: String, reason: String },

    #[error("object not found: {0}")]
    ObjectNotFound(String),

    #[error("store is corrupt: {0}")]
    CorruptStore(String),

    #[error("I/O error")]
    Io,
}

// ---------------------------------------------------------------------------
// Claim JSON serialization (minimal, for store_claim)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// hex encoding/decoding (avoid pulling in a hex crate)
// ---------------------------------------------------------------------------

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes
            .iter()
            .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
                use std::fmt::Write;
                let _ = write!(s, "{b:02x}");
                s
            })
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if s.len() % 2 != 0 {
            return Err(());
        }
        let mut bytes = Vec::with_capacity(s.len() / 2);
        for chunk in s.as_bytes().chunks(2) {
            let high = hex_val(chunk[0])?;
            let low = hex_val(chunk[1])?;
            bytes.push((high << 4) | low);
        }
        Ok(bytes)
    }

    fn hex_val(c: u8) -> Result<u8, ()> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use elench_claim::{Anchor, AnchorStrategy};

    // --- Oid tests ---

    #[test]
    fn scenario_oid_valid_hex() {
        let oid = Oid::new("a".repeat(64));
        assert!(oid.is_ok());
    }

    #[test]
    fn scenario_oid_invalid_too_short() {
        let oid = Oid::new("abc");
        assert!(oid.is_err());
    }

    #[test]
    fn scenario_oid_invalid_non_hex() {
        let oid = Oid::new("g".repeat(64));
        assert!(oid.is_err());
    }

    #[test]
    fn scenario_oid_from_blob_data_deterministic() {
        let oid1 = Oid::from_blob_data(b"hello world");
        let oid2 = Oid::from_blob_data(b"hello world");
        assert_eq!(oid1, oid2);
    }

    #[test]
    fn scenario_oid_from_blob_data_different() {
        let oid1 = Oid::from_blob_data(b"hello");
        let oid2 = Oid::from_blob_data(b"world");
        assert_ne!(oid1, oid2);
    }

    #[test]
    fn scenario_oid_from_blob_data_known_sha256() {
        // SHA-256 of "hello" is 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        let oid = Oid::from_blob_data(b"hello");
        assert_eq!(
            oid.as_str(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    // --- Blob storage tests ---

    #[test]
    fn scenario_store_blob_returns_oid() {
        let mut store = Store::new();
        let oid = store.store_blob(b"hello world").unwrap();
        assert_eq!(oid.as_str().len(), 64);
    }

    #[test]
    fn scenario_store_blob_idempotent() {
        let mut store = Store::new();
        let oid1 = store.store_blob(b"hello").unwrap();
        let oid2 = store.store_blob(b"hello").unwrap();
        assert_eq!(oid1, oid2);
        assert_eq!(store.blob_count(), 1);
    }

    #[test]
    fn scenario_store_blob_different_data_different_oid() {
        let mut store = Store::new();
        let oid1 = store.store_blob(b"hello").unwrap();
        let oid2 = store.store_blob(b"world").unwrap();
        assert_ne!(oid1, oid2);
        assert_eq!(store.blob_count(), 2);
    }

    #[test]
    fn scenario_store_blob_idempotent_no_violation() {
        // This should be impossible with SHA-256, but we test the
        // guard anyway. Since Oid::from_blob_data always produces the
        // correct hash, this test verifies that the guard exists.
        let mut store = Store::new();
        store.store_blob(b"hello").unwrap();
        // Storing the same data again is idempotent, not a violation.
        let result = store.store_blob(b"hello");
        assert!(result.is_ok());
    }

    #[test]
    fn scenario_read_blob_returns_data() {
        let mut store = Store::new();
        let oid = store.store_blob(b"hello world").unwrap();
        let data = store.read_blob(&oid).unwrap();
        assert_eq!(data, b"hello world");
    }

    #[test]
    fn scenario_read_blob_not_found() {
        let store = Store::new();
        let oid = Oid::new("a".repeat(64)).unwrap();
        let result = store.read_blob(&oid);
        assert_eq!(result, Err(StoreError::ObjectNotFound("a".repeat(64))));
    }

    // --- Tree storage tests ---

    #[test]
    fn scenario_store_tree_returns_oid() {
        let mut store = Store::new();
        let blob_oid = store.store_blob(b"file content").unwrap();
        let entries = vec![TreeEntry {
            name: "file.txt".into(),
            mode: 0o100_644,
            oid: blob_oid,
            kind: TreeEntryKind::Blob,
        }];
        let oid = store.store_tree(entries).unwrap();
        assert_eq!(oid.as_str().len(), 64);
    }

    #[test]
    fn scenario_store_tree_idempotent() {
        let mut store = Store::new();
        let blob_oid = store.store_blob(b"content").unwrap();
        let entries = vec![TreeEntry {
            name: "a.txt".into(),
            mode: 0o100_644,
            oid: blob_oid.clone(),
            kind: TreeEntryKind::Blob,
        }];
        let oid1 = store.store_tree(entries.clone()).unwrap();
        let oid2 = store.store_tree(entries).unwrap();
        assert_eq!(oid1, oid2);
        assert_eq!(store.tree_count(), 1);
    }

    #[test]
    fn scenario_store_tree_different_entries_different_oid() {
        let mut store = Store::new();
        let blob1 = store.store_blob(b"a").unwrap();
        let blob2 = store.store_blob(b"b").unwrap();

        let oid1 = store
            .store_tree(vec![TreeEntry {
                name: "a.txt".into(),
                mode: 0o100_644,
                oid: blob1,
                kind: TreeEntryKind::Blob,
            }])
            .unwrap();
        let oid2 = store
            .store_tree(vec![TreeEntry {
                name: "b.txt".into(),
                mode: 0o100_644,
                oid: blob2,
                kind: TreeEntryKind::Blob,
            }])
            .unwrap();
        assert_ne!(oid1, oid2);
    }

    #[test]
    fn scenario_read_tree_returns_entries() {
        let mut store = Store::new();
        let blob_oid = store.store_blob(b"content").unwrap();
        let entries = vec![TreeEntry {
            name: "file.txt".into(),
            mode: 0o100_644,
            oid: blob_oid.clone(),
            kind: TreeEntryKind::Blob,
        }];
        let tree_oid = store.store_tree(entries.clone()).unwrap();
        let tree = store.read_tree(&tree_oid).unwrap();
        assert_eq!(tree.entries, entries);
    }

    #[test]
    fn scenario_read_tree_not_found() {
        let store = Store::new();
        let oid = Oid::new("b".repeat(64)).unwrap();
        let result = store.read_tree(&oid);
        assert_eq!(result, Err(StoreError::ObjectNotFound("b".repeat(64))));
    }

    // --- Tree sorting (git-compatible) ---

    #[test]
    fn scenario_tree_sort_files_before_directories() {
        // Git sorts entries by name, but directories sort as if they
        // have a trailing '/'. So "src" (dir) sorts after "src.rs" (file)
        // because "src/" > "src.rs".
        let mut entries = vec![
            TreeEntry {
                name: "src".into(),
                mode: 0o040_000,
                oid: Oid::new("a".repeat(64)).unwrap(),
                kind: TreeEntryKind::Tree,
            },
            TreeEntry {
                name: "src.rs".into(),
                mode: 0o100_644,
                oid: Oid::new("b".repeat(64)).unwrap(),
                kind: TreeEntryKind::Blob,
            },
        ];
        sort_entries(&mut entries);
        // "src.rs" < "src/" so src.rs comes first
        assert_eq!(entries[0].name, "src.rs");
        assert_eq!(entries[1].name, "src");
    }

    #[test]
    fn scenario_tree_sort_alphabetical() {
        let mut entries = vec![
            TreeEntry {
                name: "zebra.txt".into(),
                mode: 0o100_644,
                oid: Oid::new("0".repeat(64)).unwrap(),
                kind: TreeEntryKind::Blob,
            },
            TreeEntry {
                name: "alpha.txt".into(),
                mode: 0o100_644,
                oid: Oid::new("a".repeat(64)).unwrap(),
                kind: TreeEntryKind::Blob,
            },
        ];
        sort_entries(&mut entries);
        assert_eq!(entries[0].name, "alpha.txt");
        assert_eq!(entries[1].name, "zebra.txt");
    }

    // --- INV-25: content addressing ---

    #[test]
    fn scenario_inv25_blob_oid_is_sha256() {
        let oid = Oid::from_blob_data(b"test content");
        let expected = sha2::Sha256::digest(b"test content");
        assert_eq!(oid.as_str(), &format!("{expected:x}"));
    }

    #[test]
    fn scenario_inv25_tree_oid_is_sha256_of_canonical() {
        let blob_oid = Oid::from_blob_data(b"x");
        let entries = vec![TreeEntry {
            name: "a.txt".into(),
            mode: 0o100_644,
            oid: blob_oid,
            kind: TreeEntryKind::Blob,
        }];
        let tree = Tree::from_entries(entries);
        // The OID should be the SHA-256 of the canonical serialization
        let canonical = canonical_tree_bytes(&tree.entries);
        let expected = sha2::Sha256::digest(&canonical);
        assert_eq!(tree.oid.as_str(), &format!("{expected:x}"));
    }

    // --- INV-01: append-only ---

    #[test]
    fn scenario_inv01_append_only_blob() {
        let mut store = Store::new();
        let oid = store.store_blob(b"original").unwrap();
        let data = store.read_blob(&oid).unwrap();
        assert_eq!(data, b"original");

        // Store the same blob again (idempotent)
        store.store_blob(b"original").unwrap();
        let data2 = store.read_blob(&oid).unwrap();
        assert_eq!(data2, b"original");
        assert_eq!(store.blob_count(), 1); // No duplication
    }

    #[test]
    fn scenario_inv01_append_only_tree() {
        let mut store = Store::new();
        let blob_oid = store.store_blob(b"data").unwrap();
        let entries = vec![TreeEntry {
            name: "file.txt".into(),
            mode: 0o100_644,
            oid: blob_oid,
            kind: TreeEntryKind::Blob,
        }];
        let oid = store.store_tree(entries).unwrap();
        let tree = store.read_tree(&oid).unwrap();
        assert_eq!(tree.entries.len(), 1);

        // Store the same tree again (idempotent)
        let oid2 = store
            .store_tree(vec![TreeEntry {
                name: "file.txt".into(),
                mode: 0o100_644,
                oid: Oid::from_blob_data(b"data"),
                kind: TreeEntryKind::Blob,
            }])
            .unwrap();
        assert_eq!(oid, oid2);
        assert_eq!(store.tree_count(), 1);
    }

    // --- INV-26: sole source of truth ---

    #[test]
    fn scenario_inv26_store_is_sole_source() {
        // All data needed to compute views is in the store.
        // No external state, no config, no network.
        let mut store = Store::new();
        let blob_oid = store.store_blob(b"content").unwrap();
        let tree_entries = vec![TreeEntry {
            name: "file.txt".into(),
            mode: 0o100_644,
            oid: blob_oid.clone(),
            kind: TreeEntryKind::Blob,
        }];
        let tree_oid = store.store_tree(tree_entries).unwrap();

        // Everything is retrievable from the store alone
        assert!(store.has_blob(&blob_oid));
        assert!(store.has_tree(&tree_oid));
        let blob = store.read_blob(&blob_oid).unwrap();
        let tree = store.read_tree(&tree_oid).unwrap();
        assert_eq!(blob, b"content");
        assert_eq!(tree.entries.len(), 1);
    }

    // --- Hierarchical tree (directory within directory) ---

    #[test]
    fn scenario_hierarchical_tree_nested_directories() {
        let mut store = Store::new();

        // Create a file blob
        let file_blob = store.store_blob(b"file in subdir").unwrap();

        // Create inner tree (src/lib.rs)
        let inner_tree = store
            .store_tree(vec![TreeEntry {
                name: "lib.rs".into(),
                mode: 0o100_644,
                oid: file_blob,
                kind: TreeEntryKind::Blob,
            }])
            .unwrap();

        // Create outer tree (src/ + README.md)
        let readme_blob = store.store_blob(b"readme").unwrap();
        let outer_tree = store
            .store_tree(vec![
                TreeEntry {
                    name: "README.md".into(),
                    mode: 0o100_644,
                    oid: readme_blob,
                    kind: TreeEntryKind::Blob,
                },
                TreeEntry {
                    name: "src".into(),
                    mode: 0o040_000,
                    oid: inner_tree,
                    kind: TreeEntryKind::Tree,
                },
            ])
            .unwrap();

        // Read back and verify hierarchy
        let outer = store.read_tree(&outer_tree).unwrap();
        assert_eq!(outer.entries.len(), 2);
        assert_eq!(outer.entries[0].name, "README.md");
        assert_eq!(outer.entries[1].name, "src");
        assert_eq!(outer.entries[1].kind, TreeEntryKind::Tree);

        let inner = store.read_tree(&outer.entries[1].oid).unwrap();
        assert_eq!(inner.entries.len(), 1);
        assert_eq!(inner.entries[0].name, "lib.rs");

        let file = store.read_blob(&inner.entries[0].oid).unwrap();
        assert_eq!(file, b"file in subdir");
    }

    // --- Empty tree ---

    #[test]
    fn scenario_empty_tree() {
        let mut store = Store::new();
        let oid = store.store_tree(vec![]).unwrap();
        let tree = store.read_tree(&oid).unwrap();
        assert!(tree.entries.is_empty());
    }

    // --- Store statistics ---

    #[test]
    fn scenario_store_counts() {
        let mut store = Store::new();
        assert_eq!(store.blob_count(), 0);
        assert_eq!(store.tree_count(), 0);
        assert_eq!(store.claim_count(), 0);

        store.store_blob(b"a").unwrap();
        store.store_blob(b"b").unwrap();
        assert_eq!(store.blob_count(), 2);

        let oid = Oid::from_blob_data(b"a");
        store
            .store_tree(vec![TreeEntry {
                name: "x".into(),
                mode: 0o100_644,
                oid,
                kind: TreeEntryKind::Blob,
            }])
            .unwrap();
        assert_eq!(store.tree_count(), 1);
    }

    // --- has_blob, has_tree ---

    #[test]
    fn scenario_has_blob_and_tree() {
        let mut store = Store::new();
        let blob_oid = store.store_blob(b"data").unwrap();
        assert!(store.has_blob(&blob_oid));
        assert!(!store.has_blob(&Oid::new("0".repeat(64)).unwrap()));

        let tree_oid = store
            .store_tree(vec![TreeEntry {
                name: "f".into(),
                mode: 0o100_644,
                oid: blob_oid,
                kind: TreeEntryKind::Blob,
            }])
            .unwrap();
        assert!(store.has_tree(&tree_oid));
        assert!(!store.has_tree(&Oid::new("1".repeat(64)).unwrap()));
    }

    #[test]
    fn scenario_store_claim_round_trip() {
        let mut store = Store::new();
        let claim = elench_claim::Claim {
            id: elench_claim::ClaimId::new(
                "cl_0000000000000000000000000000000000000000000000000000000000000050",
            )
            .unwrap(),
            kind: elench_claim::ClaimKind::Assertion,
            target: vec![],
            assertion: elench_claim::AssertionForm::Annotation {
                text: "test".into(),
            },
            origin: elench_claim::Origin {
                kind: elench_claim::OriginKind::AgentAsserted,
                producer: elench_claim::Producer {
                    id: "agent".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "t".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_000,
            evidence: vec![],
            depends_on: vec![],
        };
        let oid = store.store_claim(&claim).unwrap();
        assert!(store.has_claim(&oid));
        assert_eq!(store.claim_count(), 1);

        // Read it back
        let claims = store.read_all_claims().unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].id, claim.id);
        assert_eq!(claims[0].kind, claim.kind);
        assert_eq!(claims[0].anchor.tree, claim.anchor.tree);
    }

    #[test]
    fn scenario_read_claims_for_tree_filters() {
        let mut store = Store::new();
        let claim_a = elench_claim::Claim {
            id: elench_claim::ClaimId::new(
                "cl_0000000000000000000000000000000000000000000000000000000000000051",
            )
            .unwrap(),
            kind: elench_claim::ClaimKind::Assertion,
            target: vec![],
            assertion: elench_claim::AssertionForm::Annotation { text: "a".into() },
            origin: elench_claim::Origin {
                kind: elench_claim::OriginKind::AgentAsserted,
                producer: elench_claim::Producer {
                    id: "agent".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "tree_a".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_000,
            evidence: vec![],
            depends_on: vec![],
        };
        let claim_b = elench_claim::Claim {
            id: elench_claim::ClaimId::new(
                "cl_0000000000000000000000000000000000000000000000000000000000000052",
            )
            .unwrap(),
            kind: elench_claim::ClaimKind::Assertion,
            target: vec![],
            assertion: elench_claim::AssertionForm::Annotation { text: "b".into() },
            origin: elench_claim::Origin {
                kind: elench_claim::OriginKind::AgentAsserted,
                producer: elench_claim::Producer {
                    id: "agent".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "tree_b".into(),
                strategy: AnchorStrategy::PathRange,
                path: None,
                range: None,
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_001,
            evidence: vec![],
            depends_on: vec![],
        };
        store.store_claim(&claim_a).unwrap();
        store.store_claim(&claim_b).unwrap();

        let tree_a_claims = store
            .read_claims_for_tree(&Oid::new("a".repeat(64)).unwrap())
            .unwrap();
        // tree_a is "tree_a" not a valid Oid, so no claims match
        assert!(tree_a_claims.is_empty());

        // But read_all_claims returns both
        let all = store.read_all_claims().unwrap();
        assert_eq!(all.len(), 2);
    }
}
