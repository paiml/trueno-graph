//! Subgraph pattern matching for anti-pattern detection
//!
//! Find specific graph patterns (e.g., code smells, anti-patterns).
//! Based on subgraph isomorphism and motif detection algorithms.
//!
//! # References
//! - Kim et al. (2023): "Efficient Subgraph Matching on Large Graphs"
//! - Cordella et al. (2004): "VF2 algorithm for subgraph isomorphism"

use crate::{CsrGraph, NodeId};
use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};

/// Pattern matching result
#[derive(Debug, Clone)]
pub struct PatternMatch {
    /// Nodes involved in the pattern (pattern node → graph node mapping)
    pub node_mapping: HashMap<u32, NodeId>,

    /// Pattern name (e.g., "God Class", "Circular Dependency")
    pub pattern_name: String,

    /// Severity: Low, Medium, High, Critical
    pub severity: Severity,
}

/// Severity level for anti-patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Low severity (minor code smell)
    Low,
    /// Medium severity (should be refactored)
    Medium,
    /// High severity (significant anti-pattern)
    High,
    /// Critical severity (architectural flaw)
    Critical,
}

/// Pattern specification for matching
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Pattern name
    pub name: String,

    /// Pattern edges (source, target) - node IDs are local to the pattern
    pub edges: Vec<(u32, u32)>,

    /// Minimum number of nodes in the pattern
    pub min_nodes: usize,

    /// Maximum number of nodes (None = unbounded)
    pub max_nodes: Option<usize>,

    /// Severity if found
    pub severity: Severity,
}

impl Pattern {
    /// Create a new pattern
    #[must_use]
    pub fn new(name: String, edges: Vec<(u32, u32)>, severity: Severity) -> Self {
        let num_nodes = edges
            .iter()
            .flat_map(|(s, t)| [*s, *t])
            .max()
            .map_or(0, |max| max as usize + 1);

        Self {
            name,
            edges,
            min_nodes: num_nodes,
            max_nodes: None,
            severity,
        }
    }

    /// Create a "God Class" pattern (node with many outgoing edges)
    ///
    /// Detects functions that call too many other functions (high fan-out)
    #[must_use]
    pub fn god_class(min_callees: usize) -> Self {
        Self {
            name: "God Class".to_string(),
            edges: Vec::new(), // Filled dynamically based on min_callees
            min_nodes: min_callees + 1,
            max_nodes: None,
            severity: Severity::High,
        }
    }

    /// Create a "Circular Dependency" pattern (cycle of length N)
    ///
    /// Detects function call cycles
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Pattern length >4B nodes not realistic
    pub fn circular_dependency(cycle_length: usize) -> Self {
        let mut edges = Vec::new();
        for i in 0..cycle_length {
            let next = (i + 1) % cycle_length;
            edges.push((i as u32, next as u32));
        }

        Self {
            name: "Circular Dependency".to_string(),
            edges,
            min_nodes: cycle_length,
            max_nodes: Some(cycle_length),
            severity: Severity::Critical,
        }
    }

    /// Create a "Dead Code" pattern (node with no incoming edges)
    ///
    /// Detects uncalled functions
    #[must_use]
    pub fn dead_code() -> Self {
        Self {
            name: "Dead Code".to_string(),
            edges: Vec::new(),
            min_nodes: 1,
            max_nodes: Some(1),
            severity: Severity::Medium,
        }
    }
}

/// Find all pattern matches in the graph
///
/// # Arguments
///
/// * `graph` - The graph to search
/// * `pattern` - The pattern to find
///
/// # Returns
///
/// Vector of pattern matches
///
/// # Errors
///
/// Returns error if pattern is malformed
///
/// # Example
///
/// ```ignore
/// # use trueno_graph::{CsrGraph, NodeId};
/// # use trueno_graph::algorithms::pattern::{Pattern, find_patterns, Severity};
/// let mut graph = CsrGraph::new();
/// // Create a 3-node cycle: 0 -> 1 -> 2 -> 0
/// graph.add_edge(NodeId(0), NodeId(1), 1.0).unwrap();
/// graph.add_edge(NodeId(1), NodeId(2), 1.0).unwrap();
/// graph.add_edge(NodeId(2), NodeId(0), 1.0).unwrap();
///
/// // Find circular dependencies of length 3
/// let pattern = Pattern::circular_dependency(3);
/// let matches = find_patterns(&graph, &pattern).unwrap();
///
/// assert_eq!(matches.len(), 1);
/// ```
pub fn find_patterns(graph: &CsrGraph, pattern: &Pattern) -> Result<Vec<PatternMatch>> {
    match pattern.name.as_str() {
        "God Class" => Ok(find_god_class_patterns(graph, pattern)),
        "Circular Dependency" => Ok(find_cycle_patterns(graph, pattern)),
        "Dead Code" => Ok(find_dead_code_patterns(graph, pattern)),
        _ => find_generic_patterns(graph, pattern),
    }
}

/// Find "God Class" patterns (high fan-out nodes)
fn find_god_class_patterns(graph: &CsrGraph, pattern: &Pattern) -> Vec<PatternMatch> {
    let mut matches = Vec::new();

    for node_id in 0..graph.num_nodes() {
        #[allow(clippy::cast_possible_truncation)]
        let node = NodeId(node_id as u32);

        if let Ok(neighbors) = graph.outgoing_neighbors(node) {
            if neighbors.len() >= pattern.min_nodes - 1 {
                // High fan-out detected
                let mut node_mapping = HashMap::new();
                node_mapping.insert(0, node); // Pattern node 0 = god class

                matches.push(PatternMatch {
                    node_mapping,
                    pattern_name: pattern.name.clone(),
                    severity: pattern.severity,
                });
            }
        }
    }

    matches
}

/// Find "Circular Dependency" patterns (cycles)
fn find_cycle_patterns(graph: &CsrGraph, pattern: &Pattern) -> Vec<PatternMatch> {
    let cycle_length = pattern.min_nodes;
    let mut matches = Vec::new();
    let mut visited_cycles = HashSet::new();

    // DFS to find cycles of specific length
    for start_node in 0..graph.num_nodes() {
        #[allow(clippy::cast_possible_truncation)]
        let start = NodeId(start_node as u32);

        let cycles = find_cycles_from_node(graph, start, cycle_length);

        for cycle in cycles {
            // Normalize cycle to avoid duplicates (start from smallest node)
            let mut normalized = cycle.clone();
            normalized.sort_unstable();

            if visited_cycles.insert(normalized) {
                // Create node mapping
                let mut node_mapping = HashMap::new();
                for (pattern_id, &graph_node) in cycle.iter().enumerate() {
                    #[allow(clippy::cast_possible_truncation)]
                    node_mapping.insert(pattern_id as u32, graph_node);
                }

                matches.push(PatternMatch {
                    node_mapping,
                    pattern_name: pattern.name.clone(),
                    severity: pattern.severity,
                });
            }
        }
    }

    matches
}

/// Find "Dead Code" patterns (nodes with no incoming edges)
fn find_dead_code_patterns(graph: &CsrGraph, pattern: &Pattern) -> Vec<PatternMatch> {
    let mut matches = Vec::new();

    for node_id in 0..graph.num_nodes() {
        #[allow(clippy::cast_possible_truncation)]
        let node = NodeId(node_id as u32);

        if let Ok(incoming) = graph.incoming_neighbors(node) {
            if incoming.is_empty() {
                // No callers = dead code
                let mut node_mapping = HashMap::new();
                node_mapping.insert(0, node);

                matches.push(PatternMatch {
                    node_mapping,
                    pattern_name: pattern.name.clone(),
                    severity: pattern.severity,
                });
            }
        }
    }

    matches
}

/// Find generic patterns using VF2-like subgraph isomorphism
fn find_generic_patterns(_graph: &CsrGraph, pattern: &Pattern) -> Result<Vec<PatternMatch>> {
    // Simplified VF2 implementation (stub for now)
    // Full VF2 implementation would be quite complex
    Err(anyhow!(
        "Generic pattern matching not yet implemented for pattern '{}'",
        pattern.name
    ))
}

/// Helper: Find cycles of specific length starting from a node
fn find_cycles_from_node(
    graph: &CsrGraph,
    start: NodeId,
    target_length: usize,
) -> Vec<Vec<NodeId>> {
    let mut cycles = Vec::new();
    let mut path = vec![start];
    let mut visited = HashSet::new();
    visited.insert(start.0);

    dfs_find_cycles(
        graph,
        start,
        start,
        &mut path,
        &mut visited,
        target_length,
        &mut cycles,
    );

    cycles
}

/// DFS helper for cycle detection
#[allow(clippy::too_many_arguments)]
fn dfs_find_cycles(
    graph: &CsrGraph,
    current: NodeId,
    start: NodeId,
    path: &mut Vec<NodeId>,
    visited: &mut HashSet<u32>,
    target_length: usize,
    cycles: &mut Vec<Vec<NodeId>>,
) {
    if path.len() == target_length {
        // Check if current node connects back to start
        if let Ok(neighbors) = graph.outgoing_neighbors(current) {
            if neighbors.contains(&start.0) {
                cycles.push(path.clone());
            }
        }
        return;
    }

    // Explore neighbors
    if let Ok(neighbors) = graph.outgoing_neighbors(current) {
        for &neighbor_id in neighbors {
            let neighbor = NodeId(neighbor_id);

            if !visited.contains(&neighbor_id)
                || (neighbor == start && path.len() == target_length - 1)
            {
                visited.insert(neighbor_id);
                path.push(neighbor);

                dfs_find_cycles(graph, neighbor, start, path, visited, target_length, cycles);

                path.pop();
                visited.remove(&neighbor_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_god_class_pattern() {
        // Node 0 calls 5 other functions (high fan-out)
        let mut graph = CsrGraph::new();
        for i in 1..=5 {
            graph.add_edge(NodeId(0), NodeId(i), 1.0).unwrap();
        }

        let pattern = Pattern::god_class(5);
        let matches = find_patterns(&graph, &pattern).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern_name, "God Class");
        assert_eq!(matches[0].severity, Severity::High);
    }

    #[test]
    fn test_circular_dependency_pattern() {
        // Create 3-node cycle: 0 -> 1 -> 2 -> 0
        let mut graph = CsrGraph::new();
        graph.add_edge(NodeId(0), NodeId(1), 1.0).unwrap();
        graph.add_edge(NodeId(1), NodeId(2), 1.0).unwrap();
        graph.add_edge(NodeId(2), NodeId(0), 1.0).unwrap();

        let pattern = Pattern::circular_dependency(3);
        let matches = find_patterns(&graph, &pattern).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pattern_name, "Circular Dependency");
        assert_eq!(matches[0].severity, Severity::Critical);
    }

    #[test]
    fn test_dead_code_pattern() {
        // Node 0 -> Node 1, but Node 2 has no incoming edges
        let mut graph = CsrGraph::new();
        graph.add_edge(NodeId(0), NodeId(1), 1.0).unwrap();
        graph.add_edge(NodeId(2), NodeId(1), 1.0).unwrap(); // Node 2 exists but not called

        // Manually expand graph to include node 3 with no callers
        graph.add_edge(NodeId(3), NodeId(4), 1.0).unwrap();

        let pattern = Pattern::dead_code();
        let matches = find_patterns(&graph, &pattern).unwrap();

        // Nodes with no incoming edges (except if they're source nodes in edge list)
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_pattern_new() {
        let edges = vec![(0, 1), (1, 2)];
        let pattern = Pattern::new("Test Pattern".to_string(), edges, Severity::Medium);

        assert_eq!(pattern.name, "Test Pattern");
        assert_eq!(pattern.min_nodes, 3); // Nodes 0, 1, 2
        assert_eq!(pattern.severity, Severity::Medium);
    }

    #[test]
    fn test_no_circular_dependency() {
        // Linear graph: 0 -> 1 -> 2 (no cycle)
        let mut graph = CsrGraph::new();
        graph.add_edge(NodeId(0), NodeId(1), 1.0).unwrap();
        graph.add_edge(NodeId(1), NodeId(2), 1.0).unwrap();

        let pattern = Pattern::circular_dependency(3);
        let matches = find_patterns(&graph, &pattern).unwrap();

        assert_eq!(matches.len(), 0); // No cycles found
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }
}
