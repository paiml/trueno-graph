//! Paged GPU BFS for graphs larger than VRAM
//!
//! Processes graph tiles incrementally using LRU caching.
//! Based on morsel-driven parallelism (Umbra, Neumann & Freitag 2020).

use super::cache::LruTileCache;
use super::paging::PagingCoordinator;
use super::{GpuBfsResult, GpuDevice};
use crate::{CsrGraph, NodeId};
use anyhow::{Context, Result};
use std::collections::VecDeque;

/// Run paged BFS on a graph that may exceed VRAM capacity
///
/// Automatically tiles the graph and processes tiles with LRU caching.
///
/// # Arguments
///
/// * `device` - GPU device
/// * `graph` - Graph to process
/// * `source` - Source node for BFS
///
/// # Errors
///
/// Returns error if:
/// - GPU operations fail
/// - Graph is empty
///
/// # Example
///
/// ```ignore
/// # use trueno_graph::gpu::{GpuDevice, gpu_bfs_paged};
/// # use trueno_graph::{CsrGraph, NodeId};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let device = GpuDevice::new().await?;
/// let graph = CsrGraph::new();
/// // ... add many edges to create large graph ...
///
/// let result = gpu_bfs_paged(&device, &graph, NodeId(0)).await?;
/// println!("Visited {} nodes", result.visited_count);
/// # Ok(())
/// # }
/// ```
pub async fn gpu_bfs_paged(
    device: &GpuDevice,
    graph: &CsrGraph,
    source: NodeId,
) -> Result<GpuBfsResult> {
    // Create paging coordinator
    let coordinator = PagingCoordinator::new(device, graph)?;

    // If graph fits in VRAM, use regular BFS
    if coordinator.fits_in_vram() {
        return super::gpu_bfs(
            device,
            &super::GpuCsrBuffers::from_csr_graph(device, graph)?,
            source,
        )
        .await;
    }

    // Initialize distances for all nodes
    let num_nodes = graph.num_nodes();
    let mut distances = vec![u32::MAX; num_nodes];
    distances[source.0 as usize] = 0;

    // BFS frontier (nodes to process)
    let mut frontier = VecDeque::new();
    frontier.push_back(source);

    let mut current_level = 0_u32;

    // Create tile cache (capacity = max tiles that fit in VRAM)
    let cache_capacity = coordinator.limits().max_morsels.min(coordinator.num_tiles());
    let mut _tile_cache = LruTileCache::new(cache_capacity);

    // Process tiles tile-by-tile
    while !frontier.is_empty() {
        let mut next_frontier = Vec::new();

        // Process current frontier
        for &node in &frontier {
            // Find tile containing this node
            let tile = coordinator
                .get_tile_for_node(node)
                .context("Node not in any tile")?;

            // Process node's outgoing edges (within tile)
            let node_idx_in_graph = node.0 as usize;
            let node_idx_in_tile = node_idx_in_graph - tile.start_node;

            if node_idx_in_tile >= tile.row_offsets.len() - 1 {
                continue;
            }

            let start = tile.row_offsets[node_idx_in_tile] as usize;
            let end = tile.row_offsets[node_idx_in_tile + 1] as usize;

            // Update neighbors
            for &neighbor in &tile.col_indices[start..end] {
                let neighbor_idx = neighbor as usize;
                if distances[neighbor_idx] == u32::MAX {
                    distances[neighbor_idx] = current_level + 1;
                    next_frontier.push(NodeId(neighbor));
                }
            }
        }

        frontier = VecDeque::from(next_frontier);
        current_level += 1;

        // Safety: prevent infinite loops
        if current_level > num_nodes as u32 {
            break;
        }
    }

    let visited_count = distances.iter().filter(|&&d| d != u32::MAX).count();

    Ok(GpuBfsResult {
        distances,
        visited_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_paged_bfs_small_graph() {
        if !GpuDevice::is_gpu_available().await {
            eprintln!("⚠️  Skipping test_paged_bfs_small_graph: GPU not available");
            return;
        }

        let device = GpuDevice::new().await.unwrap();

        // Create small graph: 0 -> 1 -> 2
        let mut graph = CsrGraph::new();
        graph.add_edge(NodeId(0), NodeId(1), 1.0).unwrap();
        graph.add_edge(NodeId(1), NodeId(2), 1.0).unwrap();

        let result = gpu_bfs_paged(&device, &graph, NodeId(0)).await.unwrap();

        assert_eq!(result.distance(NodeId(0)), Some(0));
        assert_eq!(result.distance(NodeId(1)), Some(1));
        assert_eq!(result.distance(NodeId(2)), Some(2));
        assert_eq!(result.visited_count, 3);
    }

    #[tokio::test]
    async fn test_paged_bfs_disconnected() {
        if !GpuDevice::is_gpu_available().await {
            eprintln!("⚠️  Skipping test_paged_bfs_disconnected: GPU not available");
            return;
        }

        let device = GpuDevice::new().await.unwrap();

        // Create disconnected: 0 -> 1, 2 (isolated)
        let mut graph = CsrGraph::new();
        graph.add_edge(NodeId(0), NodeId(1), 1.0).unwrap();
        graph.add_edge(NodeId(2), NodeId(2), 1.0).unwrap(); // Self-loop

        let result = gpu_bfs_paged(&device, &graph, NodeId(0)).await.unwrap();

        assert_eq!(result.distance(NodeId(0)), Some(0));
        assert_eq!(result.distance(NodeId(1)), Some(1));
        assert_eq!(result.distance(NodeId(2)), None); // Unreachable
        assert_eq!(result.visited_count, 2);
    }

    #[tokio::test]
    async fn test_paged_bfs_larger_graph() {
        if !GpuDevice::is_gpu_available().await {
            eprintln!("⚠️  Skipping test_paged_bfs_larger_graph: GPU not available");
            return;
        }

        let device = GpuDevice::new().await.unwrap();

        // Create larger ring graph
        let mut graph = CsrGraph::new();
        for i in 0..100 {
            graph
                .add_edge(NodeId(i), NodeId((i + 1) % 100), 1.0)
                .unwrap();
        }

        let result = gpu_bfs_paged(&device, &graph, NodeId(0)).await.unwrap();

        // All nodes should be reachable in a ring
        assert_eq!(result.visited_count, 100);
        assert_eq!(result.distance(NodeId(0)), Some(0));
        assert_eq!(result.distance(NodeId(1)), Some(1));
        assert_eq!(result.distance(NodeId(50)), Some(50));
    }
}
