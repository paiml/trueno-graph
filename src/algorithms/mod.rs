//! Graph algorithms (BFS, `PageRank`, clustering)
//!
//! Phase 2-4 implementations leveraging aprender for sparse matrix operations.

pub mod louvain;
pub mod pagerank;
pub mod traversal;

pub use louvain::{louvain, CommunityDetectionResult};
pub use pagerank::pagerank;
pub use traversal::{bfs, find_callers};
