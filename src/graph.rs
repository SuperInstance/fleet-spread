//! Fleet graph data structures

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A fleet graph: vertices (agents) connected by trust-weighted edges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetGraph {
    pub id: String,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub adjacency: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vertex {
    pub id: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub trust: TrustValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustValue {
    pub value: f64,           // Primary trust value [-1, 1] or [0, 1]
    pub confidence: f64,      // How certain is this trust value
    pub history: Vec<f64>,     // Historical values for drift detection
    pub timestamp: Option<String>,
}

impl FleetGraph {
    /// Create a new fleet graph from vertices and edges
    pub fn new(id: String, vertices: Vec<Vertex>, edges: Vec<Edge>) -> Self {
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for v in &vertices {
            adjacency.insert(v.id.clone(), Vec::new());
        }
        for edge in &edges {
            adjacency.entry(edge.from.clone()).or_default().push(edge.to.clone());
            adjacency.entry(edge.to.clone()).or_default().push(edge.from.clone());
        }
        Self { id, vertices, edges, adjacency }
    }

    /// Number of vertices
    pub fn v(&self) -> usize {
        self.vertices.len()
    }

    /// Number of edges
    pub fn e(&self) -> usize {
        self.edges.len()
    }

    /// Number of connected components
    pub fn components(&self) -> usize {
        let mut visited: HashSet<String> = HashSet::new();
        let mut count = 0;
        for vertex in &self.vertices {
            if !visited.contains(&vertex.id) {
                self.dfs(&vertex.id, &mut visited);
                count += 1;
            }
        }
        count
    }

    fn dfs(&self, start: &str, visited: &mut HashSet<String>) {
        let mut stack = vec![start.to_string()];
        while let Some(current) = stack.pop() {
            if visited.insert(current.clone()) {
                if let Some(neighbors) = self.adjacency.get(&current) {
                    for neighbor in neighbors {
                        if !visited.contains(neighbor) {
                            stack.push(neighbor.clone());
                        }
                    }
                }
            }
        }
    }

    /// Get the cycle basis using a simple DFS approach
    pub fn cycle_basis(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited_edges: HashSet<(String, String)> = HashSet::new();

        for edge in &self.edges {
            let key = (edge.from.clone(), edge.to.clone());
            let rev_key = (edge.to.clone(), edge.from.clone());
            if visited_edges.contains(&key) || visited_edges.contains(&rev_key) {
                continue;
            }

            // Find cycle starting from this edge
            if let Some(cycle) = self.find_cycle(&edge.from, &edge.to, &mut visited_edges) {
                cycles.push(cycle);
            }
        }
        cycles
    }

    fn find_cycle(&self, start: &str, via: &str, visited_edges: &mut HashSet<(String, String)>) -> Option<Vec<String>> {
        let mut path = vec![start.to_string(), via.to_string()];
        let mut current = via.to_string();
        let mut used_edges: HashSet<(String, String)> = HashSet::new();
        used_edges.insert((start.to_string(), via.to_string()));

        while path.len() <= self.v() {
            let neighbors = self.adjacency.get(&current)?;
            let mut found_next = false;

            for neighbor in neighbors {
                if neighbor == start && path.len() > 2 {
                    // Found cycle back to start
                    visited_edges.extend(used_edges.iter().cloned());
                    visited_edges.extend(used_edges.iter().map(|(a, b)| (b.clone(), a.clone())));
                    return Some(path);
                }

                let edge_key = (current.clone(), neighbor.clone());
                if !used_edges.contains(&edge_key) && !used_edges.contains(&(neighbor.clone(), current.clone())) {
                    path.push(neighbor.clone());
                    used_edges.insert(edge_key);
                    current = neighbor.clone();
                    found_next = true;
                    break;
                }
            }

            if !found_next {
                // Dead end, backtrack
                if path.len() <= 2 {
                    return None;
                }
                path.pop();
                if let Some(prev) = path.last() {
                    current = prev.clone();
                }
            }
        }
        None
    }

    /// Calculate the first Betti number: β₁ = E - V + C
    pub fn betti_1(&self) -> i64 {
        (self.e() as i64) - (self.v() as i64) + (self.components() as i64)
    }

    /// Check if the graph meets Laman rigidity condition: E = 2V - 3
    pub fn is_laman_candidate(&self) -> bool {
        self.e() == 2 * self.v() - 3
    }

    /// Check if the graph is over-constrained
    pub fn is_over_constrained(&self) -> bool {
        self.e() > 2 * self.v() - 3
    }

    /// Check if the graph is under-constrained
    pub fn is_under_constrained(&self) -> bool {
        self.e() < 2 * self.v() - 3
    }

    /// Get the maximum degree (neighbor count) of any vertex
    pub fn max_degree(&self) -> usize {
        self.adjacency.values().map(|v| v.len()).max().unwrap_or(0)
    }

    /// Get degree of a specific vertex
    pub fn degree(&self, vertex_id: &str) -> usize {
        self.adjacency.get(vertex_id).map(|v| v.len()).unwrap_or(0)
    }

    /// Find edges that are redundant (part of cycles)
    pub fn redundant_edges(&self) -> HashSet<String> {
        let cycles = self.cycle_basis();
        let mut redundant: HashSet<String> = HashSet::new();
        for cycle in cycles {
            for window in cycle.windows(2) {
                redundant.insert(format!("{}->{}", window[0], window[1]));
                redundant.insert(format!("{}->{}", window[1], window[0]));
            }
        }
        redundant
    }
}

impl TrustValue {
    pub fn new(value: f64, confidence: f64) -> Self {
        Self {
            value,
            confidence,
            history: Vec::new(),
            timestamp: None,
        }
    }

    pub fn with_history(mut self, history: Vec<f64>) -> Self {
        self.history = history;
        self
    }

    /// Mean of historical values
    pub fn historical_mean(&self) -> Option<f64> {
        if self.history.is_empty() {
            None
        } else {
            Some(self.history.iter().sum::<f64>() / self.history.len() as f64)
        }
    }

    /// Standard deviation of historical values
    pub fn historical_std(&self) -> Option<f64> {
        if self.history.len() < 2 {
            return None;
        }
        let mean = self.historical_mean()?;
        let variance = self.history.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / self.history.len() as f64;
        Some(variance.sqrt())
    }

    /// Is the current value anomalous (> 2σ from history)?
    pub fn is_anomalous(&self, threshold: f64) -> bool {
        if let (Some(mean), Some(std)) = (self.historical_mean(), self.historical_std()) {
            if std < 1e-10 {
                return false; // No meaningful variance
            }
            return (self.value - mean).abs() > threshold * std;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_small_rigid() -> FleetGraph {
        // V=5, E=7, C=1: Laman rigid (E = 2V-3 = 7)
        let vertices = (0..5).map(|i| Vertex {
            id: format!("agent-{}", i),
            metadata: HashMap::new(),
        }).collect();

        let edges = vec![
            Edge { from: "agent-0".into(), to: "agent-1".into(), trust: TrustValue::new(0.9, 0.8) },
            Edge { from: "agent-1".into(), to: "agent-2".into(), trust: TrustValue::new(0.85, 0.8) },
            Edge { from: "agent-2".into(), to: "agent-0".into(), trust: TrustValue::new(0.88, 0.8) },
            Edge { from: "agent-0".into(), to: "agent-3".into(), trust: TrustValue::new(0.75, 0.8) },
            Edge { from: "agent-3".into(), to: "agent-4".into(), trust: TrustValue::new(0.82, 0.8) },
            Edge { from: "agent-4".into(), to: "agent-2".into(), trust: TrustValue::new(0.78, 0.8) },
            Edge { from: "agent-1".into(), to: "agent-4".into(), trust: TrustValue::new(0.71, 0.8) },
        ];

        FleetGraph::new("small-rigid".into(), vertices, edges)
    }

    fn make_over_connected() -> FleetGraph {
        // V=5, E=20, over-connected
        let vertices = (0..5).map(|i| Vertex {
            id: format!("agent-{}", i),
            metadata: HashMap::new(),
        }).collect();

        let mut edges = Vec::new();
        for i in 0..5 {
            for j in (i+1)..5 {
                edges.push(Edge {
                    from: format!("agent-{}", i),
                    to: format!("agent-{}", j),
                    trust: TrustValue::new(0.5 + (i * j) as f64 * 0.05, 0.7),
                });
            }
        }

        // Add extra edges to exceed 2V-3 = 7
        edges.push(Edge { from: "agent-0".into(), to: "agent-1".into(), trust: TrustValue::new(0.6, 0.5) });
        edges.push(Edge { from: "agent-2".into(), to: "agent-3".into(), trust: TrustValue::new(0.55, 0.5) });

        FleetGraph::new("over-connected".into(), vertices, edges)
    }

    fn make_disconnected() -> FleetGraph {
        // V=8, 2 components: A={0,1,2,3,4}, B={5,6,7}
        let vertices = (0..8).map(|i| Vertex {
            id: format!("agent-{}", i),
            metadata: HashMap::new(),
        }).collect();

        // Component A: 5 vertices, 7 edges (rigid)
        let edges_a = vec![
            Edge { from: "agent-0".into(), to: "agent-1".into(), trust: TrustValue::new(0.9, 0.8) },
            Edge { from: "agent-1".into(), to: "agent-2".into(), trust: TrustValue::new(0.85, 0.8) },
            Edge { from: "agent-2".into(), to: "agent-0".into(), trust: TrustValue::new(0.88, 0.8) },
            Edge { from: "agent-0".into(), to: "agent-3".into(), trust: TrustValue::new(0.75, 0.8) },
            Edge { from: "agent-3".into(), to: "agent-4".into(), trust: TrustValue::new(0.82, 0.8) },
            Edge { from: "agent-4".into(), to: "agent-2".into(), trust: TrustValue::new(0.78, 0.8) },
            Edge { from: "agent-1".into(), to: "agent-4".into(), trust: TrustValue::new(0.71, 0.8) },
        ];

        // Component B: 3 vertices, 2 edges (under-constrained)
        let edges_b = vec![
            Edge { from: "agent-5".into(), to: "agent-6".into(), trust: TrustValue::new(0.6, 0.7) },
            Edge { from: "agent-6".into(), to: "agent-7".into(), trust: TrustValue::new(0.65, 0.7) },
        ];

        let mut all_edges = edges_a;
        all_edges.extend(edges_b);

        FleetGraph::new("disconnected".into(), vertices, all_edges)
    }

    #[test]
    fn test_small_rigid_properties() {
        let g = make_small_rigid();
        assert_eq!(g.v(), 5);
        assert_eq!(g.e(), 7);
        assert_eq!(g.components(), 1);
        assert!(g.is_laman_candidate());
        assert_eq!(g.betti_1(), 3); // E - V + C = 7 - 5 + 1 = 3
    }

    #[test]
    fn test_over_connected_properties() {
        let g = make_over_connected();
        assert_eq!(g.v(), 5);
        assert!(g.e() > 7); // More than Laman count
        assert!(g.is_over_constrained());
        assert!(g.betti_1() > 3);
    }

    #[test]
    fn test_disconnected_properties() {
        let g = make_disconnected();
        assert_eq!(g.v(), 8);
        assert_eq!(g.e(), 9);
        assert_eq!(g.components(), 2);
        assert!(g.betti_1() <= 3); // 9 - 8 + 2 = 3, exactly at boundary
    }

    #[test]
    fn test_trust_anomaly_detection() {
        // Value close to history should not be anomalous
        let trust = TrustValue::new(0.91, 0.8)
            .with_history(vec![0.9, 0.88, 0.92, 0.89, 0.91]);

        assert!(!trust.is_anomalous(2.0)); // 0.91 is within 2σ of mean 0.90
        assert!(!trust.is_anomalous(1.0)); // 0.91 is within 1σ of mean 0.90

        // Value far from history should be anomalous
        let trust_far = TrustValue::new(0.95, 0.8)
            .with_history(vec![0.9, 0.88, 0.92, 0.89, 0.91]);
        assert!(trust_far.is_anomalous(2.0)); // 0.95 is ~3.5σ from mean, anomalous at 2σ
    }
}
