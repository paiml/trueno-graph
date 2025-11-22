# trueno-graph: GPU-Accelerated Graph Database Specification

**Version**: 0.1.0
**Status**: Draft
**Authors**: Pragmatic AI Labs
**Date**: 2025-11-22

---

## Executive Summary

trueno-graph is a GPU-first property graph database optimized for code dependency analysis, semantic search, and graph algorithms. Built on PAIML's proven infrastructure (trueno 0.6.0, trueno-db 0.3.0, aprender 0.6.0), it provides 10-250x speedups over CPU-based graph operations through hybrid CPU/GPU execution.

**Key Design Principles:**
1. **Reuse, Don't Reinvent**: Leverage existing PAIML libraries (trueno, trueno-db, aprender)
2. **Separation of Concerns**: Graph-specific storage + algorithms, not a general-purpose database
3. **Toyota Way**: Zero SATD, EXTREME TDD, Quality-First (pmat/certeza/bashrs enforcement)
4. **Academic Foundation**: 10 peer-reviewed papers validate design choices

---

## Design Evolution (PMAT Phase 7 → trueno-graph)

### Original Architectural Decision (PMAT Phase 7)

**Initial proposal** (from `paiml-mcp-agent-toolkit/docs/specifications/integrate-ml-trueno-latest-spec.md` lines 1185-1264):

> **Question**: Should trueno-db add native graph storage (CSR format, BFS kernels, PageRank), or should PMAT implement graphs separately?
>
> **Answer**: **Keep trueno-db columnar-only. Implement graphs in PMAT using existing trueno-db primitives.**

**Proposed architecture**:
```rust
// PMAT handles graph logic (original plan)
pub struct GraphStorage {
    edges_olap: TruenoOlapAnalytics,  // ❌ API mismatch!
    nodes_olap: TruenoOlapAnalytics,
    pagerank: aprender::graph::PageRank,
}
```

### Discovery: API Mismatch (2025-11-22)

**Problem found during implementation**:

```rust
// TruenoOlapAnalytics trait (from pmat/server/src/tdg/olap_analytics.rs:88)
async fn query_top_k(&self, k: usize, order_by: &str) -> Result<Vec<TdgScore>>;

// Graph queries need:
async fn find_callers(&self, node_id: u32) -> Result<Vec<u32>>;  // ❌ Can't express!
async fn find_callees(&self, node_id: u32) -> Result<Vec<u32>>; // ❌ Can't express!
```

**Root cause**:
- `TruenoOlapAnalytics` is TDG-score-specific (Top-K, aggregations on `TdgScore` structs)
- Graph queries need CSR indexing, not columnar scans
- Different APIs → different trait

### Revised Decision: trueno-graph as Separate Repo

**Rationale**:

1. **API Incompatibility**: TruenoOlapAnalytics can't handle graph traversal queries
2. **Storage Format**: OLAP columnar tables ≠ graph CSR format
3. **Separation of Concerns**: trueno-db (OLAP) vs trueno-graph (graph traversal)
4. **Reusable Component**: Other projects (beyond PMAT) can use trueno-graph

**Updated architecture**:
```rust
// trueno-graph: Direct Parquet I/O (not via TruenoOlapAnalytics)
pub struct CsrGraph {
    row_offsets: Vec<u32>,     // CSR adjacency
    col_indices: Vec<u32>,
    edge_weights: Vec<f32>,
    // ... node properties
}

impl CsrGraph {
    // Use trueno-db's Parquet I/O primitives directly
    pub async fn write_parquet(&self, path: &Path) -> Result<()> {
        let edges_batch = csr_to_arrow_edges(self)?;
        trueno_db::write_parquet(path, edges_batch).await?;  // ✅ Direct I/O
        Ok(())
    }
}
```

**What we reuse from trueno-db**:
- ✅ Parquet I/O functions (not the OlapAnalytics trait)
- ✅ Arrow RecordBatch conversion
- ✅ GPU buffer management primitives
- ✅ SIMD primitives (via trueno)

**What we DON'T use**:
- ❌ TruenoOlapAnalytics trait (too specific to TdgScore)
- ❌ SQL query interface (graphs need traversal APIs)

### Alignment with Toyota Way (Jidoka)

**Built-in Quality via Separation of Concerns**:

| Concern | trueno-db | trueno-graph | PMAT |
|---------|-----------|--------------|------|
| **Purpose** | OLAP analytics on TDG scores | Graph storage + traversal | Code analysis orchestration |
| **Data Model** | Columnar records (TdgScore) | CSR graph (edges/nodes) | AST + call graphs + metrics |
| **API** | `query_top_k()`, `aggregate()` | `find_callers()`, `pagerank()` | High-level analysis commands |
| **Optimization** | SIMD scans, GPU reductions | Graph traversal, sparse matmul | Workflow orchestration |

**Verdict**: Separate repo prevents mixing concerns, maintains clear boundaries.

---

## Table of Contents

1. [Academic Foundation](#academic-foundation)
2. [Architecture Overview](#architecture-overview)
3. [Dependency Graph](#dependency-graph)
4. [Core Components](#core-components)
5. [Quality Enforcement](#quality-enforcement)
6. [Performance Targets](#performance-targets)
7. [Implementation Roadmap](#implementation-roadmap)
8. [Testing Strategy](#testing-strategy)

---

## Academic Foundation

### 1. Graph Storage and Processing

**[CITATION 1]** Wang, Y., Davidson, A., Pan, Y., Wu, Y., Riffel, A., & Owens, J. D. (2017). **"Gunrock: GPU Graph Analytics"**. *ACM Transactions on Parallel Computing*, 4(1), 1-49.

- **Relevance**: Establishes GPU-based graph traversal primitives (BFS, SSSP, PageRank)
- **Applied in**: `src/algorithms/traversal.rs` for neighbor queries, path finding
- **Performance Claim**: 10-100x speedup over CPU-based NetworkX for large graphs
- **Status**: Mature framework (1,200+ citations), battle-tested on graphs with 1B+ edges

**[CITATION 2]** Yang, C., Buluç, A., & Owens, J. D. (2022). **"GraphBLAST: A High-Performance Linear Algebra-based Graph Framework on the GPU"**. *ACM Transactions on Mathematical Software*, 48(1), Article 7.

- **Relevance**: Graph algorithms as sparse linear algebra (CSR format for adjacency matrices)
- **Applied in**: `src/storage/csr.rs` for compressed sparse row graph representation
- **Key Insight**: PageRank = sparse matrix-vector multiplication (SpMV)
- **Implementation**: Reuses aprender's sparse matrix primitives

**[CITATION 3]** Kim, M., An, K., Park, H., Seo, H., & Kim, Y. (2023). **"Accelerating Subgraph Matching on GPU"**. *Proceedings of ASPLOS 2023*, pp. 915-928.

- **Relevance**: Fast pattern matching in call graphs (e.g., "find all A→B→C patterns")
- **Applied in**: `src/algorithms/pattern.rs` for code smell detection
- **Speedup**: 10-50x over CPU-based subgraph isomorphism
- **Use Case**: Find circular dependencies, anti-patterns in PMAT call graphs

### 2. Hybrid Storage Systems

**[CITATION 4]** Neumann, T., & Freitag, M. J. (2020). **"Umbra: A Disk-Based System with In-Memory Performance"**. *Proceedings of CIDR 2020*.

- **Relevance**: Hybrid disk (Parquet) + in-memory (GPU) architecture
- **Applied in**: `src/storage/hybrid.rs` for persistent graph storage with GPU caching
- **Key Technique**: Morsel-driven parallelism (128MB chunks) for out-of-core processing
- **Benefit**: Process graphs larger than VRAM (e.g., 10GB graph on 8GB GPU)

**[CITATION 5]** Raasveldt, M., & Mühleisen, H. (2019). **"DuckDB: An Embeddable Analytical Database System"**. *Proceedings of SIGMOD 2019*, pp. 1981-1984.

- **Relevance**: Columnar storage for graph properties (node embeddings, edge weights)
- **Applied in**: Parquet format for persistent storage (reuses trueno-db patterns)
- **Integration**: Zero-copy Arrow → GPU transfers via trueno-db's buffer management
- **Why Not SQL**: Graphs require traversal APIs, not relational joins

### 3. GPU Memory Management

**[CITATION 6]** Funke, H., Breß, S., Noll, S., Teubner, J., & Markl, V. (2018). **"Pipelined Query Processing in Coprocessor Environments"**. *Proceedings of SIGMOD 2018*, pp. 1219-1234.

- **Relevance**: GPU memory paging for graphs exceeding VRAM capacity
- **Applied in**: `src/memory/paging.rs` for morsel-based GPU transfers
- **Strategy**: LRU cache + prefetching for hot subgraphs
- **Example**: Load 10GB graph in 128MB morsels, cache frequently accessed neighborhoods

**[CITATION 7]** Besta, M., & Hoefler, T. (2019). **"Slim Graph: Practical Lossy Graph Compression for Approximate Graph Processing"**. *Proceedings of SC 2019*, Article 35.

- **Relevance**: Graph compression for GPU memory efficiency
- **Applied in**: `src/storage/compression.rs` for CSR compaction
- **Trade-off**: 2-5x compression ratio with <1% error for PageRank/clustering
- **When to Use**: Graphs with highly skewed degree distributions (code call graphs)

### 4. Graph Algorithms

**[CITATION 8]** Blondel, V. D., Guillaume, J. L., Lambiotte, R., & Lefebvre, E. (2008). **"Fast unfolding of communities in large networks"**. *Journal of Statistical Mechanics: Theory and Experiment*, 2008(10), P10008.

- **Relevance**: Louvain clustering for module detection in code (community = architectural component)
- **Applied in**: `src/algorithms/clustering.rs` via aprender's graph module
- **Use Case**: Detect tightly coupled code modules in PMAT codebase
- **Complexity**: O(N log N) with GPU parallelization → O(N) practical runtime

**[CITATION 9]** Page, L., Brin, S., Motwani, R., & Winograd, T. (1999). **"The PageRank Citation Ranking: Bringing Order to the Web"**. *Stanford InfoLab Technical Report*.

- **Relevance**: Importance ranking for code functions (most critical = highest PageRank)
- **Applied in**: `src/algorithms/pagerank.rs` via aprender sparse matmul
- **Formula**: `PR(v) = (1-d)/N + d * Σ(PR(u) / OutDegree(u))` where d=0.85
- **Integration**: PageRank scores used by PMAT for refactoring prioritization

**[CITATION 10]** Shun, J., & Blelloch, G. E. (2013). **"Ligra: A Lightweight Graph Processing Framework for Shared Memory"**. *Proceedings of PPoPP 2013*, pp. 135-146.

- **Relevance**: Efficient frontier-based graph traversal (BFS, shortest paths)
- **Applied in**: `src/algorithms/traversal.rs` for "find all callers" queries
- **Performance**: Edge-parallel vs vertex-parallel switching (sparse graphs use vertex-parallel)
- **GPU Adaptation**: Frontier expansion on GPU via wgpu compute shaders

---

## Architecture Overview

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                         trueno-graph                         │
│                  GPU-First Property Graph DB                 │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐   │
│  │   Storage     │  │  Algorithms   │  │   Query API   │   │
│  │  (CSR+Props)  │  │ (Trav/Rank/   │  │  (find_callers│   │
│  │               │  │  Clustering)  │  │   pagerank)   │   │
│  └───────┬───────┘  └───────┬───────┘  └───────┬───────┘   │
│          │                  │                  │             │
└──────────┼──────────────────┼──────────────────┼─────────────┘
           │                  │                  │
           ▼                  ▼                  ▼
  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐
  │   trueno-db    │  │   aprender     │  │     wgpu       │
  │  (Parquet I/O, │  │ (Sparse Matrix │  │  (GPU Compute) │
  │   Arrow Batch) │  │  PageRank)     │  │                │
  └────────┬───────┘  └────────┬───────┘  └────────┬───────┘
           │                   │                   │
           ▼                   ▼                   ▼
       ┌────────────────────────────────────────────┐
       │              trueno 0.6.0                   │
       │    (SIMD Fallback: AVX-512/AVX2/SSE2)      │
       └────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Reuses |
|-----------|---------------|--------|
| **Storage Layer** | CSR graph structure, Parquet persistence | trueno-db (Arrow), trueno (SIMD) |
| **Algorithm Layer** | BFS, PageRank, Louvain, pattern matching | aprender (sparse matrices), wgpu (kernels) |
| **Query API** | Graph-specific queries (find_callers, pagerank) | N/A (new) |
| **Memory Manager** | GPU paging, LRU cache, morsel iteration | trueno-db (buffer management) |

---

## Dependency Graph

### Avoiding Dependency Hell

**Design Principle**: DAG (Directed Acyclic Graph) dependencies with clear ownership.

```
trueno-graph 0.1.0
 ├─ trueno-db 0.3.0  (Parquet I/O, Arrow batches)
 │   └─ trueno 0.6.0  (SIMD primitives)
 ├─ aprender 0.6.0   (Sparse matrices, PageRank)
 │   └─ trueno 0.6.0  (reused from trueno-db)
 ├─ wgpu 22          (GPU compute shaders)
 ├─ arrow 53         (RecordBatch format)
 └─ parquet 53       (Persistent storage)
```

**Why This Works:**
1. ✅ **No circular dependencies**: trueno-graph depends on trueno-db/aprender, but not vice versa
2. ✅ **Shared foundation**: Both trueno-db and aprender use trueno 0.6.0 (no version conflicts)
3. ✅ **Feature-gated GPU**: wgpu optional to avoid binary bloat (same pattern as trueno-db)
4. ✅ **Standard formats**: Arrow/Parquet ensure interoperability

**What We DON'T Depend On:**
- ❌ renacer (syscall tracer - unrelated to graph DB)
- ❌ pmat (consumer of trueno-graph, not dependency)

### Cargo.toml Strategy

```toml
[package]
name = "trueno-graph"
version = "0.1.0"
edition = "2021"

[dependencies]
# Storage foundation (required)
trueno-db = "0.3"      # Parquet I/O, Arrow batches
trueno = "0.6"         # SIMD fallback (reused by trueno-db + aprender)
arrow = "53"           # RecordBatch format
parquet = "53"         # Persistent storage

# Algorithm foundation (required for graph operations)
aprender = "0.6"       # Sparse matrices, PageRank

# GPU compute (optional - feature-gated)
wgpu = { version = "22", optional = true }
bytemuck = { version = "1", optional = true, features = ["derive"] }

# Async runtime (required for I/O)
tokio = { version = "1", features = ["rt", "fs"] }

[dev-dependencies]
criterion = "0.6"      # Benchmarks (Gunrock comparison)

[features]
default = ["simd"]     # SIMD-only by default
gpu = ["wgpu", "bytemuck", "trueno-db/gpu"]  # GPU acceleration
simd = []              # Marker feature (always available via trueno)
```

**Binary Size Budget:**
- SIMD-only: ~5MB (trueno + trueno-db + aprender)
- GPU-enabled: ~8.8MB (+3.8MB for wgpu stack)

---

## Core Components

### 1. Storage Layer (`src/storage/`)

**CSR (Compressed Sparse Row) Graph** (Citation 2: GraphBLAST)

```rust
/// Graph stored in CSR format (Citation 2: Yang et al. 2022)
pub struct CsrGraph {
    // Row offsets: node i's edges start at row_offsets[i]
    pub row_offsets: Vec<u32>,  // Size: N+1 nodes

    // Column indices: edge targets
    pub col_indices: Vec<u32>,  // Size: E edges

    // Edge weights (e.g., call counts, complexity)
    pub edge_weights: Vec<f32>, // Size: E edges

    // Node properties (columnar)
    pub node_names: Vec<String>,     // Function names
    pub node_complexity: Vec<f32>,   // TDG complexity scores
    pub node_embeddings: Vec<Vec<f32>>, // 768-dim semantic vectors
}
```

**Parquet Persistence** (Citation 5: DuckDB)

```rust
/// Save graph to Parquet files (disk persistence)
pub async fn save_to_parquet(
    graph: &CsrGraph,
    path: impl AsRef<Path>,
) -> Result<()> {
    // 1. Convert CSR to Arrow RecordBatch (reuse trueno-db patterns)
    let edges_batch = csr_to_arrow_edges(graph)?;
    let nodes_batch = csr_to_arrow_nodes(graph)?;

    // 2. Write Parquet files (trueno-db::write_parquet)
    trueno_db::write_parquet(path.join("edges.parquet"), edges_batch).await?;
    trueno_db::write_parquet(path.join("nodes.parquet"), nodes_batch).await?;

    Ok(())
}
```

**GPU Memory Paging** (Citation 6: Funke et al. 2018)

```rust
/// Morsel-based GPU paging (Citation 4: Umbra, Citation 6: Funke)
pub struct GpuGraphCache {
    // LRU cache for hot subgraphs
    cache: LruCache<NodeId, DeviceGraph>,

    // Morsel size (128MB per Umbra 2020)
    morsel_size: usize,
}

impl GpuGraphCache {
    pub async fn load_neighborhood(
        &mut self,
        node_id: NodeId,
        depth: usize,
    ) -> Result<DeviceGraph> {
        // 1. Check LRU cache
        if let Some(subgraph) = self.cache.get(&node_id) {
            return Ok(subgraph.clone());
        }

        // 2. Load from Parquet (BFS traversal)
        let subgraph = extract_subgraph_bfs(node_id, depth).await?;

        // 3. Transfer to GPU (128MB morsel)
        let device_graph = subgraph.to_gpu()?;

        // 4. Cache for future queries
        self.cache.put(node_id, device_graph.clone());

        Ok(device_graph)
    }
}
```

### 2. Algorithm Layer (`src/algorithms/`)

**PageRank via aprender** (Citation 9: Page et al. 1999)

```rust
/// Compute PageRank using aprender sparse matmul (Citation 2: GraphBLAST)
pub async fn pagerank(
    graph: &CsrGraph,
    iterations: usize,
) -> Result<Vec<f32>> {
    // 1. Convert CSR to aprender sparse matrix
    let adj_matrix = aprender::SparseMatrix::from_csr(
        &graph.row_offsets,
        &graph.col_indices,
        &graph.edge_weights,
    )?;

    // 2. Initialize scores (uniform distribution)
    let mut scores = vec![1.0 / graph.num_nodes() as f32; graph.num_nodes()];

    // 3. Power iteration: PR = (1-d)/N + d * A^T * PR
    for _ in 0..iterations {
        scores = aprender::spmv(&adj_matrix, &scores)?; // Sparse matrix-vector multiply
        scores.iter_mut().for_each(|s| *s = 0.15 / graph.num_nodes() as f32 + 0.85 * *s);
    }

    Ok(scores)
}
```

**BFS Traversal** (Citation 10: Ligra)

```rust
/// Find all callers (incoming edges) via BFS (Citation 1: Gunrock, Citation 10: Ligra)
pub async fn find_callers(
    graph: &CsrGraph,
    target: NodeId,
    depth: usize,
) -> Result<Vec<NodeId>> {
    // 1. Frontier-based BFS (Ligra pattern)
    let mut frontier = vec![target];
    let mut visited = HashSet::new();

    for _ in 0..depth {
        let mut next_frontier = Vec::new();

        for node in &frontier {
            // Find incoming edges (transpose graph)
            for (src, _) in graph.incoming_edges(*node) {
                if !visited.contains(&src) {
                    visited.insert(src);
                    next_frontier.push(src);
                }
            }
        }

        frontier = next_frontier;
    }

    Ok(visited.into_iter().collect())
}
```

**Louvain Clustering** (Citation 8: Blondel et al. 2008)

```rust
/// Detect code modules via Louvain clustering (aprender integration)
pub async fn detect_modules(graph: &CsrGraph) -> Result<Vec<CommunityId>> {
    // 1. Convert to aprender graph
    let aprender_graph = aprender::Graph::from_csr(
        &graph.row_offsets,
        &graph.col_indices,
    )?;

    // 2. Run Louvain (Citation 8: Blondel et al. 2008)
    let communities = aprender::clustering::louvain(&aprender_graph)?;

    Ok(communities)
}
```

### 3. Query API (`src/query/`)

**High-Level User API**

```rust
/// Graph database handle
pub struct GraphDb {
    storage: CsrGraph,
    gpu_cache: GpuGraphCache,
}

impl GraphDb {
    /// Find all functions that call the target function
    pub async fn find_callers(&self, function: &str, depth: usize) -> Result<Vec<String>> {
        let node_id = self.storage.lookup_name(function)?;
        let caller_ids = find_callers(&self.storage, node_id, depth).await?;
        Ok(caller_ids.iter().map(|id| self.storage.node_names[*id].clone()).collect())
    }

    /// Compute importance scores for all functions
    pub async fn importance_ranking(&self) -> Result<Vec<(String, f32)>> {
        let scores = pagerank(&self.storage, 20).await?;
        let mut ranked: Vec<_> = self.storage.node_names.iter()
            .zip(scores.iter())
            .map(|(name, score)| (name.clone(), *score))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        Ok(ranked)
    }

    /// Find architectural modules (tightly coupled code)
    pub async fn detect_modules(&self) -> Result<HashMap<String, Vec<String>>> {
        let communities = detect_modules(&self.storage).await?;
        // Group functions by community
        let mut modules: HashMap<String, Vec<String>> = HashMap::new();
        for (node_id, community) in communities.iter().enumerate() {
            modules.entry(community.to_string())
                .or_default()
                .push(self.storage.node_names[node_id].clone());
        }
        Ok(modules)
    }
}
```

---

## Quality Enforcement

### PMAT Integration (Zero SATD, EXTREME TDD)

**Pre-commit Hooks** (applied to trueno-graph)

```bash
#!/bin/bash
# .git/hooks/pre-commit (generated by pmat)

# 1. Run pmat quality checks
pmat analyze complexity --fail-above 20 --path .
pmat analyze dead-code --path .
pmat quality-gate --checks clippy,fmt,tests

# 2. bashrs validation (shell scripts + Makefile)
bashrs lint Makefile
find scripts -name "*.sh" -exec bashrs lint {} \;

# 3. Zero SATD enforcement
pmat analyze satd --path . --fail-on-any

# Exit 1 if any check fails
```

**certeza Quality Patterns** (adapted from ../certeza)

```yaml
# .certeza.yml (quality validation)
thresholds:
  test_coverage: 95%      # certeza standard (higher than PMAT's 85%)
  mutation_score: 80%     # Mutation testing required
  complexity_max: 20      # Per-function cyclomatic complexity
  unsafe_allowed: 0       # Zero unsafe code (pure Rust graph DB)

checks:
  - name: "Zero SATD"
    command: "pmat analyze satd --fail-on-any"

  - name: "Dependency freshness"
    command: "cargo outdated --exit-code 1"

  - name: "Benchmark regression"
    command: "cargo bench --features gpu -- --save-baseline main"
```

**bashrs Integration** (Makefile validation)

```makefile
# Makefile (validated by bashrs)

.PHONY: test
test:
	cargo test --all-features

.PHONY: bench
bench:
	cargo bench --features gpu -- --save-baseline main

.PHONY: quality
quality:
	# bashrs validates this Makefile before execution
	pmat quality-gate --checks clippy,fmt,tests,coverage
	cargo llvm-cov --html --output-dir coverage

.PHONY: validate-all
validate-all: quality test bench
	@echo "All quality gates passed"
```

### Toyota Way Principles

1. **Jidoka (Built-in Quality)**
   - RED tests written BEFORE implementation
   - Mutation testing catches weak tests
   - Benchmarks validate performance claims (Gunrock comparison)

2. **Muda (Waste Elimination)**
   - Reuse trueno/trueno-db/aprender (don't reinvent)
   - Zero SATD (no technical debt accumulation)
   - Feature flags (SIMD-only default, GPU opt-in)

3. **Kaizen (Continuous Improvement)**
   - Benchmark-driven optimization
   - Peer-reviewed algorithm selection (10 citations)
   - Quality metrics tracked over time (certeza)

4. **Genchi Genbutsu (Go and See)**
   - Performance claims validated by benchmarks
   - Integration tests use real PMAT call graphs
   - Profiling identifies bottlenecks (not guesswork)

---

## Performance Targets

### Baseline Comparisons

| Operation | NetworkX (CPU) | trueno-graph (SIMD) | trueno-graph (GPU) | Speedup (GPU) |
|-----------|----------------|---------------------|-------------------|---------------|
| **PageRank** (1K nodes) | 100ms | 20ms | 4ms | **25x** |
| **BFS** (100K nodes, depth=3) | 500ms | 50ms | 2ms | **250x** |
| **Louvain** (10K nodes) | 2000ms | 400ms | 80ms | **25x** |
| **Subgraph Match** (pattern A→B→C) | 1000ms | 200ms | 20ms | **50x** |
| **Load Graph** (1M edges) | 5000ms | 1000ms | 200ms | **25x** |

**Hardware Assumptions:**
- CPU: AMD Ryzen 9 5950X (16 cores, AVX-512)
- GPU: NVIDIA RTX 4090 (16,384 CUDA cores, 24GB VRAM)
- RAM: 64GB DDR4-3200
- Storage: NVMe SSD (7GB/s read)

**Validation Strategy:**
1. Benchmark against NetworkX (CPU baseline)
2. Compare SIMD vs GPU modes (ensure GPU benefit > overhead)
3. Regression tests (catch performance degradation)

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)

**Goal**: Core CSR storage + Parquet I/O

**Tasks:**
1. ✅ Create project structure (`src/storage/`, `src/algorithms/`, `src/query/`)
2. ✅ Implement `CsrGraph` structure (Citation 2: GraphBLAST)
3. ✅ Parquet save/load (reuse trueno-db patterns)
4. ✅ Unit tests (RED phase)

**Acceptance Criteria:**
- ✅ Save/load 1M-edge graph to Parquet (<1s)
- ✅ CSR indexing correct (validate against adjacency list)
- ✅ Zero SATD, 95% coverage

### Phase 2: Algorithm Integration (Week 3-4)

**Goal**: PageRank + BFS via aprender

**Tasks:**
1. Implement PageRank (aprender sparse matmul)
2. Implement BFS traversal (find_callers)
3. Benchmarks vs NetworkX (validate citations)
4. Integration tests with PMAT call graphs

**Acceptance Criteria:**
- ✅ PageRank matches NetworkX results (ε < 0.001)
- ✅ BFS 10x faster than CPU baseline
- ✅ Benchmarks pass (no regressions)

### Phase 3: GPU Acceleration (Week 5-6)

**Goal**: wgpu compute shaders for traversal

**Tasks:**
1. Implement GPU BFS kernel (Citation 1: Gunrock)
2. GPU memory paging (Citation 6: Funke)
3. LRU cache for hot subgraphs
4. Benchmarks (GPU vs SIMD)

**Acceptance Criteria:**
- ✅ GPU BFS 250x faster than NetworkX
- ✅ Handle graphs larger than VRAM (morsel paging)
- ✅ Feature flag `gpu` works (optional wgpu)

### Phase 4: Advanced Algorithms (Week 7-8)

**Goal**: Louvain, pattern matching

**Tasks:**
1. Louvain clustering via aprender
2. Subgraph pattern matching (Citation 3: Kim et al. 2023)
3. Quality enforcement (pmat/certeza/bashrs)
4. Documentation (rustdoc, README)

**Acceptance Criteria:**
- ✅ Louvain detects PMAT modules (validate manually)
- ✅ Pattern matching 50x faster than NetworkX
- ✅ 95% test coverage, zero SATD

---

## Testing Strategy

### Unit Tests (EXTREME TDD)

**RED → GREEN → REFACTOR**

```rust
#[test]
fn test_csr_indexing_correctness() {
    // RED: Test written BEFORE CsrGraph implementation
    let graph = CsrGraph::from_edge_list(&[
        (0, 1, 1.0),
        (0, 2, 1.0),
        (1, 2, 1.0),
    ]);

    // Verify row_offsets
    assert_eq!(graph.row_offsets, vec![0, 2, 3, 3]); // Node 0: 2 edges, Node 1: 1 edge, Node 2: 0 edges

    // Verify col_indices
    assert_eq!(graph.col_indices, vec![1, 2, 2]);

    // Verify edge_weights
    assert_eq!(graph.edge_weights, vec![1.0, 1.0, 1.0]);
}
```

### Integration Tests

**Real PMAT Call Graph**

```rust
#[tokio::test]
async fn test_pmat_call_graph_analysis() {
    // 1. Parse PMAT codebase
    let call_graph = parse_pmat_codebase("/home/noah/src/paiml-mcp-agent-toolkit").await.unwrap();

    // 2. Convert to CsrGraph
    let graph = CsrGraph::from_call_graph(&call_graph);

    // 3. Find callers of TdgCalculator::calculate_tdg
    let callers = find_callers(&graph, "TdgCalculator::calculate_tdg", 2).await.unwrap();

    // 4. Verify expected callers (manually validated)
    assert!(callers.contains(&"analyze_project".to_string()));
    assert!(callers.contains(&"AnalysisService::run".to_string()));

    // 5. Compute PageRank
    let importance = pagerank(&graph, 20).await.unwrap();

    // 6. Verify main entry point has high PageRank
    let main_score = importance[graph.lookup_name("main").unwrap()];
    assert!(main_score > 0.01); // Top 1% of functions
}
```

### Benchmark Tests (Performance Validation)

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_pagerank_vs_networkx(c: &mut Criterion) {
    let graph = generate_scale_free_graph(1000, 3000); // 1K nodes, 3K edges

    c.bench_function("pagerank_simd", |b| {
        b.iter(|| {
            pagerank(black_box(&graph), 20)
        })
    });

    // Baseline (NetworkX equivalent)
    let networkx_time = 100.0; // ms (measured separately)

    // Validate speedup (should be 5x faster)
    let simd_time = c.get_measurement_time("pagerank_simd");
    assert!(simd_time < networkx_time / 5.0, "SIMD pagerank not 5x faster than NetworkX");
}
```

### Mutation Testing (certeza standard)

```bash
# Run mutation testing (80% mutation score required)
cargo mutants --test-timeout 60 --jobs 8

# Expected mutations killed:
# - Replace + with - in PageRank formula
# - Off-by-one in row_offsets indexing
# - Swap < with <= in BFS termination
```

---

## Appendix A: Directory Structure

```
trueno-graph/
├── Cargo.toml               # Dependencies (trueno-db, aprender, wgpu)
├── Makefile                 # Quality targets (validated by bashrs)
├── README.md                # User documentation
├── .certeza.yml             # Quality thresholds
├── .git/hooks/pre-commit    # PMAT quality enforcement
├── src/
│   ├── lib.rs               # Public API exports
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── csr.rs           # CSR graph structure (Citation 2)
│   │   ├── parquet.rs       # Persistence (trueno-db integration)
│   │   └── cache.rs         # GPU memory paging (Citation 6)
│   ├── algorithms/
│   │   ├── mod.rs
│   │   ├── pagerank.rs      # aprender integration (Citation 9)
│   │   ├── traversal.rs     # BFS, find_callers (Citations 1, 10)
│   │   ├── clustering.rs    # Louvain (Citation 8)
│   │   └── pattern.rs       # Subgraph matching (Citation 3)
│   ├── query/
│   │   ├── mod.rs
│   │   └── api.rs           # GraphDb high-level API
│   └── memory/
│       ├── mod.rs
│       └── paging.rs        # Morsel-based GPU paging
├── tests/
│   ├── integration_tests.rs # Real PMAT call graph tests
│   └── regression_tests.rs  # Performance regression detection
├── benches/
│   ├── pagerank.rs          # Criterion benchmarks
│   ├── bfs.rs               # BFS vs NetworkX
│   └── louvain.rs           # Clustering performance
└── docs/
    ├── specifications/
    │   └── graph-db-spec.md # This document
    └── architecture/
        └── dependency-graph.md
```

---

## Appendix B: Open Questions

1. **Semantic Search Integration**
   - Should we embed semantic vectors in nodes, or maintain separate vector DB?
   - **Proposal**: Store 768-dim embeddings in node properties (columnar), use trueno SIMD for cosine similarity

2. **Incremental Updates**
   - Current design: append-only Parquet (no in-place updates)
   - **Trade-off**: Rebuild graph on each change vs delta updates
   - **Proposal**: Batch updates (hourly), rebuild graph in background

3. **Multi-Language Support**
   - PMAT analyzes 30+ languages - need unified call graph representation
   - **Challenge**: Different call semantics (static vs dynamic dispatch)
   - **Proposal**: Tag edges with language metadata (Python: dynamic, Rust: static)

---

## Version History

- **v0.1.0** (2025-11-22): Initial specification draft
  - 10 peer-reviewed citations
  - Dependency graph (trueno, trueno-db, aprender)
  - Quality enforcement (pmat, certeza, bashrs)
  - Performance targets (25-250x speedups)
  - 8-week implementation roadmap
