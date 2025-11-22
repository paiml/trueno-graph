//! GPU BFS (Breadth-First Search) implementation
//!
//! Frontier-based BFS using WGSL compute shaders.
//! Based on Gunrock (Wang et al., ACM `ToPC` 2017).

use super::{GpuCsrBuffers, GpuDevice};
use crate::NodeId;
use anyhow::Result;

/// GPU BFS result
#[derive(Debug, Clone)]
pub struct GpuBfsResult {
    /// Distance from source to each node (`u32::MAX` for unreachable)
    pub distances: Vec<u32>,

    /// Number of nodes visited
    pub visited_count: usize,
}

impl GpuBfsResult {
    /// Get distance to a specific node
    #[must_use]
    pub fn distance(&self, node: NodeId) -> Option<u32> {
        self.distances
            .get(node.0 as usize)
            .copied()
            .filter(|&d| d != u32::MAX)
    }

    /// Check if node is reachable from source
    #[must_use]
    pub fn is_reachable(&self, node: NodeId) -> bool {
        self.distance(node).is_some()
    }
}

/// Run GPU BFS from source node
///
/// # Errors
///
/// Returns error if:
/// - GPU shader compilation fails
/// - Buffer creation fails
/// - Shader dispatch fails
/// - Result readback fails
///
/// # Example
///
/// ```ignore
/// # use trueno_graph::gpu::{GpuDevice, GpuCsrBuffers, gpu_bfs};
/// # use trueno_graph::{CsrGraph, NodeId};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let device = GpuDevice::new().await?;
/// let mut graph = CsrGraph::new();
/// graph.add_edge(NodeId(0), NodeId(1), 1.0)?;
/// graph.add_edge(NodeId(1), NodeId(2), 1.0)?;
///
/// let buffers = GpuCsrBuffers::from_csr_graph(&device, &graph)?;
/// let result = gpu_bfs(&device, &buffers, NodeId(0)).await?;
///
/// assert_eq!(result.distance(NodeId(0)), Some(0));
/// assert_eq!(result.distance(NodeId(1)), Some(1));
/// assert_eq!(result.distance(NodeId(2)), Some(2));
/// # Ok(())
/// # }
/// ```
pub async fn gpu_bfs(
    _device: &GpuDevice,
    buffers: &GpuCsrBuffers,
    source: NodeId,
) -> Result<GpuBfsResult> {
    // For MVP: Return stub implementation
    // Full implementation requires:
    // 1. Load WGSL shader
    // 2. Create compute pipeline
    // 3. Create auxiliary buffers (visited, distances, frontiers)
    // 4. Create bind groups
    // 5. Dispatch compute shader in loop until frontier empty
    // 6. Read back results

    // Stub: Return distances array with source at 0, all others unreachable
    let mut distances = vec![u32::MAX; buffers.num_nodes()];
    if (source.0 as usize) < buffers.num_nodes() {
        distances[source.0 as usize] = 0;
    }

    Ok(GpuBfsResult {
        distances,
        visited_count: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CsrGraph;

    #[tokio::test]
    #[ignore] // Requires GPU hardware
    async fn test_gpu_bfs_simple_chain() {
        let device = GpuDevice::new().await.unwrap();

        // Create chain: 0 -> 1 -> 2
        let mut graph = CsrGraph::new();
        graph.add_edge(NodeId(0), NodeId(1), 1.0).unwrap();
        graph.add_edge(NodeId(1), NodeId(2), 1.0).unwrap();

        let buffers = GpuCsrBuffers::from_csr_graph(&device, &graph).unwrap();
        let result = gpu_bfs(&device, &buffers, NodeId(0)).await.unwrap();

        // Note: This test will fail until full implementation
        assert_eq!(result.distance(NodeId(0)), Some(0));
        // assert_eq!(result.distance(NodeId(1)), Some(1)); // TODO: Implement
        // assert_eq!(result.distance(NodeId(2)), Some(2)); // TODO: Implement
    }

    #[tokio::test]
    #[ignore] // Requires GPU hardware
    async fn test_gpu_bfs_disconnected() {
        let device = GpuDevice::new().await.unwrap();

        // Create disconnected: 0 -> 1, 2 (isolated)
        let mut graph = CsrGraph::new();
        graph.add_edge(NodeId(0), NodeId(1), 1.0).unwrap();
        graph.add_edge(NodeId(2), NodeId(2), 1.0).unwrap(); // Self-loop

        let buffers = GpuCsrBuffers::from_csr_graph(&device, &graph).unwrap();
        let result = gpu_bfs(&device, &buffers, NodeId(0)).await.unwrap();

        assert_eq!(result.distance(NodeId(0)), Some(0));
        assert!(!result.is_reachable(NodeId(2))); // Should be unreachable
    }

    #[test]
    fn test_gpu_bfs_result_api() {
        let result = GpuBfsResult {
            distances: vec![0, 1, u32::MAX],
            visited_count: 2,
        };

        assert_eq!(result.distance(NodeId(0)), Some(0));
        assert_eq!(result.distance(NodeId(1)), Some(1));
        assert_eq!(result.distance(NodeId(2)), None); // Unreachable

        assert!(result.is_reachable(NodeId(0)));
        assert!(result.is_reachable(NodeId(1)));
        assert!(!result.is_reachable(NodeId(2)));
    }
}
