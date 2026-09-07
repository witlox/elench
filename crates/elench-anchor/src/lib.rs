//! # elench-anchor
//!
//! Multi-strategy anchor resolution (E1: strategy=multi).
//!
//! An anchor points at code within a tree. Code moves — renamed,
//! reformatted, semantically edited. E1 measured survival rates
//! for three strategies (path-range, symbol, content-digest) and
//! found all usable. The multi strategy records all three and
//! resolves by agreement, reporting `Degraded` when they disagree.
//!
//! ## Resolution
//!
//! Given an anchor and a store containing the tree, `resolve` tries
//! all three strategies by actually traversing the tree:
//! - **Path-range**: navigate to the path in the tree, read the blob,
//!   verify the line range `[start, end]` is within the blob. Dies on
//!   rename or reformat that moves the code outside the range.
//! - **Symbol**: traverse all blobs in the tree, search for a
//!   definition pattern (`fn name`, `def name`, `struct name`, etc.).
//!   Dies on rename.
//! - **Content-digest**: traverse all blobs, compute SHA-256 of each,
//!   compare against the anchor's `content_digest`. Also checks
//!   `has_blob(digest)` directly. Dies on semantic edit.
//!
//! If all agree → `Correct`. If one fails but others agree → `Correct`
//! (the failed strategy is noted). If active strategies disagree →
//! `Degraded` (reported to policy, gate may treat differently). If all
//! fail → `Failed`.
//!
//! ## Wrong-resolution vs Failed
//!
//! **Wrong-resolution** is the outcome that matters (E1 pre-registered
//! threshold: >2% disqualifies). A wrong resolution silently points at
//! the wrong code — the blast radius is fiction. A failed resolution is
//! loud and recoverable.
//!
//! The multi strategy reduces wrong-resolution by requiring agreement.
//! When strategies disagree, the result is `Degraded` — not a silent
//! pick of one strategy.

use elench_claim::{Anchor, AnchorStrategy};
use elench_store::{Oid, StoreBackend, TreeEntryKind};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Resolution result
// ---------------------------------------------------------------------------

/// The result of resolving an anchor against a tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// All strategies agree: the anchor resolves to the correct code.
    Correct {
        /// The path the anchor resolves to (may differ from original if moved).
        path: String,
        /// Strategies that agreed (subset of: path-range, symbol, content-digest).
        strategies: Vec<StrategyName>,
        /// Strategies that failed (noted but not blocking if others agree).
        failed: Vec<StrategyName>,
    },
    /// Strategies disagree: the anchor is degraded. Policy may treat
    /// this differently (e.g., as unevaluated or as a warning).
    Degraded {
        /// What each strategy resolved to.
        disagreements: Vec<StrategyDisagreement>,
    },
    /// All strategies failed to resolve: the anchored code is gone
    /// or unrecognizable. Loud, recoverable.
    Failed {
        /// What each strategy reported.
        reasons: Vec<StrategyFailure>,
    },
    /// One or more strategies resolved to the WRONG code silently.
    /// This is the outcome that matters (E1: >2% disqualifies).
    WrongResolution {
        /// The strategy that resolved to wrong code.
        strategy: StrategyName,
        /// Where it resolved to.
        resolved_to: String,
    },
}

// ---------------------------------------------------------------------------
// Reconciliation — detect anchors that no longer resolve
// ---------------------------------------------------------------------------

/// A claim whose anchor no longer resolves after a tree change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftedClaim {
    /// The claim ID.
    pub claim_id: String,
    /// The tree OID the claim is anchored to.
    pub tree: String,
    /// The anchor's path (if any).
    pub path: Option<String>,
    /// The anchor's symbol (if any).
    pub symbol: Option<String>,
    /// The resolution result (`Failed`, `Degraded`, or `WrongResolution`).
    pub resolution: Resolution,
}

/// Result of reconciling claims against a tree.
///
/// A reconciliation pass detects claims whose anchors no longer
/// resolve after a tree change. It reports affected claims — it
/// does NOT auto-fix. The human (or agent) must re-anchor or
/// falsify.
///
/// This is the reconciliation pass referenced in A-A02 and
/// FM-P2-03. It does not arbitrate who writes; it reports what
/// drifted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationReport {
    /// The tree OID that was reconciled.
    pub tree: String,
    /// Claims whose anchors still resolve correctly.
    pub intact: Vec<String>,
    /// Claims whose anchors no longer resolve.
    pub drifted: Vec<DriftedClaim>,
}

impl ReconciliationReport {
    /// Number of drifted claims.
    #[must_use]
    pub fn drift_count(&self) -> usize {
        self.drifted.len()
    }

    /// Number of intact claims.
    #[must_use]
    pub fn intact_count(&self) -> usize {
        self.intact.len()
    }

    /// True if any claims drifted.
    #[must_use]
    pub fn has_drift(&self) -> bool {
        !self.drifted.is_empty()
    }
}

/// Reconcile claims against a tree stored in `store`.
///
/// For each claim anchored to `tree`, resolve the anchor using
/// the multi strategy against the store. Claims that fail to resolve
/// or are degraded are reported as drifted.
///
/// This is a read-only operation — it does not modify claims or
/// the tree. The human (or agent) must re-anchor or falsify
/// drifted claims.
///
/// # Errors
///
/// Returns [`AnchorError`] if any anchor's strategy is not `Multi`.
pub fn reconcile(
    tree: &str,
    log: &[elench_claim::Claim],
    store: &dyn StoreBackend,
) -> Result<ReconciliationReport, AnchorError> {
    let tree_claims: Vec<&elench_claim::Claim> =
        log.iter().filter(|c| c.anchor.tree == tree).collect();

    let mut intact = Vec::new();
    let mut drifted = Vec::new();

    for claim in &tree_claims {
        match resolve(&claim.anchor, store)? {
            Resolution::Correct { .. } => {
                intact.push(claim.id.as_str().to_string());
            }
            res @ (Resolution::Degraded { .. }
            | Resolution::Failed { .. }
            | Resolution::WrongResolution { .. }) => {
                drifted.push(DriftedClaim {
                    claim_id: claim.id.as_str().to_string(),
                    tree: claim.anchor.tree.clone(),
                    path: claim.anchor.path.clone(),
                    symbol: claim.anchor.symbol.clone(),
                    resolution: res,
                });
            }
        }
    }

    Ok(ReconciliationReport {
        tree: tree.to_string(),
        intact,
        drifted,
    })
}

/// Which strategy was used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrategyName {
    PathRange,
    Symbol,
    ContentDigest,
}

impl std::fmt::Display for StrategyName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathRange => write!(f, "path-range"),
            Self::Symbol => write!(f, "symbol"),
            Self::ContentDigest => write!(f, "content-digest"),
        }
    }
}

/// A disagreement between strategies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyDisagreement {
    pub strategy: StrategyName,
    pub resolved_to: String,
}

/// A strategy that failed to resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyFailure {
    pub strategy: StrategyName,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AnchorError {
    #[error("anchor has no path (required for path-range strategy)")]
    NoPath,

    #[error("anchor has no symbol (required for symbol strategy)")]
    NoSymbol,

    #[error("anchor has no content digest (required for content-digest strategy)")]
    NoContentDigest,

    #[error("anchor strategy is not multi: {0:?}")]
    NotMulti(AnchorStrategy),

    #[error("invalid tree OID: {0}")]
    InvalidTreeOid(String),
}

// ---------------------------------------------------------------------------
// Resolve — multi-strategy anchor resolution against a store
// ---------------------------------------------------------------------------

/// Resolve an anchor against a tree stored in `store` using the multi
/// strategy.
///
/// Tries all three strategies (path-range, symbol, content-digest) by
/// actually traversing the tree in the store. Resolves by agreement:
/// all agree → `Correct`, disagreement → `Degraded`, all fail →
/// `Failed`.
///
/// # Errors
///
/// Returns [`AnchorError`] if the anchor's strategy is not `Multi` or
/// the tree OID is invalid.
pub fn resolve(anchor: &Anchor, store: &dyn StoreBackend) -> Result<Resolution, AnchorError> {
    if anchor.strategy != AnchorStrategy::Multi {
        return Err(AnchorError::NotMulti(anchor.strategy.clone()));
    }

    if anchor.tree.is_empty() {
        return Err(AnchorError::InvalidTreeOid("(empty)".into()));
    }
    // Validate the tree OID format (may not be in the store, but must
    // be a valid 64-hex-char string for strategies that need it).
    if Oid::new(&anchor.tree).is_err() {
        return Err(AnchorError::InvalidTreeOid(anchor.tree.clone()));
    }

    let path_result = resolve_path_range(anchor, store);
    let symbol_result = resolve_symbol(anchor, store);
    let content_result = resolve_content_digest(anchor, store);

    let mut resolved: Vec<(StrategyName, String)> = Vec::new();
    let mut failed: Vec<StrategyFailure> = Vec::new();

    match path_result {
        StrategyOutcome::Resolved(p) => resolved.push((StrategyName::PathRange, p)),
        StrategyOutcome::Failed(r) => failed.push(StrategyFailure {
            strategy: StrategyName::PathRange,
            reason: r,
        }),
        StrategyOutcome::Wrong(p) => {
            return Ok(Resolution::WrongResolution {
                strategy: StrategyName::PathRange,
                resolved_to: p,
            });
        }
    }

    match symbol_result {
        StrategyOutcome::Resolved(p) => resolved.push((StrategyName::Symbol, p)),
        StrategyOutcome::Failed(r) => failed.push(StrategyFailure {
            strategy: StrategyName::Symbol,
            reason: r,
        }),
        StrategyOutcome::Wrong(p) => {
            return Ok(Resolution::WrongResolution {
                strategy: StrategyName::Symbol,
                resolved_to: p,
            });
        }
    }

    match content_result {
        StrategyOutcome::Resolved(p) => resolved.push((StrategyName::ContentDigest, p)),
        StrategyOutcome::Failed(r) => failed.push(StrategyFailure {
            strategy: StrategyName::ContentDigest,
            reason: r,
        }),
        StrategyOutcome::Wrong(p) => {
            return Ok(Resolution::WrongResolution {
                strategy: StrategyName::ContentDigest,
                resolved_to: p,
            });
        }
    }

    // All failed?
    if resolved.is_empty() {
        return Ok(Resolution::Failed { reasons: failed });
    }

    // Check agreement: do all resolved strategies point to the same path?
    let paths: Vec<&str> = resolved.iter().map(|(_, p)| p.as_str()).collect();
    let all_agree = paths.windows(2).all(|w| w[0] == w[1]);

    if all_agree {
        let agreed_strategies: Vec<StrategyName> = resolved.iter().map(|(s, _)| *s).collect();
        let failed_strategies: Vec<StrategyName> = failed.iter().map(|f| f.strategy).collect();
        Ok(Resolution::Correct {
            path: resolved[0].1.clone(),
            strategies: agreed_strategies,
            failed: failed_strategies,
        })
    } else {
        let disagreements: Vec<StrategyDisagreement> = resolved
            .iter()
            .map(|(s, p)| StrategyDisagreement {
                strategy: *s,
                resolved_to: p.clone(),
            })
            .collect();
        Ok(Resolution::Degraded { disagreements })
    }
}

/// Outcome of a single strategy.
enum StrategyOutcome {
    /// Resolved to a path.
    Resolved(String),
    /// Failed to resolve.
    Failed(String),
    /// Resolved to wrong code (content mismatch).
    #[allow(dead_code)]
    Wrong(String),
}

// ---------------------------------------------------------------------------
// Path-range strategy: navigate tree, read blob, check line range
// ---------------------------------------------------------------------------

/// Path-range strategy: navigate to the path in the tree, read the
/// blob, verify the line range `[start, end]` is within the blob.
fn resolve_path_range(anchor: &Anchor, store: &dyn StoreBackend) -> StrategyOutcome {
    let Some(path) = &anchor.path else {
        return StrategyOutcome::Failed("anchor has no path".into());
    };
    if path.is_empty() {
        return StrategyOutcome::Failed("path is empty".into());
    }

    let Ok(tree_oid) = Oid::new(&anchor.tree) else {
        return StrategyOutcome::Failed(format!("invalid tree OID: {}", anchor.tree));
    };

    // Navigate the tree to find the blob at this path.
    let Some(blob_oid) = navigate_to_blob(store, &tree_oid, path) else {
        return StrategyOutcome::Failed(format!("path not found in tree: {path}"));
    };

    // Read the blob.
    let Ok(blob_data) = store.read_blob(&blob_oid) else {
        return StrategyOutcome::Failed(format!("failed to read blob: {blob_oid}"));
    };

    // Check the line range if present.
    if let Some([start, end]) = anchor.range {
        if start == 0 {
            return StrategyOutcome::Failed("line range start is 0 (1-indexed)".into());
        }
        if start > end {
            return StrategyOutcome::Failed(format!("line range {start}-{end}: start > end"));
        }
        let line_count = count_lines(&blob_data);
        if i64::try_from(line_count).unwrap_or(0) < end {
            return StrategyOutcome::Failed(format!(
                "line range {start}-{end} exceeds blob ({line_count} lines)"
            ));
        }
    }

    StrategyOutcome::Resolved(path.clone())
}

/// Navigate from a root tree to a blob by path (e.g., "src/main.rs").
/// Returns the blob's OID if found, `None` if the path doesn't exist
/// or a non-final component isn't a tree.
fn navigate_to_blob(store: &dyn StoreBackend, root: &Oid, path: &str) -> Option<Oid> {
    let components: Vec<&str> = path.split('/').filter(|c| !c.is_empty()).collect();
    if components.is_empty() {
        return None;
    }

    let mut current_tree_oid = root.clone();
    for (i, component) in components.iter().enumerate() {
        let tree = store.read_tree(&current_tree_oid).ok()?;
        let entry = tree.entries.iter().find(|e| e.name == *component)?;
        if i == components.len() - 1 {
            if entry.kind == TreeEntryKind::Blob {
                return Some(entry.oid.clone());
            }
            return None;
        }
        if entry.kind != TreeEntryKind::Tree {
            return None;
        }
        current_tree_oid = entry.oid.clone();
    }

    None
}

/// Count lines in a byte array. A trailing newline does not add an
/// extra line: `"a\nb\n"` has 2 lines, `"a\nb"` also has 2.
#[allow(clippy::naive_bytecount)]
fn count_lines(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }
    let n_count = data.iter().filter(|&&b| b == b'\n').count();
    if data.last() == Some(&b'\n') {
        n_count
    } else {
        n_count + 1
    }
}

// ---------------------------------------------------------------------------
// Symbol strategy: traverse blobs, search for definition pattern
// ---------------------------------------------------------------------------

/// Symbol strategy: traverse all blobs in the tree, search for a
/// definition pattern (`fn name`, `def name`, `struct name`, etc.).
/// Returns the path of the first blob containing the definition.
fn resolve_symbol(anchor: &Anchor, store: &dyn StoreBackend) -> StrategyOutcome {
    let Some(symbol) = &anchor.symbol else {
        return StrategyOutcome::Failed("anchor has no symbol".into());
    };
    if symbol.is_empty() {
        return StrategyOutcome::Failed("symbol is empty".into());
    }

    let Ok(tree_oid) = Oid::new(&anchor.tree) else {
        return StrategyOutcome::Failed(format!("invalid tree OID: {}", anchor.tree));
    };

    match find_symbol_in_tree(store, &tree_oid, symbol) {
        Some(path) => StrategyOutcome::Resolved(path),
        None => StrategyOutcome::Failed(format!("symbol not found: {symbol}")),
    }
}

/// Traverse a tree recursively, searching each blob for a symbol
/// definition. Returns the full path to the first blob containing it.
fn find_symbol_in_tree(store: &dyn StoreBackend, root: &Oid, symbol: &str) -> Option<String> {
    let tree = store.read_tree(root).ok()?;
    for entry in &tree.entries {
        match entry.kind {
            TreeEntryKind::Blob => {
                if let Ok(data) = store.read_blob(&entry.oid) {
                    if blob_contains_symbol(&data, symbol) {
                        return Some(entry.name.clone());
                    }
                }
            }
            TreeEntryKind::Tree => {
                if let Some(found) = find_symbol_in_tree(store, &entry.oid, symbol) {
                    return Some(format!("{}/{found}", entry.name));
                }
            }
        }
    }
    None
}

/// Check if a blob contains a definition of `symbol`. Uses a simple
/// heuristic: looks for common definition patterns at the start of
/// a trimmed line, followed by a word boundary.
fn blob_contains_symbol(data: &[u8], symbol: &str) -> bool {
    let text = std::str::from_utf8(data).unwrap_or("");
    let prefixes = [
        "fn ",
        "pub fn ",
        "def ",
        "function ",
        "struct ",
        "pub struct ",
        "class ",
        "enum ",
        "pub enum ",
        "const ",
        "static ",
        "pub const ",
        "pub static ",
        "type ",
        "pub type ",
        "impl ",
        "trait ",
        "pub trait ",
        "module ",
        "func ",
        "let ",
        "var ",
    ];
    for line in text.lines() {
        let trimmed = line.trim_start();
        for prefix in &prefixes {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                if let Some(after) = rest.strip_prefix(symbol) {
                    if after.is_empty()
                        || !after.starts_with(|c: char| c.is_alphanumeric() || c == '_')
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Content-digest strategy: traverse blobs, compare SHA-256
// ---------------------------------------------------------------------------

/// Content-digest strategy: check if the digest is a blob OID that
/// exists, or traverse all blobs comparing SHA-256. Returns the path
/// of the first matching blob.
fn resolve_content_digest(anchor: &Anchor, store: &dyn StoreBackend) -> StrategyOutcome {
    let Some(digest) = &anchor.content_digest else {
        return StrategyOutcome::Failed("anchor has no content digest".into());
    };
    if digest.is_empty() {
        return StrategyOutcome::Failed("content digest is empty".into());
    }

    // Fast path: if the digest IS a valid blob OID that exists, the
    // content is in the store. We still need to find its path in the
    // tree for agreement checking.
    if let Ok(oid) = Oid::new(digest) {
        if store.has_blob(&oid) {
            let Ok(tree_oid) = Oid::new(&anchor.tree) else {
                return StrategyOutcome::Failed(format!("invalid tree OID: {}", anchor.tree));
            };
            if let Some(path) = find_blob_oid_in_tree(store, &tree_oid, &oid) {
                return StrategyOutcome::Resolved(path);
            }
            // Blob exists but not in this tree — degraded territory.
            return StrategyOutcome::Failed("blob exists but not in tree".into());
        }
    }

    // Slow path: traverse all blobs, compute SHA-256, compare.
    let Ok(tree_oid) = Oid::new(&anchor.tree) else {
        return StrategyOutcome::Failed(format!("invalid tree OID: {}", anchor.tree));
    };
    match find_content_in_tree(store, &tree_oid, digest) {
        Some(path) => StrategyOutcome::Resolved(path),
        None => StrategyOutcome::Failed(format!("content digest not found: {digest}")),
    }
}

/// Traverse a tree recursively, finding a blob whose SHA-256 matches
/// `digest`. Returns the full path to the first match.
fn find_content_in_tree(store: &dyn StoreBackend, root: &Oid, digest: &str) -> Option<String> {
    let tree = store.read_tree(root).ok()?;
    for entry in &tree.entries {
        match entry.kind {
            TreeEntryKind::Blob => {
                if let Ok(data) = store.read_blob(&entry.oid) {
                    let blob_digest = Oid::from_blob_data(&data);
                    if blob_digest.as_str() == digest {
                        return Some(entry.name.clone());
                    }
                }
            }
            TreeEntryKind::Tree => {
                if let Some(found) = find_content_in_tree(store, &entry.oid, digest) {
                    return Some(format!("{}/{found}", entry.name));
                }
            }
        }
    }
    None
}

/// Traverse a tree recursively, finding a blob whose OID matches.
/// Returns the full path to the first match.
fn find_blob_oid_in_tree(store: &dyn StoreBackend, root: &Oid, target: &Oid) -> Option<String> {
    let tree = store.read_tree(root).ok()?;
    for entry in &tree.entries {
        match entry.kind {
            TreeEntryKind::Blob => {
                if &entry.oid == target {
                    return Some(entry.name.clone());
                }
            }
            TreeEntryKind::Tree => {
                if let Some(found) = find_blob_oid_in_tree(store, &entry.oid, target) {
                    return Some(format!("{}/{found}", entry.name));
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use elench_claim::{AnchorStrategy, ClaimId, ClaimKind};
    use elench_store::{MemoryStore, StoreBackend, TreeEntry, TreeEntryKind};

    fn make_anchor(
        tree: &str,
        path: Option<&str>,
        symbol: Option<&str>,
        content_digest: Option<&str>,
    ) -> Anchor {
        Anchor {
            tree: tree.into(),
            strategy: AnchorStrategy::Multi,
            path: path.map(String::from),
            range: Some([1, 10]),
            symbol: symbol.map(String::from),
            content_digest: content_digest.map(String::from),
        }
    }

    fn make_anchor_no_range(
        tree: &str,
        path: Option<&str>,
        symbol: Option<&str>,
        content_digest: Option<&str>,
    ) -> Anchor {
        Anchor {
            tree: tree.into(),
            strategy: AnchorStrategy::Multi,
            path: path.map(String::from),
            range: None,
            symbol: symbol.map(String::from),
            content_digest: content_digest.map(String::from),
        }
    }

    fn make_claim_with_anchor(id: &str, tree: &str, anchor: Anchor) -> elench_claim::Claim {
        let mut a = anchor;
        a.tree = tree.into();
        elench_claim::Claim {
            id: ClaimId::new(id).unwrap(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: elench_claim::AssertionForm::Annotation {
                text: "test".into(),
            },
            origin: elench_claim::Origin {
                kind: elench_claim::OriginKind::AgentAsserted,
                producer: elench_claim::Producer {
                    id: "test-producer".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: a,
            timestamp: 1_700_000_000,
            evidence: vec![],
            depends_on: vec![],
        }
    }

    // Build a store with a tree containing "src/lib.rs" (10 lines) and
    // "src/parser.rs" (with "fn parse_input"). Returns the tree OID.
    fn build_simple_tree(store: &mut MemoryStore) -> Oid {
        let lib_content =
            "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\n";
        let parser_content = "fn parse_input() {\n    // do stuff\n}\n";

        let lib_blob = store.store_blob(lib_content.as_bytes()).unwrap();
        let parser_blob = store.store_blob(parser_content.as_bytes()).unwrap();

        // Inner tree: src/
        let src_tree = store
            .store_tree(vec![
                TreeEntry {
                    name: "lib.rs".into(),
                    mode: 0o100_644,
                    oid: lib_blob,
                    kind: TreeEntryKind::Blob,
                },
                TreeEntry {
                    name: "parser.rs".into(),
                    mode: 0o100_644,
                    oid: parser_blob,
                    kind: TreeEntryKind::Blob,
                },
            ])
            .unwrap();

        // Root tree: src/ (tree), README.md (blob)
        let readme_blob = store.store_blob(b"# project").unwrap();
        store
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
                    oid: src_tree,
                    kind: TreeEntryKind::Tree,
                },
            ])
            .unwrap()
    }

    // --- Path-range strategy tests ---

    #[test]
    fn scenario_resolve_path_range_finds_blob_in_tree() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let anchor = make_anchor(tree_oid.as_str(), Some("src/lib.rs"), None, None);
        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Correct {
                path,
                strategies,
                failed,
            } => {
                assert_eq!(path, "src/lib.rs");
                assert!(strategies.contains(&StrategyName::PathRange));
                assert!(failed.contains(&StrategyName::Symbol));
                assert!(failed.contains(&StrategyName::ContentDigest));
            }
            _ => panic!("expected Correct, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_path_range_fails_when_path_not_in_tree() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let anchor = make_anchor(tree_oid.as_str(), Some("nonexistent.rs"), None, None);
        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Failed { reasons } => {
                let pr = reasons
                    .iter()
                    .find(|r| r.strategy == StrategyName::PathRange);
                assert!(pr.is_some(), "path-range should fail");
                assert!(pr.unwrap().reason.contains("not found"));
            }
            _ => panic!("expected Failed, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_path_range_fails_when_range_exceeds_blob() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let anchor = Anchor {
            tree: tree_oid.as_str().into(),
            strategy: AnchorStrategy::Multi,
            path: Some("src/lib.rs".into()),
            range: Some([1, 100]),
            symbol: None,
            content_digest: None,
        };
        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Failed { reasons } => {
                let pr = reasons
                    .iter()
                    .find(|r| r.strategy == StrategyName::PathRange);
                assert!(pr.is_some());
                assert!(pr.unwrap().reason.contains("exceeds blob"));
            }
            _ => panic!("expected Failed, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_path_range_no_range_succeeds() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let anchor = make_anchor_no_range(tree_oid.as_str(), Some("src/lib.rs"), None, None);
        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Correct { path, .. } => {
                assert_eq!(path, "src/lib.rs");
            }
            _ => panic!("expected Correct, got {result:?}"),
        }
    }

    // --- Symbol strategy tests ---

    #[test]
    fn scenario_resolve_symbol_finds_definition_in_tree() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let anchor = make_anchor(tree_oid.as_str(), None, Some("parse_input"), None);
        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Correct {
                path,
                strategies,
                failed,
            } => {
                assert_eq!(path, "src/parser.rs");
                assert!(strategies.contains(&StrategyName::Symbol));
                assert!(failed.contains(&StrategyName::PathRange));
                assert!(failed.contains(&StrategyName::ContentDigest));
            }
            _ => panic!("expected Correct, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_symbol_fails_when_not_found() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let anchor = make_anchor(tree_oid.as_str(), None, Some("nonexistent_fn"), None);
        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Failed { reasons } => {
                let sym = reasons.iter().find(|r| r.strategy == StrategyName::Symbol);
                assert!(sym.is_some());
                assert!(sym.unwrap().reason.contains("not found"));
            }
            _ => panic!("expected Failed, got {result:?}"),
        }
    }

    // --- Content-digest strategy tests ---

    #[test]
    fn scenario_resolve_content_digest_finds_blob_by_sha256() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let lib_content =
            "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\n";
        let digest = Oid::from_blob_data(lib_content.as_bytes());
        let anchor = make_anchor(tree_oid.as_str(), None, None, Some(digest.as_str()));
        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Correct {
                path,
                strategies,
                failed,
            } => {
                assert_eq!(path, "src/lib.rs");
                assert!(strategies.contains(&StrategyName::ContentDigest));
                assert!(failed.contains(&StrategyName::PathRange));
                assert!(failed.contains(&StrategyName::Symbol));
            }
            _ => panic!("expected Correct, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_content_digest_fails_when_no_match() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let wrong_digest = "a".repeat(64);
        let anchor = make_anchor(tree_oid.as_str(), None, None, Some(&wrong_digest));
        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Failed { reasons } => {
                let cd = reasons
                    .iter()
                    .find(|r| r.strategy == StrategyName::ContentDigest);
                assert!(cd.is_some());
                assert!(cd.unwrap().reason.contains("not found"));
            }
            _ => panic!("expected Failed, got {result:?}"),
        }
    }

    // --- Multi-strategy agreement tests ---

    #[test]
    fn scenario_resolve_multi_all_three_agree() {
        let mut store = MemoryStore::new();
        let _tree_oid = build_simple_tree(&mut store);
        let lib_content =
            "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\n";
        let _digest = Oid::from_blob_data(lib_content.as_bytes());

        // All three point to src/lib.rs (path = "src/lib.rs",
        // symbol = "lib" -- we'd need "fn lib" in the content for
        // this to work; instead let's use a content that has a symbol
        // matching the path).
        //
        // Actually, for a true all-three-agree test, we need a blob
        // at "src/lib.rs" that ALSO contains a definition of "lib" and
        // whose SHA-256 matches the content_digest. Let's construct
        // that.
        let content = "fn lib() {\n    // do stuff\n}\n";
        let blob = store.store_blob(content.as_bytes()).unwrap();
        let digest = Oid::from_blob_data(content.as_bytes());

        let inner_tree = store
            .store_tree(vec![TreeEntry {
                name: "lib.rs".into(),
                mode: 0o100_644,
                oid: blob,
                kind: TreeEntryKind::Blob,
            }])
            .unwrap();
        let root = store
            .store_tree(vec![TreeEntry {
                name: "src".into(),
                mode: 0o040_000,
                oid: inner_tree,
                kind: TreeEntryKind::Tree,
            }])
            .unwrap();

        let anchor = Anchor {
            tree: root.as_str().into(),
            strategy: AnchorStrategy::Multi,
            path: Some("src/lib.rs".into()),
            range: Some([1, 3]),
            symbol: Some("lib".into()),
            content_digest: Some(digest.as_str().to_string()),
        };

        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Correct {
                path,
                strategies,
                failed,
            } => {
                assert_eq!(path, "src/lib.rs");
                assert_eq!(strategies.len(), 3);
                assert!(failed.is_empty());
            }
            _ => panic!("expected Correct, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_multi_two_agree_one_failed() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let lib_content =
            "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\n";
        let digest = Oid::from_blob_data(lib_content.as_bytes());

        // path-range and content-digest both point to src/lib.rs;
        // symbol is missing.
        let anchor = Anchor {
            tree: tree_oid.as_str().into(),
            strategy: AnchorStrategy::Multi,
            path: Some("src/lib.rs".into()),
            range: Some([1, 10]),
            symbol: None,
            content_digest: Some(digest.as_str().to_string()),
        };

        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Correct {
                path,
                strategies,
                failed,
            } => {
                assert_eq!(path, "src/lib.rs");
                assert_eq!(strategies.len(), 2);
                assert!(strategies.contains(&StrategyName::PathRange));
                assert!(strategies.contains(&StrategyName::ContentDigest));
                assert_eq!(failed.len(), 1);
                assert_eq!(failed[0], StrategyName::Symbol);
            }
            _ => panic!("expected Correct, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_multi_disagreement_degraded() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);

        // path = src/lib.rs, symbol = parse_input (in src/parser.rs).
        // They disagree → Degraded.
        let anchor = Anchor {
            tree: tree_oid.as_str().into(),
            strategy: AnchorStrategy::Multi,
            path: Some("src/lib.rs".into()),
            range: Some([1, 10]),
            symbol: Some("parse_input".into()),
            content_digest: None,
        };

        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Degraded { disagreements } => {
                assert_eq!(disagreements.len(), 2);
                let paths: Vec<&str> = disagreements
                    .iter()
                    .map(|d| d.resolved_to.as_str())
                    .collect();
                assert!(paths.contains(&"src/lib.rs"));
                assert!(paths.contains(&"src/parser.rs"));
            }
            _ => panic!("expected Degraded, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_not_multi_rejected() {
        let store = MemoryStore::new();
        let anchor = Anchor {
            tree: "abc123".into(),
            strategy: AnchorStrategy::PathRange,
            path: Some("src/main.rs".into()),
            range: Some([1, 10]),
            symbol: None,
            content_digest: None,
        };
        let result = resolve(&anchor, &store);
        assert_eq!(
            result,
            Err(AnchorError::NotMulti(AnchorStrategy::PathRange))
        );
    }

    #[test]
    fn scenario_resolve_empty_path_treated_as_failed() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let anchor = make_anchor(tree_oid.as_str(), Some(""), None, None);
        let result = resolve(&anchor, &store).unwrap();
        match result {
            Resolution::Failed { reasons } => {
                let pr = reasons
                    .iter()
                    .find(|r| r.strategy == StrategyName::PathRange);
                assert!(pr.is_some());
                assert!(pr.unwrap().reason.contains("empty"));
            }
            _ => panic!("expected Failed, got {result:?}"),
        }
    }

    // --- Reconciliation tests ---

    #[test]
    fn scenario_reconcile_all_intact() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let anchor = make_anchor(tree_oid.as_str(), Some("src/lib.rs"), None, None);
        let claim = make_claim_with_anchor(
            "cl_0000000000000000000000000000000000000000000000000000000000000070",
            tree_oid.as_str(),
            anchor,
        );
        let log = vec![claim];
        let report = reconcile(tree_oid.as_str(), &log, &store).unwrap();
        assert_eq!(report.intact_count(), 1);
        assert_eq!(report.drift_count(), 0);
        assert!(!report.has_drift());
    }

    #[test]
    fn scenario_reconcile_all_drifted() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        // No path, no symbol, no content-digest → all strategies fail.
        let anchor = make_anchor(tree_oid.as_str(), None, None, None);
        let claim = make_claim_with_anchor(
            "cl_0000000000000000000000000000000000000000000000000000000000000071",
            tree_oid.as_str(),
            anchor,
        );
        let log = vec![claim];
        let report = reconcile(tree_oid.as_str(), &log, &store).unwrap();
        assert_eq!(report.intact_count(), 0);
        assert_eq!(report.drift_count(), 1);
        assert!(report.has_drift());
        match &report.drifted[0].resolution {
            Resolution::Failed { reasons } => {
                assert_eq!(reasons.len(), 3);
            }
            _ => panic!("expected Failed"),
        }
    }

    #[test]
    fn scenario_reconcile_mixed() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);

        let intact_anchor = make_anchor(tree_oid.as_str(), Some("src/lib.rs"), None, None);
        let drifted_anchor = make_anchor(tree_oid.as_str(), None, None, None);

        let claim_a = make_claim_with_anchor(
            "cl_0000000000000000000000000000000000000000000000000000000000000072",
            tree_oid.as_str(),
            intact_anchor,
        );
        let claim_b = make_claim_with_anchor(
            "cl_0000000000000000000000000000000000000000000000000000000000000073",
            tree_oid.as_str(),
            drifted_anchor,
        );
        let log = vec![claim_a, claim_b];
        let report = reconcile(tree_oid.as_str(), &log, &store).unwrap();
        assert_eq!(report.intact_count(), 1);
        assert_eq!(report.drift_count(), 1);
    }

    #[test]
    fn scenario_reconcile_other_tree_not_affected() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);
        let anchor = make_anchor(tree_oid.as_str(), Some("src/lib.rs"), None, None);
        let claim = make_claim_with_anchor(
            "cl_0000000000000000000000000000000000000000000000000000000000000074",
            "other_tree_oid_that_does_not_match",
            anchor,
        );
        let log = vec![claim];
        let report = reconcile(tree_oid.as_str(), &log, &store).unwrap();
        // "other_tree_..." is not a valid OID — resolve would error on it.
        // But the claim is filtered by tree == tree_oid, so it's not
        // included at all. No claims to reconcile.
        assert_eq!(report.intact_count(), 0);
        assert_eq!(report.drift_count(), 0);
        assert!(!report.has_drift());
    }

    #[test]
    fn scenario_reconcile_degraded_reported() {
        let mut store = MemoryStore::new();
        let tree_oid = build_simple_tree(&mut store);

        let anchor = Anchor {
            tree: tree_oid.as_str().into(),
            strategy: AnchorStrategy::Multi,
            path: Some("src/lib.rs".into()),
            range: Some([1, 10]),
            symbol: Some("parse_input".into()),
            content_digest: None,
        };
        let claim = make_claim_with_anchor(
            "cl_0000000000000000000000000000000000000000000000000000000000000075",
            tree_oid.as_str(),
            anchor,
        );
        let log = vec![claim];
        let report = reconcile(tree_oid.as_str(), &log, &store).unwrap();
        assert_eq!(report.drift_count(), 1);
        match &report.drifted[0].resolution {
            Resolution::Degraded { disagreements } => {
                assert_eq!(disagreements.len(), 2);
            }
            _ => panic!("expected Degraded"),
        }
    }

    // --- blob_contains_symbol unit tests ---

    #[test]
    fn scenario_blob_contains_rust_fn() {
        assert!(blob_contains_symbol(b"fn parse_input() {}", "parse_input"));
        assert!(blob_contains_symbol(
            b"pub fn parse_input() {}",
            "parse_input"
        ));
    }

    #[test]
    fn scenario_blob_contains_python_def() {
        assert!(blob_contains_symbol(b"def parse_input():", "parse_input"));
    }

    #[test]
    fn scenario_blob_contains_struct() {
        assert!(blob_contains_symbol(b"struct Foo { x: i32 }", "Foo"));
        assert!(blob_contains_symbol(b"pub struct Foo<T> {", "Foo"));
    }

    #[test]
    fn scenario_blob_does_not_match_prefix() {
        // "fn parse" should NOT match "parse_input" (prefix-only).
        assert!(!blob_contains_symbol(b"fn parse() {}", "parse_input"));
        // "fn parse_input_extra" should NOT match "parse_input".
        assert!(!blob_contains_symbol(
            b"fn parse_input_extra() {}",
            "parse_input"
        ));
    }

    // --- count_lines unit tests ---

    #[test]
    fn scenario_count_lines_basic() {
        assert_eq!(count_lines(b"one\ntwo\nthree"), 3);
        assert_eq!(count_lines(b"one\ntwo\nthree\n"), 3);
        assert_eq!(count_lines(b""), 0);
        assert_eq!(count_lines(b"single"), 1);
    }
}
