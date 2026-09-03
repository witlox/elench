//! Fjall-backed persistent content-addressed store (ADR-0008).
//!
//! Requires the `fjall-backend` feature. Uses fjall's LSM-tree
//! keyspaces with cross-keyspace atomic semantics:
//!
//! - `blobs`:  SHA-256 OID → blob data (`PersistMode::Buffer`)
//! - `trees`:  SHA-256 OID → serialized tree entries (`PersistMode::Buffer`)
//! - `claims`: `cl_` + SHA-256 OID → serialized claim JSON (`PersistMode::SyncAll`)
//!
//! INV-01: append-only. fjall's insert is idempotent for the same key.
//! INV-18: elench owns the store. fjall is pure Rust, embeddable.
//! INV-26: sole source of truth. All data is in one fjall database.

use std::path::Path;

use elench_claim::Claim;
use fjall::{KeyspaceCreateOptions, PersistMode};

use crate::{Oid, StoreBackend, StoreError, Tree, canonical_tree_bytes};

/// Persistent content-addressed store backed by fjall.
///
/// One fjall database at `path`, three keyspaces (blobs, trees,
/// claims). Claims use `PersistMode::SyncAll` (must survive crashes).
/// Blobs and trees use `PersistMode::Buffer` (content-addressed,
/// can be recomputed from source).
pub struct FjallStore {
    db: fjall::Database,
    blobs: fjall::Keyspace,
    trees: fjall::Keyspace,
    claims: fjall::Keyspace,
}

impl FjallStore {
    /// Open a persistent store at the given path.
    /// Creates the database and keyspaces if they don't exist.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::CorruptStore`] if the database cannot be
    /// opened or keyspaces cannot be created.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let db = fjall::Database::builder(path.as_ref())
            .open()
            .map_err(|e| StoreError::CorruptStore(format!("failed to open fjall: {e}")))?;

        let blobs = db
            .keyspace("blobs", KeyspaceCreateOptions::default)
            .map_err(|e| {
                StoreError::CorruptStore(format!("failed to create blobs keyspace: {e}"))
            })?;

        let trees = db
            .keyspace("trees", KeyspaceCreateOptions::default)
            .map_err(|e| {
                StoreError::CorruptStore(format!("failed to create trees keyspace: {e}"))
            })?;

        let claims = db
            .keyspace("claims", KeyspaceCreateOptions::default)
            .map_err(|e| {
                StoreError::CorruptStore(format!("failed to create claims keyspace: {e}"))
            })?;

        Ok(Self {
            db,
            blobs,
            trees,
            claims,
        })
    }

    /// Persist all data to disk.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::CorruptStore`] if persistence fails.
    pub fn flush(&self) -> Result<(), StoreError> {
        self.db
            .persist(PersistMode::SyncAll)
            .map_err(|e| StoreError::CorruptStore(format!("flush failed: {e}")))
    }
}

impl StoreBackend for FjallStore {
    fn store_blob(&mut self, data: &[u8]) -> Result<Oid, StoreError> {
        let oid = Oid::from_blob_data(data);
        let key = oid.as_str().as_bytes();

        if let Some(existing) = self
            .blobs
            .get(key)
            .map_err(|e| StoreError::CorruptStore(format!("blob read failed: {e}")))?
        {
            if existing.as_slice() != data {
                return Err(StoreError::ContentAddressingViolation {
                    oid: oid.as_str().to_string(),
                    reason: "blob data does not match existing OID".to_string(),
                });
            }
            return Ok(oid);
        }

        self.blobs
            .insert(key, data)
            .map_err(|e| StoreError::CorruptStore(format!("blob write failed: {e}")))?;

        self.db
            .persist(PersistMode::Buffer)
            .map_err(|e| StoreError::CorruptStore(format!("blob persist failed: {e}")))?;

        Ok(oid)
    }

    fn store_tree(&mut self, entries: Vec<crate::TreeEntry>) -> Result<Oid, StoreError> {
        let oid = Oid::from_tree_entries(&entries);
        let key = oid.as_str().as_bytes();
        let canonical = canonical_tree_bytes(&entries);

        if let Some(existing) = self
            .trees
            .get(key)
            .map_err(|e| StoreError::CorruptStore(format!("tree read failed: {e}")))?
        {
            if existing.as_slice() != canonical.as_slice() {
                return Err(StoreError::ContentAddressingViolation {
                    oid: oid.as_str().to_string(),
                    reason: "tree entries do not match existing OID".to_string(),
                });
            }
            return Ok(oid);
        }

        self.trees
            .insert(key, canonical.as_slice())
            .map_err(|e| StoreError::CorruptStore(format!("tree write failed: {e}")))?;

        self.db
            .persist(PersistMode::Buffer)
            .map_err(|e| StoreError::CorruptStore(format!("tree persist failed: {e}")))?;

        Ok(oid)
    }

    fn store_claim(&mut self, claim: &Claim) -> Result<String, StoreError> {
        let oid = elench_claim::ClaimId::from_content(claim).to_string();
        let key = oid.as_bytes();
        let json = serde_json::to_string(claim)
            .map_err(|e| StoreError::CorruptStore(format!("claim serialization failed: {e}")))?;

        if let Some(existing) = self
            .claims
            .get(key)
            .map_err(|e| StoreError::CorruptStore(format!("claim read failed: {e}")))?
        {
            if existing.as_slice() != json.as_bytes() {
                return Err(StoreError::ContentAddressingViolation {
                    oid: oid.clone(),
                    reason: "claim content does not match existing OID".to_string(),
                });
            }
            return Ok(oid);
        }

        self.claims
            .insert(key, json.as_bytes())
            .map_err(|e| StoreError::CorruptStore(format!("claim write failed: {e}")))?;

        // Claims must survive crashes — SyncAll (ADR-0008)
        self.db
            .persist(PersistMode::SyncAll)
            .map_err(|e| StoreError::CorruptStore(format!("claim persist failed: {e}")))?;

        Ok(oid)
    }

    fn read_blob(&self, oid: &Oid) -> Result<Vec<u8>, StoreError> {
        self.blobs
            .get(oid.as_str().as_bytes())
            .map_err(|e| StoreError::CorruptStore(format!("blob read failed: {e}")))?
            .map(|s| s.to_vec())
            .ok_or_else(|| StoreError::ObjectNotFound(oid.as_str().to_string()))
    }

    fn read_tree(&self, oid: &Oid) -> Result<Tree, StoreError> {
        let _data = self
            .trees
            .get(oid.as_str().as_bytes())
            .map_err(|e| StoreError::CorruptStore(format!("tree read failed: {e}")))?
            .ok_or_else(|| StoreError::ObjectNotFound(oid.as_str().to_string()))?;

        // Trees are stored as canonical bytes (mode space name null oid).
        // For Phase 1, we return an empty tree since deserialization
        // of canonical bytes back to TreeEntry is deferred.
        // The OID is correct and content-addressed.
        Ok(Tree {
            oid: oid.clone(),
            entries: Vec::new(),
        })
    }

    fn read_all_claims(&self) -> Result<Vec<Claim>, StoreError> {
        let mut claims = Vec::new();
        for guard in self.claims.iter() {
            let pair = guard.into_inner().map_err(|e| {
                StoreError::CorruptStore(format!("claim key_value read failed: {e}"))
            })?;
            let oid_str = String::from_utf8_lossy(&pair.0);
            let json = String::from_utf8_lossy(&pair.1);
            let claim: Claim = serde_json::from_str(&json).map_err(|e| {
                StoreError::CorruptStore(format!("failed to deserialize claim {oid_str}: {e}"))
            })?;
            claims.push(claim);
        }
        Ok(claims)
    }

    fn read_claims_for_tree(&self, tree: &Oid) -> Result<Vec<Claim>, StoreError> {
        let all = self.read_all_claims()?;
        Ok(all
            .into_iter()
            .filter(|c| c.anchor.tree == tree.as_str())
            .collect())
    }

    fn has_blob(&self, oid: &Oid) -> bool {
        self.blobs
            .contains_key(oid.as_str().as_bytes())
            .unwrap_or(false)
    }

    fn has_tree(&self, oid: &Oid) -> bool {
        self.trees
            .contains_key(oid.as_str().as_bytes())
            .unwrap_or(false)
    }

    fn has_claim(&self, claim_oid: &str) -> bool {
        self.claims
            .contains_key(claim_oid.as_bytes())
            .unwrap_or(false)
    }

    fn blob_count(&self) -> usize {
        self.blobs.len().unwrap_or(0)
    }

    fn tree_count(&self) -> usize {
        self.trees.len().unwrap_or(0)
    }

    fn claim_count(&self) -> usize {
        self.claims.len().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elench_claim::{
        Anchor, AnchorStrategy, AssertionForm, ClaimId, ClaimKind, Origin, OriginKind, Producer,
    };

    fn make_claim(id: &str) -> Claim {
        Claim {
            id: ClaimId::new(id).unwrap(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Annotation {
                text: "test".into(),
            },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: "test-producer".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: "abc123".into(),
                strategy: AnchorStrategy::PathRange,
                path: Some("src/main.rs".into()),
                range: Some([1, 10]),
                symbol: None,
                content_digest: None,
            },
            timestamp: 1_700_000_000,
            evidence: vec![],
            depends_on: vec![],
        }
    }

    fn temp_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "elench_fjall_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn scenario_fjall_store_blob_round_trip() {
        let dir = temp_dir();
        let mut store = FjallStore::open(&dir).unwrap();
        let oid = store.store_blob(b"hello world").unwrap();
        assert_eq!(oid.as_str().len(), 64);
        let data = store.read_blob(&oid).unwrap();
        assert_eq!(data, b"hello world");
        assert_eq!(store.blob_count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scenario_fjall_store_claim_round_trip() {
        let dir = temp_dir();
        let mut store = FjallStore::open(&dir).unwrap();
        let claim =
            make_claim("cl_0000000000000000000000000000000000000000000000000000000000000060");
        let oid = store.store_claim(&claim).unwrap();
        assert!(store.has_claim(&oid));
        assert_eq!(store.claim_count(), 1);
        let claims = store.read_all_claims().unwrap();
        assert_eq!(claims.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scenario_fjall_store_blob_idempotent() {
        let dir = temp_dir();
        let mut store = FjallStore::open(&dir).unwrap();
        let oid1 = store.store_blob(b"hello").unwrap();
        let oid2 = store.store_blob(b"hello").unwrap();
        assert_eq!(oid1, oid2);
        assert_eq!(store.blob_count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
