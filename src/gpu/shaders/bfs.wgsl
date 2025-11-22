// GPU BFS Compute Shader
// Based on Gunrock (Wang et al., ACM ToPC 2017) frontier-based traversal
//
// Algorithm:
// 1. Start with source node in frontier
// 2. For each vertex in frontier:
//    - Mark all unvisited neighbors as visited
//    - Add neighbors to next frontier
// 3. Swap frontiers and repeat until empty

struct GraphData {
    num_nodes: u32,
    num_edges: u32,
}

@group(0) @binding(0) var<storage, read> graph_meta: GraphData;
@group(0) @binding(1) var<storage, read> row_offsets: array<u32>;
@group(0) @binding(2) var<storage, read> col_indices: array<u32>;
@group(0) @binding(3) var<storage, read_write> visited: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> distances: array<u32>;
@group(0) @binding(5) var<storage, read> frontier: array<u32>;
@group(0) @binding(6) var<storage, read_write> next_frontier: array<u32>;
@group(0) @binding(7) var<storage, read_write> next_frontier_size: atomic<u32>;

// BFS kernel - process one frontier vertex per thread
@compute @workgroup_size(256)
fn bfs_kernel(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let tid = global_id.x;
    let frontier_size = arrayLength(&frontier);

    // Bounds check
    if (tid >= frontier_size) {
        return;
    }

    // Get current vertex from frontier
    let vertex = frontier[tid];
    if (vertex >= graph_meta.num_nodes) {
        return;
    }

    // Get current distance
    let current_dist = distances[vertex];

    // Process neighbors
    let start = row_offsets[vertex];
    let end = row_offsets[vertex + 1u];

    for (var i = start; i < end; i++) {
        let neighbor = col_indices[i];

        // Try to mark neighbor as visited (atomic)
        // Returns 0 if not visited before, 1 if already visited
        let was_visited = atomicExchange(&visited[neighbor], 1u);

        if (was_visited == 0u) {
            // First time visiting this neighbor
            distances[neighbor] = current_dist + 1u;

            // Add to next frontier (atomic counter)
            let next_idx = atomicAdd(&next_frontier_size, 1u);
            next_frontier[next_idx] = neighbor;
        }
    }
}
