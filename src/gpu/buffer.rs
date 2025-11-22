//! GPU buffer management for CSR graph data
//!
//! Handles uploading CSR (`row_offsets`, `col_indices`, `edge_weights`) to GPU
//! and downloading results (distances, visited arrays) from GPU.

use super::GpuDevice;
use crate::storage::CsrGraph;
use anyhow::Result;

/// GPU buffers for CSR graph representation
///
/// Manages GPU-side storage of:
/// - Row offsets (CSR row pointers)
/// - Column indices (CSR column indices)
/// - Edge weights (optional)
/// - Auxiliary arrays (visited, distances, etc.)
#[derive(Debug)]
pub struct GpuCsrBuffers {
    /// Number of nodes in the graph
    pub num_nodes: usize,

    /// Number of edges in the graph
    pub num_edges: usize,

    /// GPU buffer for `row_offsets` (size: `num_nodes` + 1)
    pub row_offsets: wgpu::Buffer,

    /// GPU buffer for `col_indices` (size: `num_edges`)
    pub col_indices: wgpu::Buffer,

    /// GPU buffer for `edge_weights` (size: `num_edges`, optional)
    pub edge_weights: Option<wgpu::Buffer>,
}

impl GpuCsrBuffers {
    /// Upload CSR graph to GPU
    ///
    /// # Errors
    ///
    /// Returns error if buffer creation fails
    pub fn from_csr_graph(device: &GpuDevice, graph: &CsrGraph) -> Result<Self> {
        let num_nodes = graph.num_nodes();
        let num_edges = graph.num_edges();

        // Create row_offsets buffer
        let row_offsets_data = graph.row_offsets_slice();
        let row_offsets = device.create_buffer_init(
            "CSR row_offsets",
            bytemuck::cast_slice(row_offsets_data),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        )?;

        // Create col_indices buffer
        let col_indices_data = graph.col_indices_slice();
        let col_indices = device.create_buffer_init(
            "CSR col_indices",
            bytemuck::cast_slice(col_indices_data),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        )?;

        // Create edge_weights buffer (optional for unweighted graphs)
        let edge_weights_data = graph.edge_weights_slice();
        let edge_weights = if edge_weights_data.is_empty() {
            None
        } else {
            Some(device.create_buffer_init(
                "CSR edge_weights",
                bytemuck::cast_slice(edge_weights_data),
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            )?)
        };

        Ok(Self {
            num_nodes,
            num_edges,
            row_offsets,
            col_indices,
            edge_weights,
        })
    }

    /// Get number of nodes
    #[must_use]
    pub const fn num_nodes(&self) -> usize {
        self.num_nodes
    }

    /// Get number of edges
    #[must_use]
    pub const fn num_edges(&self) -> usize {
        self.num_edges
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeId;

    #[tokio::test]
    async fn test_upload_csr_to_gpu() {
        if !GpuDevice::is_gpu_available().await {
            eprintln!("⚠️  Skipping test_upload_csr_to_gpu: GPU not available");
            return;
        }

        let device = GpuDevice::new().await.unwrap();

        // Create simple graph: 0 -> 1 -> 2
        let mut graph = CsrGraph::new();
        graph.add_edge(NodeId(0), NodeId(1), 1.0).unwrap();
        graph.add_edge(NodeId(1), NodeId(2), 1.0).unwrap();

        // Upload to GPU
        let buffers = GpuCsrBuffers::from_csr_graph(&device, &graph).unwrap();

        assert_eq!(buffers.num_nodes(), 3);
        assert_eq!(buffers.num_edges(), 2);
    }

    #[tokio::test]
    async fn test_upload_empty_graph() {
        if !GpuDevice::is_gpu_available().await {
            eprintln!("⚠️  Skipping test_upload_empty_graph: GPU not available");
            return;
        }

        let device = GpuDevice::new().await.unwrap();
        let graph = CsrGraph::new();

        let buffers = GpuCsrBuffers::from_csr_graph(&device, &graph).unwrap();

        assert_eq!(buffers.num_nodes(), 0);
        assert_eq!(buffers.num_edges(), 0);
    }
}
