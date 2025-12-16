<div align="center">

<img src=".github/trueno-graph-hero.svg" alt="trueno-graph" width="600">

**GPU-First Embedded Graph Database for Code Analysis**

[![CI](https://github.com/paiml/trueno-graph/actions/workflows/ci.yml/badge.svg)](https://github.com/paiml/trueno-graph/actions)

</div>

---

Call graphs, dependencies, AST traversals with 10-250x GPU acceleration.

## Features

- **CSR Storage**: Compressed Sparse Row for O(1) neighbor queries
- **GPU Acceleration**: BFS (250x), PageRank (100x) via WGSL shaders
- **Parquet Persistence**: DuckDB-inspired columnar storage
- **Louvain Clustering**: Community detection for code modules
- **Anti-Pattern Detection**: God Class, Circular Dependencies, Dead Code
- **VRAM Paging**: Morsel-based tiling for large graphs

## Installation

```toml
[dependencies]
trueno-graph = "0.1"

# Optional: GPU acceleration
trueno-graph = { version = "0.1", features = ["gpu"] }
```

## Quick Start

```rust
use trueno_graph::{CsrGraph, NodeId, pagerank, bfs};

let mut graph = CsrGraph::new();
graph.add_edge(NodeId(0), NodeId(1), 1.0)?;
graph.add_edge(NodeId(0), NodeId(2), 1.0)?;

// Graph algorithms
let reachable = bfs(&graph, NodeId(0))?;
let scores = pagerank(&graph, 20, 1e-6)?;

// Persistence
graph.write_parquet("graph").await?;
```

## GPU Usage

```rust
use trueno_graph::gpu::{GpuDevice, GpuCsrBuffers, gpu_bfs};

let device = GpuDevice::new().await?;
let buffers = GpuCsrBuffers::from_csr_graph(&device, &graph)?;
let result = gpu_bfs(&device, &buffers, NodeId(0)).await?;
```

## Performance

| Operation | Graph Size | CPU | GPU | Speedup |
|-----------|-----------|-----|-----|---------|
| BFS | 5K nodes | 6ms | 200µs | 30x |
| PageRank | 1K nodes | 15ms | 500µs | 30x |

## Development

```bash
make test      # Run tests
make coverage  # ≥95% coverage
make bench     # Benchmarks
```

## License

MIT
