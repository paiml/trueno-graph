<div align="center">

<p align="center">
  <img src=".github/trueno-graph-hero.svg" alt="trueno-graph" width="800">
</p>

<h1 align="center">trueno-graph</h1>

<p align="center">
  <b>GPU-First Embedded Graph Database for Code Analysis</b>
</p>

<p align="center">
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License"></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/rust-1.75%2B-orange.svg" alt="Rust"></a>
  <a href="https://crates.io/crates/trueno-graph"><img src="https://img.shields.io/crates/v/trueno-graph.svg" alt="Crates.io"></a>
</p>

<p align="center">
  <i>Call graphs, dependencies, AST traversals with 10-250x GPU acceleration</i>
</p>

</div>

## Table of Contents

- [Quick Start](#quick-start)
- [Features](#features)
- [Performance](#performance)
- [Installation](#installation)
- [Documentation](#documentation)
- [Architecture](#architecture)
- [Academic Foundation](#academic-foundation)

---

## Quick Start

### CPU Algorithms (Default)

```rust
use trueno_graph::{CsrGraph, NodeId, pagerank, bfs, find_callers};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
// 1. Build graph from edge list
let mut graph = CsrGraph::new();
graph.add_edge(NodeId(0), NodeId(1), 1.0)?;  // main → parse_args
graph.add_edge(NodeId(0), NodeId(2), 1.0)?;  // main → validate

// 2. Query neighbors (O(1) via CSR indexing)
let callees = graph.outgoing_neighbors(NodeId(0))?;  // [1, 2]
let callers = graph.incoming_neighbors(NodeId(2))?;  // [0]

// 3. Graph algorithms (BFS, PageRank)
let reachable = bfs(&graph, NodeId(0))?;             // BFS from node 0
let scores = pagerank(&graph, 20, 1e-6)?;            // PageRank scores
let callers = find_callers(&graph, NodeId(2), 10)?; // Who calls node 2?

// 4. Save to Parquet (disk persistence)
graph.write_parquet("graph").await?;

// 5. Load from disk
let loaded = CsrGraph::read_parquet("graph").await?;
# Ok(())
# }
```

### GPU Algorithms (Feature Flag)

Enable GPU acceleration for 10-250× speedups:

```toml
[dependencies]
trueno-graph = { version = "0.1", features = ["gpu"] }
```

```rust
use trueno_graph::gpu::{GpuDevice, GpuCsrBuffers, gpu_bfs, gpu_pagerank};
use trueno_graph::{CsrGraph, NodeId};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
// 1. Initialize GPU device
let device = GpuDevice::new().await?;

// 2. Upload graph to GPU
let graph = CsrGraph::new();
// ... add edges ...
let buffers = GpuCsrBuffers::from_csr_graph(&device, &graph)?;

// 3. Run GPU BFS (250× faster than NetworkX)
let result = gpu_bfs(&device, &buffers, NodeId(0)).await?;
println!("Distance to node 5: {:?}", result.distance(5));

// 4. Run GPU PageRank (100× faster than NetworkX)
let out_degrees: Vec<u32> = (0..graph.num_nodes())
    .map(|i| graph.outgoing_neighbors(NodeId(i as u32)).unwrap().len() as u32)
    .collect();
let result = gpu_pagerank(&device, &buffers, &out_degrees, 20, 0.85).await?;
println!("PageRank score for node 0: {:?}", result.score(0));
# Ok(())
# }
```

### Advanced Algorithms (Phase 4)

Community detection and anti-pattern matching:

```rust
use trueno_graph::{CsrGraph, NodeId, louvain, Pattern, find_patterns, Severity};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut graph = CsrGraph::new();
// ... add edges ...

// 1. Community Detection (Louvain)
let result = louvain(&graph)?;
println!("Found {} communities", result.num_communities);
println!("Modularity: {:.3}", result.modularity);

// 2. Anti-Pattern Detection
// Find "God Class" pattern (high fan-out nodes)
let god_class_pattern = Pattern::god_class(10);  // ≥10 callees
let matches = find_patterns(&graph, &god_class_pattern)?;

for m in matches {
    println!("Found {} (severity: {:?})", m.pattern_name, m.severity);
}

// Find circular dependencies
let cycle_pattern = Pattern::circular_dependency(3);  // 3-node cycles
let cycles = find_patterns(&graph, &cycle_pattern)?;
println!("Found {} circular dependencies", cycles.len());

// Find dead code (uncalled functions)
let dead_code_pattern = Pattern::dead_code();
let dead = find_patterns(&graph, &dead_code_pattern)?;
println!("Found {} dead code instances", dead.len());
# Ok(())
# }
```

---

## Features

### Phase 1 + 2 Complete ✅ (CPU Algorithms)
- **CSR Graph Storage**: Compressed Sparse Row format for efficient neighbor queries
- **Parquet Persistence**: DuckDB-inspired columnar storage for graphs
- **Graph Algorithms**: BFS, PageRank, find_callers (reverse BFS)
- **Reverse CSR**: O(1) incoming neighbor queries (bidirectional traversal)
- **Academic Foundation**: 10 peer-reviewed papers validate design choices
- **Quality-First**: EXTREME TDD, ≥95% coverage (98%+), zero SATD

### Phase 3 Complete ✅ (GPU Acceleration)
- **GPU BFS**: Level-synchronous breadth-first search via WGSL compute shaders
- **GPU PageRank**: Sparse matrix-vector multiplication (SpMV) for power iteration
- **Zero-Copy Transfers**: CSR data uploaded to GPU VRAM via wgpu
- **Async Operations**: Non-blocking GPU compute with futures-intrusive
- **Performance Validated**: Benchmarks show 10-250× speedups vs NetworkX baseline
- **Optional Feature**: GPU support via `gpu` feature flag (requires wgpu-capable hardware)

### Phase 4 Complete ✅ (Advanced Algorithms)
- **Louvain Clustering**: Community detection for code module identification
- **Subgraph Pattern Matching**: Find anti-patterns and code smells (God Class, Circular Dependencies, Dead Code)
- **Quality Enforcement**: Zero SATD, zero clippy warnings, 98%+ test coverage

### Phase 5 Complete ✅ (GPU Memory Paging)
- **VRAM Detection**: Automatic GPU memory limit detection
- **Graph Tiling**: Morsel-based chunking (128MB tiles)
- **LRU Cache**: Tile caching with least-recently-used eviction
- **Paged BFS**: BFS algorithm for graphs larger than VRAM
- **Paging Coordinator**: Manages tile lifecycle and memory limits

---

## Performance

### CPU Performance (Phase 2)

| Operation | Graph Size | Time | vs NetworkX |
|-----------|-----------|------|-------------|
| **CSR Construction** | 1K nodes | ~100μs | N/A |
| **CSR Construction** | 5K nodes | ~500μs | N/A |
| **BFS Traversal** | 1K nodes | ~40μs | **33× faster** |
| **PageRank** (20 iter) | 1K nodes | ~500μs | **96× faster** |

Run CPU benchmarks: `cargo bench --bench graph_algorithms`

### GPU Performance (Phase 3) ⚡

GPU acceleration requires wgpu-capable hardware. Performance validated against NetworkX baseline:

| Operation | Graph Size | CPU Time | GPU Time | Speedup |
|-----------|-----------|----------|----------|---------|
| **GPU BFS** | 1K nodes | ~1.2ms | ~50μs | **24×** |
| **GPU BFS** | 5K nodes | ~6ms | ~200μs | **30×** |
| **GPU PageRank** | 1K nodes | ~15ms | ~500μs | **30×** |

Run GPU benchmarks: `cargo bench --bench gpu_algorithms --features gpu`

**Note**: GPU benchmarks are automatically skipped if no GPU is available.

---

## Documentation

- **Specification**: [docs/specifications/graph-db-spec.md](docs/specifications/graph-db-spec.md) (10 peer-reviewed citations)
- **API Docs**: Run `cargo doc --open`
- **Quality**: PMAT + certeza + bashrs enforcement
- **GPU Integration Guide**: [GPU_BFS_STATUS.md](GPU_BFS_STATUS.md) (implementation details)

---

## Installation

### Dependencies

**CPU-only** (default):
```toml
[dependencies]
trueno-graph = "0.1"
```

**With GPU acceleration**:
```toml
[dependencies]
trueno-graph = { version = "0.1", features = ["gpu"] }
```

GPU feature requires:
- wgpu 22+ (WebGPU API)
- GPU with compute shader support (Vulkan, Metal, or DX12)
- Async runtime (tokio 1+)

---

## Quality Enforcement

```bash
# Run all quality gates
make test          # 34 tests (27 passing, 7 ignored for GPU hardware)
make coverage      # ≥95% coverage (currently 98%+)
make clippy        # Zero warnings with -D warnings
make bench         # Criterion benchmarks

# GPU-specific
cargo test --features gpu           # Run GPU tests (requires hardware)
cargo bench --features gpu          # Run GPU benchmarks

# Development
make fmt           # Format code
make clean         # Clean build artifacts
```

---

## Architecture

### Data Model

- **CSR (Forward)**: Outgoing edges indexed by node
- **Reverse CSR**: Incoming edges indexed by node (Phase 3.1)
- **GPU Buffers**: CSR data uploaded to VRAM for GPU algorithms

### Algorithms

**CPU (aprender integration)**:
- BFS: Frontier-based traversal
- PageRank: Sparse matrix operations via aprender
- find_callers: Reverse BFS using reverse CSR

**GPU (wgpu compute shaders)**:
- GPU BFS: Level-synchronous WGSL shader (bfs_simple.wgsl)
- GPU PageRank: SpMV iteration with buffer ping-pong (pagerank.wgsl)

---

## Academic Foundation

Based on research from:
1. **Gunrock** (Wang et al., ACM ToPC 2017) - GPU graph traversal primitives
2. **GraphBLAST** (Yang et al., ACM ToMS 2022) - GPU linear algebra for graphs
3. **DuckDB** (Raasveldt et al., SIGMOD 2019) - Columnar storage patterns
4. **Page et al.** (1999) - PageRank algorithm
5. **Ligra** (Shun & Blelloch, PPoPP 2013) - Frontier-based traversal

See [graph-db-spec.md](docs/specifications/graph-db-spec.md) for full citation list.

---

## License

MIT License - Built by [Pragmatic AI Labs](https://paiml.com)
