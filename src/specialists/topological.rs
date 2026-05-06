//! S1: Topological Specialist
//! Analyzes graph topology using Betti numbers and cycle basis

use crate::graph::FleetGraph;
use crate::specialists::{Specialist, SpecialistReport};

pub struct TopologicalSpecialist {
    cycle_threshold: usize,
}

impl TopologicalSpecialist {
    pub fn new() -> Self {
        Self { cycle_threshold: 1 }
    }

    /// Analyze connected components with detailed breakdown
    fn analyze_components(&self, graph: &FleetGraph) -> (usize, Vec<ComponentStats>) {
        let mut visited = std::collections::HashSet::new();
        let mut components = Vec::new();

        for vertex in &graph.vertices {
            if visited.contains(&vertex.id) {
                continue;
            }

            let (size, edges, vertices) = self.dfs_component(graph, &vertex.id, &mut visited);
            let betti_1 = edges as i64 - size as i64 + 1;
            components.push(ComponentStats {
                vertices: size,
                edges,
                betti_1,
                is_rigid_possible: size >= 3 && edges >= 2 * size - 3,
            });
        }

        (components.len(), components)
    }

    fn dfs_component(&self, graph: &FleetGraph, start: &str, visited: &mut std::collections::HashSet<String>) -> (usize, usize, Vec<String>) {
        let mut stack = vec![start.to_string()];
        let mut vertices = Vec::new();
        let mut edges = 0;

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            vertices.push(current.clone());

            if let Some(neighbors) = graph.adjacency.get(&current) {
                for neighbor in neighbors {
                    edges += 1;
                    if !visited.contains(neighbor) {
                        stack.push(neighbor.clone());
                    }
                }
            }
        }

        edges /= 2; // Count undirected edges once
        (vertices.len(), edges, vertices)
    }
}

#[derive(Debug, serde::Serialize)]
struct ComponentStats {
    vertices: usize,
    edges: usize,
    betti_1: i64,
    is_rigid_possible: bool,
}

impl Default for TopologicalSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl Specialist for TopologicalSpecialist {
    fn analyze(&self, graph: &FleetGraph) -> SpecialistReport {
        let mut report = SpecialistReport::new("topological");

        // Handle pathological cases
        if graph.v() < 3 {
            report.add_unanswered("Graph too small for cycle detection (V < 3)".to_string());
            report.add_finding(
                "Graph has insufficient vertices for meaningful topological analysis".to_string(),
                0.9,
                vec!["V < 3".to_string()],
            );
            report.confidence = 0.5;
            return report;
        }

        let components = graph.components();
        let v = graph.v();
        let e = graph.e();
        let beta_1 = graph.betti_1();
        let cycles = graph.cycle_basis();

        // Component analysis
        let (num_components, component_stats) = self.analyze_components(graph);
        let has_disconnected = num_components > 1;

        // Global Betti number analysis
        let beta_1_rounded = beta_1.max(0) as usize;

        if beta_1 < 0 {
            report.add_finding(
                format!("Graph is under-constrained: E={}, V={}, β₁={}", e, v, beta_1),
                0.95,
                vec![format!("E - V + C = {} - {} + {}", e, v, num_components)],
            );
        } else if beta_1 == 0 {
            report.add_finding(
                "Graph is a forest (no cycles)".to_string(),
                0.9,
                vec![format!("β₁ = {}", beta_1)],
            );
        } else {
            report.add_finding(
                format!("Graph contains {} independent cycles (β₁ = {})", beta_1_rounded, beta_1),
                0.9,
                vec![format!("E - V + C = {} - {} + {}", e, v, num_components)],
            );
        }

        // Cycle complexity
        let cycle_complexity = if cycles.len() > 10 {
            "high"
        } else if cycles.len() > 3 {
            "moderate"
        } else if cycles.len() > 0 {
            "low"
        } else {
            "none"
        };

        report.add_finding(
            format!("Cycle complexity: {} ({} cycles detected)", cycle_complexity, cycles.len()),
            0.85,
            vec![format!("cycle_basis.len() = {}", cycles.len())],
        );

        // Connectivity analysis
        if has_disconnected {
            report.add_finding(
                format!("Graph is disconnected: {} components detected", num_components),
                0.95,
                vec![format!("C = {}", num_components)],
            );
            report.add_unanswered("Cross-component trust relationships cannot be analyzed".to_string());
        } else {
            report.add_finding(
                "Graph is connected".to_string(),
                0.95,
                vec!["C = 1".to_string()],
            );
        }

        // Component-level findings
        for (i, stats) in component_stats.iter().enumerate() {
            let rigidity_status = if stats.is_rigid_possible {
                "can be rigid"
            } else {
                "cannot be rigid (under-constrained)"
            };

            report.add_finding(
                format!("Component {}: V={}, E={}, β₁={}, {}",
                    i + 1, stats.vertices, stats.edges, stats.betti_1, rigidity_status),
                0.8,
                vec![format!("component[{}]", i)],
            );
        }

        // Set raw data
        report.set_raw_data(serde_json::json!({
            "V": v,
            "E": e,
            "C": num_components,
            "beta_1": beta_1,
            "cycles_detected": cycles.len(),
            "cycle_complexity": cycle_complexity,
            "component_stats": component_stats,
        }));

        // Confidence based on data quality
        let mut confidence: f64 = 0.8;
        if has_disconnected {
            confidence -= 0.1;
        }
        if cycles.len() == 0 && graph.v() > 3 {
            confidence -= 0.1;
        }
        report.confidence = confidence.max(0.5);

        report
    }

    fn id(&self) -> &'static str {
        "topological"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_rigid_topology() {
        let specialist = TopologicalSpecialist::new();
        let graph = crate::test_helpers::make_small_rigid();
        let report = specialist.analyze(&graph);

        assert_eq!(report.specialist_id, "topological");
        assert!(report.confidence > 0.7);
        assert!(report.findings.iter().any(|f| f.claim.contains("β₁ = 3")));
    }

    #[test]
    fn test_disconnected_topology() {
        let specialist = TopologicalSpecialist::new();
        let graph = crate::test_helpers::make_disconnected();
        let report = specialist.analyze(&graph);

        assert!(report.findings.iter().any(|f| f.claim.contains("disconnected")));
        assert!(!report.unanswered.is_empty());
    }
}
