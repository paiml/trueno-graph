//! Graph algorithms (BFS, `PageRank`, clustering, topological)
//!
//! Phase 2-4 implementations leveraging aprender for sparse matrix operations.

pub mod louvain;
pub mod pagerank;
pub mod pattern;
pub mod topo;
pub mod traversal;

pub use louvain::{louvain, CommunityDetectionResult};
pub use pagerank::pagerank;
pub use pattern::{find_patterns, Pattern, PatternMatch, Severity};
pub use topo::{is_cyclic, toposort};
pub use traversal::{bfs, find_callers};
