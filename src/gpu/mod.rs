//! GPU acceleration for graph algorithms
//!
//! Based on research from:
//! - **Gunrock** (Wang et al., ACM `ToPC` 2017) - GPU graph traversal primitives
//! - **`cuGraph`** (Bader et al., 2022) - GPU-accelerated graph analytics
//! - **`GraphBLAST`** (Yang et al., 2022) - GPU linear algebra for graphs
//!
//! # Architecture
//!
//! - `device`: GPU device initialization and management
//! - `buffer`: GPU buffer management for CSR data
//! - `kernels`: WGSL compute shaders for BFS, `PageRank`, etc.
//!
//! # Feature Flag
//!
//! This module is only available with the `gpu` feature flag:
//! ```bash
//! cargo build --features gpu
//! ```

mod bfs;
mod buffer;
mod device;
mod pagerank;

pub use bfs::{gpu_bfs, GpuBfsResult};
pub use buffer::GpuCsrBuffers;
pub use device::{GpuDevice, GpuDeviceError};
pub use pagerank::{gpu_pagerank, GpuPageRankResult};
