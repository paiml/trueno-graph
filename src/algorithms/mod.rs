//! Graph algorithms (BFS, `PageRank`, clustering)
//!
//! Phase 2 implementations leveraging aprender for sparse matrix operations.

pub mod pagerank;
pub mod traversal;

pub use pagerank::pagerank;
pub use traversal::{bfs, find_callers};
