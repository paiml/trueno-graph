# trueno-graph

**GPU-first embedded graph database for code analysis (call graphs, dependencies, AST traversals)**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

---

## Quick Start

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

---

## Features

### Phase 1 + 2 Complete ✅
- **CSR Graph Storage**: Compressed Sparse Row format for efficient neighbor queries
- **Parquet Persistence**: DuckDB-inspired columnar storage for graphs
- **Graph Algorithms**: BFS, PageRank, find_callers (reverse BFS)
- **Academic Foundation**: 10 peer-reviewed papers validate design choices
- **Quality-First**: EXTREME TDD, ≥95% coverage (97.86%), zero SATD

### Phase 3 Planned 🚧
- **GPU Acceleration**: wgpu compute shaders for 25-250x speedups
- **Zero-Copy Transfers**: Arrow RecordBatch → GPU VRAM (via trueno primitives)
- **Advanced Algorithms**: Louvain clustering, betweenness centrality

---

## Performance

Current implementation (Phase 2 - CPU only):

| Operation | Graph Size | Time |
|-----------|-----------|------|
| **CSR Construction** | 1K nodes | ~100μs |
| **CSR Construction** | 5K nodes | ~500μs |
| **BFS Traversal** | 1K nodes | ~40μs |
| **PageRank** (20 iter) | 1K nodes | ~500μs |

*Note: GPU acceleration (Phase 3) will provide 25-250x speedups vs NetworkX baseline*

Run benchmarks: `cargo bench --bench graph_algorithms`

---

## Documentation

- **Specification**: [docs/specifications/graph-db-spec.md](docs/specifications/graph-db-spec.md) (10 peer-reviewed citations)
- **API Docs**: Run `cargo doc --open`
- **Quality**: PMAT + certeza + bashrs enforcement

---

## Quality Enforcement

```bash
# Run all quality gates
make test          # 32 tests (20 unit + 4 integration + 8 property)
make coverage      # ≥95% coverage (currently 97.86%)
make clippy        # Zero warnings with -D warnings
make bench         # Criterion benchmarks

# Development
make fmt           # Format code
make clean         # Clean build artifacts
```

---

## License

MIT License - Built by [Pragmatic AI Labs](https://paiml.com)
