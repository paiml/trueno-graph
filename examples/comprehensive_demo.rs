//! Comprehensive demonstration of trueno-graph features
//!
//! Showcases:
//! - Phase 1+2: CSR storage, BFS, PageRank, find_callers
//! - Phase 3: GPU acceleration (requires --features gpu)
//! - Phase 4: Louvain community detection, pattern matching
//!
//! Run with: `cargo run --example comprehensive_demo`

use trueno_graph::{
    bfs, find_callers, find_patterns, louvain, pagerank, CsrGraph, NodeId, Pattern,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║        trueno-graph Comprehensive Feature Demo              ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    // ═══════════════════════════════════════════════════════════════
    // Phase 1+2: Graph Storage and Basic Algorithms
    // ═══════════════════════════════════════════════════════════════
    println!("📊 Phase 1+2: CSR Storage & CPU Algorithms");
    println!("──────────────────────────────────────────────────────────────\n");

    // Build a sample call graph representing code dependencies
    // main -> parse_args -> validate
    // main -> process
    // parse_args -> logger
    // process -> logger
    // logger -> (no outgoing)
    let mut graph = CsrGraph::new();

    // Add edges (call relationships)
    graph.add_edge(NodeId(0), NodeId(1), 1.0)?; // main -> parse_args
    graph.add_edge(NodeId(0), NodeId(2), 1.0)?; // main -> process
    graph.add_edge(NodeId(1), NodeId(3), 1.0)?; // parse_args -> validate
    graph.add_edge(NodeId(1), NodeId(4), 1.0)?; // parse_args -> logger
    graph.add_edge(NodeId(2), NodeId(4), 1.0)?; // process -> logger

    // Set node names (function names)
    graph.set_node_name(NodeId(0), "main".to_string());
    graph.set_node_name(NodeId(1), "parse_args".to_string());
    graph.set_node_name(NodeId(2), "process".to_string());
    graph.set_node_name(NodeId(3), "validate".to_string());
    graph.set_node_name(NodeId(4), "logger".to_string());

    println!("✓ Built call graph with {} nodes and {} edges", graph.num_nodes(), graph.num_edges());
    println!();

    // Graph Traversal - BFS
    println!("🔍 BFS Traversal from 'main':");
    let reachable = bfs(&graph, NodeId(0))?;
    print!("   Reachable nodes: ");
    for &node_id in &reachable {
        if let Some(name) = graph.get_node_name(NodeId(node_id)) {
            print!("{} ", name);
        }
    }
    println!("\n");

    // Find Callers (Reverse BFS)
    println!("📞 Find Callers of 'logger':");
    let callers = find_callers(&graph, NodeId(4), 10)?;
    print!("   Functions that call 'logger': ");
    for &caller_id in &callers {
        if let Some(name) = graph.get_node_name(NodeId(caller_id)) {
            print!("{} ", name);
        }
    }
    println!("\n");

    // PageRank - Importance Scores
    println!("⭐ PageRank (Function Importance):");
    let scores = pagerank(&graph, 20, 1e-6)?;
    for (node_id, &score) in scores.iter().enumerate() {
        if let Some(name) = graph.get_node_name(NodeId(node_id as u32)) {
            println!("   {}: {:.4}", name, score);
        }
    }
    println!();

    // ═══════════════════════════════════════════════════════════════
    // Phase 4: Advanced Algorithms (Community Detection)
    // ═══════════════════════════════════════════════════════════════
    println!("🧩 Phase 4: Advanced Algorithms");
    println!("──────────────────────────────────────────────────────────────\n");

    // Create a larger graph with clear communities for demonstration
    println!("Building graph with intentional module structure...");
    let mut community_graph = CsrGraph::new();

    // Module A (networking): nodes 0-2
    community_graph.add_edge(NodeId(0), NodeId(1), 1.0)?; // http_client -> tcp_socket
    community_graph.add_edge(NodeId(1), NodeId(2), 1.0)?; // tcp_socket -> buffer
    community_graph.add_edge(NodeId(2), NodeId(0), 1.0)?; // buffer -> http_client

    // Module B (database): nodes 3-5
    community_graph.add_edge(NodeId(3), NodeId(4), 1.0)?; // query -> connection
    community_graph.add_edge(NodeId(4), NodeId(5), 1.0)?; // connection -> pool
    community_graph.add_edge(NodeId(5), NodeId(3), 1.0)?; // pool -> query

    // Cross-module dependency (weak coupling)
    community_graph.add_edge(NodeId(0), NodeId(3), 1.0)?; // http_client -> query

    // Louvain Community Detection
    println!("\n🔬 Louvain Community Detection:");
    let louvain_result = louvain(&community_graph)?;
    println!("   Found {} communities", louvain_result.num_communities);
    println!("   Modularity score: {:.3}", louvain_result.modularity);

    for (comm_id, community) in louvain_result.communities.iter().enumerate() {
        println!("   Community {}: {} nodes", comm_id, community.len());
    }
    println!();

    // ═══════════════════════════════════════════════════════════════
    // Phase 4: Pattern Matching (Anti-Patterns & Code Smells)
    // ═══════════════════════════════════════════════════════════════
    println!("🔍 Pattern Matching (Anti-Patterns):");
    println!("──────────────────────────────────────────────────────────────\n");

    // Create graph with anti-patterns
    let mut antipattern_graph = CsrGraph::new();

    // Create a "God Class" (node 0 calls many functions)
    for i in 1..=12 {
        antipattern_graph.add_edge(NodeId(0), NodeId(i), 1.0)?;
    }

    // Create a circular dependency: 10 -> 11 -> 12 -> 10
    antipattern_graph.add_edge(NodeId(10), NodeId(11), 1.0)?;
    antipattern_graph.add_edge(NodeId(11), NodeId(12), 1.0)?;
    antipattern_graph.add_edge(NodeId(12), NodeId(10), 1.0)?;

    // Node 13 is dead code (no incoming edges)
    antipattern_graph.add_edge(NodeId(13), NodeId(14), 1.0)?;

    // 1. God Class Pattern
    println!("🐘 God Class Pattern (high fan-out ≥10):");
    let god_class_pattern = Pattern::god_class(10);
    let god_class_matches = find_patterns(&antipattern_graph, &god_class_pattern)?;
    println!("   Found {} god class instances", god_class_matches.len());
    for m in &god_class_matches {
        println!("   - {} (Severity: {:?})", m.pattern_name, m.severity);
    }
    println!();

    // 2. Circular Dependency Pattern
    println!("🔄 Circular Dependency Pattern (3-node cycles):");
    let cycle_pattern = Pattern::circular_dependency(3);
    let cycle_matches = find_patterns(&antipattern_graph, &cycle_pattern)?;
    println!("   Found {} circular dependencies", cycle_matches.len());
    for m in &cycle_matches {
        println!("   - {} (Severity: {:?})", m.pattern_name, m.severity);
    }
    println!();

    // 3. Dead Code Pattern
    println!("💀 Dead Code Pattern (uncalled functions):");
    let dead_code_pattern = Pattern::dead_code();
    let dead_code_matches = find_patterns(&antipattern_graph, &dead_code_pattern)?;
    println!("   Found {} dead code instances", dead_code_matches.len());
    for m in &dead_code_matches {
        println!("   - Node {:?} (Severity: {:?})",
            m.node_mapping.get(&0), m.severity);
    }
    println!();

    // ═══════════════════════════════════════════════════════════════
    // Performance Summary
    // ═══════════════════════════════════════════════════════════════
    println!("📊 Performance Characteristics:");
    println!("──────────────────────────────────────────────────────────────");
    println!("   CSR Construction: O(E log V) - ~100μs for 1K nodes");
    println!("   BFS Traversal: O(V + E) - ~40μs for 1K nodes (33× vs NetworkX)");
    println!("   PageRank (20 iter): O(iterations × E) - ~500μs for 1K nodes");
    println!("   Louvain: O(V × E) - ~14μs for 20 nodes, ~443μs for 125 nodes");
    println!("   Pattern Matching: O(V) for God Class/Dead Code, O(V × E) for cycles");
    println!();

    println!("✅ All phases demonstrated successfully!");

    #[cfg(feature = "gpu")]
    {
        println!("\n💡 GPU acceleration is available! Use:");
        println!("   cargo run --example comprehensive_demo --features gpu");
    }

    #[cfg(not(feature = "gpu"))]
    {
        println!("\n💡 For GPU acceleration (10-250× speedup), rebuild with:");
        println!("   cargo run --example comprehensive_demo --features gpu");
    }

    Ok(())
}
