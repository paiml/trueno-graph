// GPU PageRank - Sparse Matrix-Vector Multiplication (SpMV)
//
// Algorithm:
// 1. Initialize all scores to 1/N
// 2. For each iteration:
//    - new_pr[v] = (1-d)/N + d * Σ(pr[u] / out_degree[u]) for all u -> v
// 3. Repeat until convergence (20 iterations typical)
//
// Based on Page et al. (1999) and GraphBLAST (Yang et al., ACM ToMS 2022)

struct PageRankParams {
    num_nodes: u32,
    damping: f32,        // Typically 0.85
    iteration: u32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> params: PageRankParams;
@group(0) @binding(1) var<storage, read> row_offsets: array<u32>;
@group(0) @binding(2) var<storage, read> col_indices: array<u32>;
@group(0) @binding(3) var<storage, read> current_scores: array<f32>;
@group(0) @binding(4) var<storage, read_write> next_scores: array<f32>;
@group(0) @binding(5) var<storage, read> out_degrees: array<u32>;

// Process one node per thread
@compute @workgroup_size(256)
fn pagerank_iteration(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node_id = global_id.x;

    // Bounds check
    if (node_id >= params.num_nodes) {
        return;
    }

    // Base score (teleport probability)
    let base_score = (1.0 - params.damping) / f32(params.num_nodes);

    // Accumulate contributions from incoming neighbors
    // For each u where u -> node_id:
    //   contribution += current_scores[u] / out_degrees[u]
    var contribution_sum = 0.0;

    // Note: We need reverse CSR for efficient incoming neighbor access
    // For now, we'll iterate over all nodes and check if they point to node_id
    // TODO: Optimize with reverse CSR (pre-computed in Rust)
    for (var u = 0u; u < params.num_nodes; u++) {
        let start = row_offsets[u];
        let end = row_offsets[u + 1u];

        // Check if u has edge to node_id
        for (var i = start; i < end; i++) {
            if (col_indices[i] == node_id) {
                // u -> node_id edge found
                let out_degree = f32(out_degrees[u]);
                if (out_degree > 0.0) {
                    contribution_sum += current_scores[u] / out_degree;
                }
                break; // Found edge, no need to check more
            }
        }
    }

    // Update score: (1-d)/N + d * Σ(PR(u) / out_degree(u))
    next_scores[node_id] = base_score + params.damping * contribution_sum;
}
