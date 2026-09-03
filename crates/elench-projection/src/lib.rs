//! # elench-projection
//!
//! Git projection — deterministic synthesis of git-compatible objects
//! from the claim log (ADR-0002, ADR-0007).
//!
//! The projection is read-only. `git log`, `git blame`, `git checkout`
//! work because elench produces git objects on demand. Writes go
//! through elench, never git.
//!
//! ## Determinism (BC4, INV-20)
//!
//! Given the same claim log, any party produces byte-identical git
//! objects:
//!
//! - **Tree OIDs**: passthrough. elench tree OIDs are identical to
//!   git SHA-256 tree OIDs (same canonical serialization).
//! - **Commit OIDs**: computed from (tree OID, parent commit OIDs,
//!   author, committer, message, timestamps) — all derived from the
//!   claim log, not the machine.
//! - **Author/committer**: derived from `claim.origin.producer.id`.
//!   No user-configurable `user.name` / `user.email`.
//! - **Timestamps**: from `claim.timestamp` (Unix epoch), not wall clock.
//!
//! ## Commit granularity (ADR-0007)
//!
//! One commit per tree-changing claim. A session that produces N tree
//! changes gets N commits. This preserves the blast-radius connection:
//! `git blame` maps to the specific claim that introduced the line.

use elench_claim::{Claim, ClaimKind};
use elench_store::{Oid, StoreBackend, TreeEntry, TreeEntryKind};
use sha2::{Digest, Sha256};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Git object types
// ---------------------------------------------------------------------------

/// A git blob object: content-addressed byte array.
/// The OID is the SHA-256 of `blob <len>\0<data>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitBlob {
    pub oid: String,
    pub data: Vec<u8>,
}

/// A git tree object: sorted entries.
/// The OID is the SHA-256 of `tree <len>\0<entries>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitTree {
    pub oid: String,
    pub entries: Vec<GitTreeEntry>,
}

/// A git tree entry: mode, name, OID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitTreeEntry {
    pub mode: String,
    pub name: String,
    pub oid: String,
}

/// A git commit object.
/// The OID is the SHA-256 of `commit <len>\0<content>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommit {
    pub oid: String,
    pub tree: String,
    pub parents: Vec<String>,
    pub author: GitAuthor,
    pub committer: GitAuthor,
    pub message: String,
}

/// Git author/committer identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitAuthor {
    pub name: String,
    pub email: String,
    pub timestamp: i64,
    pub timezone_offset: i64,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProjectionError {
    #[error("claim log is empty — nothing to project")]
    EmptyLog,

    #[error("claim {0} has no anchor — cannot determine tree")]
    NoAnchor(String),

    #[error("store error: {0}")]
    Store(String),
}

// ---------------------------------------------------------------------------
// Synthesis: claims -> git objects
// ---------------------------------------------------------------------------

/// The result of synthesizing git objects from a claim log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    /// Git commits, in chronological order (oldest first).
    pub commits: Vec<GitCommit>,
    /// All blobs, indexed by OID.
    pub blobs: Vec<GitBlob>,
    /// All trees, indexed by OID.
    pub trees: Vec<GitTree>,
}

/// Synthesize git objects from a claim log and store.
///
/// ADR-0002: the projection is deterministic. Given the same claim
/// log and store, any party produces byte-identical git objects.
///
/// ADR-0007: one commit per tree-changing claim. Tree-changing
/// claims are identified by having a different tree OID than their
/// predecessor. Author, committer, and timestamps are all derived
/// from the claim, not the machine.
///
/// # Errors
///
/// Returns [`ProjectionError`] if the claim log is empty or a claim
/// has no anchor.
#[allow(clippy::format_push_string, clippy::too_many_lines)]
pub fn synthesize(log: &[Claim], store: &impl StoreBackend) -> Result<Projection, ProjectionError> {
    if log.is_empty() {
        return Err(ProjectionError::EmptyLog);
    }

    let mut commits = Vec::new();
    let mut blobs = Vec::new();
    let mut trees = Vec::new();
    let mut seen_blobs = std::collections::HashSet::new();
    let mut seen_trees = std::collections::HashSet::new();

    // Sort claims by timestamp (deterministic ordering).
    let mut sorted: Vec<&Claim> = log.iter().collect();
    sorted.sort_by(|a, b| {
        a.timestamp
            .cmp(&b.timestamp)
            .then(a.id.as_str().cmp(b.id.as_str()))
    });

    let mut prev_commit_oid: Option<String> = None;
    let mut prev_tree_oid: Option<String> = None;

    for claim in &sorted {
        // Only assertions produce tree changes. Falsifications,
        // verifications, supersessions, and residue-acceptances are
        // metadata — they don't change the tree.
        if claim.kind != ClaimKind::Assertion {
            continue;
        }

        let tree_oid = &claim.anchor.tree;
        if tree_oid.is_empty() {
            return Err(ProjectionError::NoAnchor(claim.id.as_str().to_string()));
        }

        // Skip if the tree hasn't changed (same as previous).
        if prev_tree_oid.as_ref() == Some(tree_oid) {
            continue;
        }

        // Materialize the tree from the store.
        // For Phase 4, if the tree is not in the store (in-memory only),
        // we synthesize a minimal tree with the anchor path.
        let tree_entries = if let Ok(tree) =
            store.read_tree(&Oid::new(tree_oid).map_err(|e| ProjectionError::Store(e.to_string()))?)
        {
            tree.entries
        } else {
            // Tree not in store — synthesize minimal entry from anchor path.
            vec![TreeEntry {
                name: claim
                    .anchor
                    .path
                    .clone()
                    .unwrap_or_else(|| "unknown".into()),
                mode: 0o100_644,
                oid: Oid::from_blob_data(b""),
                kind: TreeEntryKind::Blob,
            }]
        };

        // Convert elench tree entries to git tree entries.
        let git_entries: Vec<GitTreeEntry> = tree_entries
            .iter()
            .map(|e| {
                tree_entry_to_git(
                    e,
                    &mut blobs,
                    &mut trees,
                    &mut seen_blobs,
                    &mut seen_trees,
                    store,
                )
            })
            .collect();

        // Compute git tree OID (same as elench tree OID — passthrough).
        let git_tree = GitTree {
            oid: tree_oid.clone(),
            entries: git_entries,
        };
        if !seen_trees.contains(&git_tree.oid) {
            trees.push(git_tree.clone());
            seen_trees.insert(git_tree.oid.clone());
        }

        // Derive author from claim producer.
        let author = GitAuthor {
            name: claim.origin.producer.id.clone(),
            email: format!("{}@elench.dev", claim.origin.producer.id),
            timestamp: claim.timestamp,
            timezone_offset: 0,
        };

        // Build commit content.
        let mut content = String::new();
        content.push_str(&format!("tree {}\n", git_tree.oid));
        if let Some(ref parent) = prev_commit_oid {
            content.push_str(&format!("parent {parent}\n"));
        }
        content.push_str(&format!(
            "author {} <{}> {} +0000\n",
            author.name, author.email, author.timestamp
        ));
        content.push_str(&format!(
            "committer {} <{}> {} +0000\n",
            author.name, author.email, author.timestamp
        ));
        content.push('\n');
        content.push_str(&format!("elench claim: {}\n", claim.id));

        // Compute git commit OID.
        let commit_header = format!("commit {}\0", content.len());
        let mut hasher = Sha256::new();
        hasher.update(commit_header.as_bytes());
        hasher.update(content.as_bytes());
        let commit_oid = format!("{:x}", hasher.finalize());

        let commit = GitCommit {
            oid: commit_oid.clone(),
            tree: git_tree.oid,
            parents: prev_commit_oid.iter().cloned().collect(),
            author: author.clone(),
            committer: author,
            message: format!("elench claim: {}", claim.id),
        };

        commits.push(commit);
        prev_commit_oid = Some(commit_oid);
        prev_tree_oid = Some(tree_oid.clone());
    }

    Ok(Projection {
        commits,
        blobs,
        trees,
    })
}

/// Convert an elench `TreeEntry` to a `GitTreeEntry`, materializing
/// child blobs and trees as needed.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
fn tree_entry_to_git(
    entry: &TreeEntry,
    blobs: &mut Vec<GitBlob>,
    trees: &mut Vec<GitTree>,
    seen_blobs: &mut std::collections::HashSet<String>,
    seen_trees: &mut std::collections::HashSet<String>,
    _store: &impl StoreBackend,
) -> GitTreeEntry {
    let mode = match entry.kind {
        TreeEntryKind::Blob => {
            if entry.mode == 0o100_755 {
                "100755".to_string()
            } else if entry.mode == 0o120_000 {
                "120000".to_string()
            } else {
                "100644".to_string()
            }
        }
        TreeEntryKind::Tree => "040000".to_string(),
    };

    // Materialize blob if not seen.
    if entry.kind == TreeEntryKind::Blob && !seen_blobs.contains(entry.oid.as_str()) {
        blobs.push(GitBlob {
            oid: entry.oid.as_str().to_string(),
            data: Vec::new(),
        });
        seen_blobs.insert(entry.oid.as_str().to_string());
    }

    // Materialize tree if not seen.
    if entry.kind == TreeEntryKind::Tree && !seen_trees.contains(entry.oid.as_str()) {
        trees.push(GitTree {
            oid: entry.oid.as_str().to_string(),
            entries: Vec::new(),
        });
        seen_trees.insert(entry.oid.as_str().to_string());
    }

    GitTreeEntry {
        mode,
        name: entry.name.clone(),
        oid: entry.oid.as_str().to_string(),
    }
}

/// Generate a `git log`-like output from a projection.
///
/// This is NOT a real git repository — it's a deterministic text
/// representation that matches what `git log --oneline` would show.
/// A full implementation would write to `.git/` or use a FUSE
/// filesystem; for Phase 4, the text representation is sufficient
/// to verify determinism (BC4).
#[must_use]
pub fn git_log_oneline(projection: &Projection) -> String {
    let mut lines = Vec::new();
    for commit in projection.commits.iter().rev() {
        lines.push(format!("{} {}", &commit.oid[..12], commit.message.trim()));
    }
    lines.join("\n")
}

/// Generate a `git log`-like full output from a projection.
#[must_use]
pub fn git_log_full(projection: &Projection) -> String {
    let mut lines = Vec::new();
    for commit in projection.commits.iter().rev() {
        lines.push(format!("commit {}", commit.oid));
        if commit.parents.len() == 1 {
            lines.push(format!("parent {}", commit.parents[0]));
        } else if commit.parents.len() > 1 {
            for p in &commit.parents {
                lines.push(format!("parent {p}"));
            }
        }
        lines.push(format!(
            "Author: {} <{}>",
            commit.author.name, commit.author.email
        ));
        lines.push(format!(
            "Date:   {} +0000",
            format_timestamp(commit.author.timestamp)
        ));
        lines.push(String::new());
        lines.push(format!("    {}", commit.message.trim()));
        lines.push(String::new());
    }
    lines.join("\n")
}

/// Format a Unix timestamp as a git-style date.
fn format_timestamp(ts: i64) -> String {
    // Simple formatting — a real implementation would use localtime.
    // For determinism, we always use UTC.
    format!("Thu Jan 1 00:00:00 {ts}")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use elench_claim::{
        Anchor, AnchorStrategy, AssertionForm, ClaimId, ClaimKind, Expression, Origin, OriginKind,
        Producer,
    };

    fn make_tree_changing_claim(id: &str, tree: &str, producer_id: &str, timestamp: i64) -> Claim {
        Claim {
            id: ClaimId::new(id).unwrap(),
            kind: ClaimKind::Assertion,
            target: vec![],
            assertion: AssertionForm::Predicate {
                expression: Expression {
                    language: "elench-predicate-v1".into(),
                    source: "exists(\"Cargo.toml\")".into(),
                    digest: None,
                },
            },
            origin: Origin {
                kind: OriginKind::AgentAsserted,
                producer: Producer {
                    id: producer_id.into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: tree.into(),
                strategy: AnchorStrategy::PathRange,
                path: Some("src/main.rs".into()),
                range: Some([1, 10]),
                symbol: None,
                content_digest: None,
            },
            timestamp,
            evidence: vec![],
            depends_on: vec![],
        }
    }

    const ID_A: &str = "cl_0000000000000000000000000000000000000000000000000000000000000001";
    const ID_B: &str = "cl_0000000000000000000000000000000000000000000000000000000000000002";
    const ID_C: &str = "cl_0000000000000000000000000000000000000000000000000000000000000003";
    const TREE_1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const TREE_2: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    const TREE_3: &str = "3333333333333333333333333333333333333333333333333333333333333333";

    // --- git log shows synthesized commits ---

    #[test]
    fn scenario_git_log_shows_synthesized_commits() {
        let log = vec![
            make_tree_changing_claim(ID_A, TREE_1, "agent-v1", 1_700_000_000),
            make_tree_changing_claim(ID_B, TREE_2, "agent-v1", 1_700_000_001),
        ];
        let store = elench_store::Store::new();
        let projection = synthesize(&log, &store).unwrap();

        assert_eq!(
            projection.commits.len(),
            2,
            "expected 2 commits (one per tree-changing claim)"
        );
        assert_eq!(projection.commits[0].tree, TREE_1);
        assert_eq!(projection.commits[1].tree, TREE_2);
    }

    // --- Author derived from producer.id ---

    #[test]
    fn scenario_author_derived_from_producer() {
        let log = vec![make_tree_changing_claim(
            ID_A,
            TREE_1,
            "claude-opus-5",
            1_700_000_000,
        )];
        let store = elench_store::Store::new();
        let projection = synthesize(&log, &store).unwrap();

        assert_eq!(projection.commits[0].author.name, "claude-opus-5");
        assert_eq!(
            projection.commits[0].author.email,
            "claude-opus-5@elench.dev"
        );
    }

    // --- Timestamps from claim, not wall clock ---

    #[test]
    fn scenario_timestamps_from_claim() {
        let log = vec![make_tree_changing_claim(
            ID_A,
            TREE_1,
            "agent",
            1_234_567_890,
        )];
        let store = elench_store::Store::new();
        let projection = synthesize(&log, &store).unwrap();

        assert_eq!(projection.commits[0].author.timestamp, 1_234_567_890);
        assert_eq!(projection.commits[0].committer.timestamp, 1_234_567_890);
    }

    // --- Determinism: two syntheses produce identical OIDs ---

    #[test]
    fn scenario_deterministic_synthesis_identical_oids() {
        let log = vec![
            make_tree_changing_claim(ID_A, TREE_1, "agent", 1_700_000_000),
            make_tree_changing_claim(ID_B, TREE_2, "agent", 1_700_000_001),
        ];
        let store1 = elench_store::Store::new();
        let store2 = elench_store::Store::new();

        let proj1 = synthesize(&log, &store1).unwrap();
        let proj2 = synthesize(&log, &store2).unwrap();

        // Commit OIDs must be identical
        assert_eq!(proj1.commits.len(), proj2.commits.len());
        for (c1, c2) in proj1.commits.iter().zip(proj2.commits.iter()) {
            assert_eq!(c1.oid, c2.oid, "commit OIDs must be identical");
            assert_eq!(c1.tree, c2.tree, "tree OIDs must be identical");
        }

        // git log output must be identical
        assert_eq!(git_log_oneline(&proj1), git_log_oneline(&proj2));
        assert_eq!(git_log_full(&proj1), git_log_full(&proj2));
    }

    // --- Non-tree-changing claims do not produce commits ---

    #[test]
    fn scenario_non_tree_changing_claims_no_commits() {
        let mut log = vec![make_tree_changing_claim(
            ID_A,
            TREE_1,
            "agent",
            1_700_000_000,
        )];

        // Add a falsification (not a tree-changing claim)
        let falsification = Claim {
            id: ClaimId::new(ID_B).unwrap(),
            kind: ClaimKind::Falsification,
            target: vec![ClaimId::new(ID_A).unwrap()],
            assertion: AssertionForm::Annotation {
                text: "wrong".into(),
            },
            origin: Origin {
                kind: OriginKind::HarnessObserved,
                producer: Producer {
                    id: "harness".into(),
                    session_id: None,
                    hermeticity: None,
                },
            },
            anchor: Anchor {
                tree: TREE_1.into(),
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
        log.push(falsification);

        let store = elench_store::Store::new();
        let projection = synthesize(&log, &store).unwrap();

        // Only 1 commit (the assertion). The falsification is metadata.
        assert_eq!(projection.commits.len(), 1);
    }

    // --- Same tree OID: no new commit ---

    #[test]
    fn scenario_same_tree_no_new_commit() {
        let log = vec![
            make_tree_changing_claim(ID_A, TREE_1, "agent", 1_700_000_000),
            make_tree_changing_claim(ID_B, TREE_1, "agent", 1_700_000_001), // Same tree
        ];
        let store = elench_store::Store::new();
        let projection = synthesize(&log, &store).unwrap();

        // Only 1 commit — second claim has same tree OID
        assert_eq!(projection.commits.len(), 1);
    }

    // --- Linear history: parent commits ---

    #[test]
    fn scenario_linear_history_parent_commits() {
        let log = vec![
            make_tree_changing_claim(ID_A, TREE_1, "agent", 1_700_000_000),
            make_tree_changing_claim(ID_B, TREE_2, "agent", 1_700_000_001),
            make_tree_changing_claim(ID_C, TREE_3, "agent", 1_700_000_002),
        ];
        let store = elench_store::Store::new();
        let projection = synthesize(&log, &store).unwrap();

        assert_eq!(projection.commits.len(), 3);
        // First commit has no parent
        assert!(projection.commits[0].parents.is_empty());
        // Second commit has first as parent
        assert_eq!(
            projection.commits[1].parents,
            vec![projection.commits[0].oid.clone()]
        );
        // Third commit has second as parent
        assert_eq!(
            projection.commits[2].parents,
            vec![projection.commits[1].oid.clone()]
        );
    }

    // --- Empty log: error ---

    #[test]
    fn scenario_empty_log_error() {
        let store = elench_store::Store::new();
        let result = synthesize(&[], &store);
        assert_eq!(result, Err(ProjectionError::EmptyLog));
    }

    // --- git log oneline format ---

    #[test]
    fn scenario_git_log_oneline_format() {
        let log = vec![make_tree_changing_claim(
            ID_A,
            TREE_1,
            "agent",
            1_700_000_000,
        )];
        let store = elench_store::Store::new();
        let projection = synthesize(&log, &store).unwrap();

        let log_output = git_log_oneline(&projection);
        assert!(!log_output.is_empty());
        assert!(log_output.contains("elench claim:"));
    }

    // --- git log full format ---

    #[test]
    fn scenario_git_log_full_format() {
        let log = vec![make_tree_changing_claim(
            ID_A,
            TREE_1,
            "agent",
            1_700_000_000,
        )];
        let store = elench_store::Store::new();
        let projection = synthesize(&log, &store).unwrap();

        let log_output = git_log_full(&projection);
        assert!(log_output.contains("commit "));
        assert!(log_output.contains("Author: agent <agent@elench.dev>"));
        assert!(log_output.contains("elench claim:"));
    }

    // --- Commit OID is deterministic function of claim ---

    #[test]
    fn scenario_commit_oid_changes_with_producer() {
        let log1 = vec![make_tree_changing_claim(
            ID_A,
            TREE_1,
            "agent-1",
            1_700_000_000,
        )];
        let log2 = vec![make_tree_changing_claim(
            ID_A,
            TREE_1,
            "agent-2",
            1_700_000_000,
        )];

        let store = elench_store::Store::new();
        let proj1 = synthesize(&log1, &store).unwrap();
        let proj2 = synthesize(&log2, &store).unwrap();

        // Different producer -> different commit OID
        assert_ne!(proj1.commits[0].oid, proj2.commits[0].oid);
    }

    #[test]
    fn scenario_commit_oid_changes_with_timestamp() {
        let log1 = vec![make_tree_changing_claim(
            ID_A,
            TREE_1,
            "agent",
            1_700_000_000,
        )];
        let log2 = vec![make_tree_changing_claim(
            ID_A,
            TREE_1,
            "agent",
            1_700_000_001,
        )];

        let store = elench_store::Store::new();
        let proj1 = synthesize(&log1, &store).unwrap();
        let proj2 = synthesize(&log2, &store).unwrap();

        // Different timestamp -> different commit OID
        assert_ne!(proj1.commits[0].oid, proj2.commits[0].oid);
    }

    // --- INV-19: projection is read-only ---

    #[test]
    fn scenario_inv19_projection_does_not_write_to_store() {
        let log = vec![
            make_tree_changing_claim(ID_A, TREE_1, "agent", 1_700_000_000),
            make_tree_changing_claim(ID_B, TREE_2, "agent", 1_700_000_001),
        ];
        let store = elench_store::Store::new();

        let blob_before = store.blob_count();
        let tree_before = store.tree_count();
        let claim_before = store.claim_count();

        let _projection = synthesize(&log, &store).unwrap();

        assert_eq!(store.blob_count(), blob_before);
        assert_eq!(store.tree_count(), tree_before);
        assert_eq!(store.claim_count(), claim_before);
    }
    // --- INV-27: projection is lossy (GAP-H1) ---

    #[test]
    fn scenario_inv27_projection_is_lossy() {
        let log = vec![make_tree_changing_claim(
            ID_A,
            TREE_1,
            "agent",
            1_700_000_000,
        )];
        let store = elench_store::Store::new();
        let projection = synthesize(&log, &store).unwrap();
        let log_output = git_log_full(&projection);
        assert!(!log_output.contains("unevaluated"));
        assert!(!log_output.contains("falsified"));
        assert!(!log_output.contains("harness-observed"));
        assert!(!log_output.contains("agent-asserted"));
    }
}
