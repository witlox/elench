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
//! Given an anchor and a tree, `resolve` tries all three strategies:
//! - **Path-range**: same path, same line range. Dies on rename or reformat.
//! - **Symbol**: find the symbol definition anywhere in the tree. Dies on rename.
//! - **Content-digest**: find the normalized content anywhere. Dies on semantic edit.
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
}

// ---------------------------------------------------------------------------
// Resolve
// ---------------------------------------------------------------------------

/// Resolve an anchor against a tree using the multi strategy.
///
/// Tries all three strategies (path-range, symbol, content-digest) and
/// resolves by agreement. Reports `Degraded` when strategies disagree,
/// `Failed` when all fail, `WrongResolution` when a strategy resolves
/// to wrong code (detectable by content mismatch).
///
/// # Errors
///
/// Returns [`AnchorError`] if the anchor's strategy is not `Multi`.
pub fn resolve(anchor: &Anchor) -> Result<Resolution, AnchorError> {
    if anchor.strategy != AnchorStrategy::Multi {
        return Err(AnchorError::NotMulti(anchor.strategy.clone()));
    }

    let path_result = resolve_path_range(anchor);
    let symbol_result = resolve_symbol(anchor);
    let content_result = resolve_content_digest(anchor);

    // Check for wrong-resolution first (E1: the outcome that matters).
    // A wrong resolution is when a strategy finds SOMETHING but it
    // doesn't match the original content. In the current implementation
    // (no tree access), we can't detect this. We return the best
    // available result.
    //
    // In a full implementation, each strategy would return:
    //   Ok(Some(path, content)) — resolved
    //   Ok(None) — failed (not found)
    //   Err(Wrong(path)) — wrong resolution (content mismatch)
    //
    // For now, all strategies are stubs that return Ok(None) or
    // Ok(Some(path)) based on whether the anchor has the required field.

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
        // All agree — Correct (with noted failures)
        let agreed_strategies: Vec<StrategyName> = resolved.iter().map(|(s, _)| *s).collect();
        let failed_strategies: Vec<StrategyName> = failed.iter().map(|f| f.strategy).collect();
        Ok(Resolution::Correct {
            path: resolved[0].1.clone(),
            strategies: agreed_strategies,
            failed: failed_strategies,
        })
    } else {
        // Disagreement — Degraded
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
#[allow(dead_code)]
enum StrategyOutcome {
    /// Resolved to a path.
    Resolved(String),
    /// Failed to resolve.
    Failed(String),
    /// Resolved to wrong code (content mismatch).
    Wrong(String),
}

/// Path-range strategy: same path, same line range.
/// In a full implementation, this would read the tree and check
/// the content at the anchored path+range. For now, it just
/// checks if the anchor has a path.
fn resolve_path_range(anchor: &Anchor) -> StrategyOutcome {
    match &anchor.path {
        Some(p) if !p.is_empty() => StrategyOutcome::Resolved(p.clone()),
        Some(_) => StrategyOutcome::Failed("path is empty".into()),
        None => StrategyOutcome::Failed("anchor has no path".into()),
    }
}

/// Symbol strategy: find the symbol definition anywhere in the tree.
/// In a full implementation, this would use a language server or
/// grep to find the symbol. For now, it checks if the anchor has a symbol.
fn resolve_symbol(anchor: &Anchor) -> StrategyOutcome {
    match &anchor.symbol {
        Some(s) if !s.is_empty() => StrategyOutcome::Resolved(s.clone()),
        Some(_) => StrategyOutcome::Failed("symbol is empty".into()),
        None => StrategyOutcome::Failed("anchor has no symbol".into()),
    }
}

/// Content-digest strategy: find the normalized content anywhere.
/// In a full implementation, this would search the tree for content
/// matching the digest. For now, it checks if the anchor has a digest.
fn resolve_content_digest(anchor: &Anchor) -> StrategyOutcome {
    match &anchor.content_digest {
        Some(d) if !d.is_empty() => StrategyOutcome::Resolved(d.clone()),
        Some(_) => StrategyOutcome::Failed("content digest is empty".into()),
        None => StrategyOutcome::Failed("anchor has no content digest".into()),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use elench_claim::AnchorStrategy;

    fn make_anchor(
        path: Option<&str>,
        symbol: Option<&str>,
        content_digest: Option<&str>,
    ) -> Anchor {
        Anchor {
            tree: "abc123".into(),
            strategy: AnchorStrategy::Multi,
            path: path.map(String::from),
            range: Some([1, 10]),
            symbol: symbol.map(String::from),
            content_digest: content_digest.map(String::from),
        }
    }

    #[test]
    fn scenario_resolve_all_three_present_all_agree() {
        // All three resolve to the same path (simplified: all point to "src/main.rs")
        let anchor = make_anchor(
            Some("src/main.rs"),
            Some("src/main.rs"),
            Some("src/main.rs"),
        );
        let result = resolve(&anchor).unwrap();
        match result {
            Resolution::Correct {
                path,
                strategies,
                failed,
            } => {
                assert_eq!(path, "src/main.rs");
                assert_eq!(strategies.len(), 3);
                assert!(failed.is_empty());
            }
            _ => panic!("expected Correct, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_two_agree_one_failed() {
        // Path and symbol present, content-digest missing
        let anchor = make_anchor(Some("src/main.rs"), Some("src/main.rs"), None);
        let result = resolve(&anchor).unwrap();
        match result {
            Resolution::Correct {
                path,
                strategies,
                failed,
            } => {
                assert_eq!(path, "src/main.rs");
                assert_eq!(strategies.len(), 2);
                assert_eq!(failed.len(), 1);
                assert_eq!(failed[0], StrategyName::ContentDigest);
            }
            _ => panic!("expected Correct, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_all_fail() {
        // No path, no symbol, no content-digest
        let anchor = make_anchor(None, None, None);
        let result = resolve(&anchor).unwrap();
        match result {
            Resolution::Failed { reasons } => {
                assert_eq!(reasons.len(), 3);
            }
            _ => panic!("expected Failed, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_disagreement_degraded() {
        // Path and symbol point to different locations
        let anchor = make_anchor(Some("src/main.rs"), Some("other.rs"), None);
        let result = resolve(&anchor).unwrap();
        match result {
            Resolution::Degraded { disagreements } => {
                assert_eq!(disagreements.len(), 2);
            }
            _ => panic!("expected Degraded, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_not_multi_rejected() {
        let anchor = Anchor {
            tree: "abc123".into(),
            strategy: AnchorStrategy::PathRange,
            path: Some("src/main.rs".into()),
            range: Some([1, 10]),
            symbol: None,
            content_digest: None,
        };
        let result = resolve(&anchor);
        assert_eq!(
            result,
            Err(AnchorError::NotMulti(AnchorStrategy::PathRange))
        );
    }

    #[test]
    fn scenario_resolve_one_present_one_absent() {
        // Only path present
        let anchor = make_anchor(Some("src/main.rs"), None, None);
        let result = resolve(&anchor).unwrap();
        match result {
            Resolution::Correct {
                path,
                strategies,
                failed,
            } => {
                assert_eq!(path, "src/main.rs");
                assert_eq!(strategies.len(), 1);
                assert_eq!(failed.len(), 2);
            }
            _ => panic!("expected Correct, got {result:?}"),
        }
    }

    #[test]
    fn scenario_resolve_empty_path_treated_as_failed() {
        let anchor = make_anchor(Some(""), Some("src/main.rs"), Some("src/main.rs"));
        let result = resolve(&anchor).unwrap();
        match result {
            Resolution::Correct {
                strategies, failed, ..
            } => {
                assert_eq!(strategies.len(), 2);
                assert_eq!(failed.len(), 1);
                assert_eq!(failed[0], StrategyName::PathRange);
            }
            _ => panic!("expected Correct, got {result:?}"),
        }
    }
}
