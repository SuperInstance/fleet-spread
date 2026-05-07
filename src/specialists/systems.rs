//! S4: Systems Specialist
//! Analyzes rigidity using Laman's theorem: E = 2V - 3 for generic rigidity in 2D

use crate::graph::FleetGraph;
use crate::specialists::{Specialist, SpecialistReport};
use std::collections::HashMap;

pub struct SystemsSpecialist {
    rigidity_threshold: i64,
}

impl SystemsSpecialist {
    pub fn new() -> Self {
        Self { rigidity_threshold: 0 }
    }

    /// Check if a subgraph is Laman-rigid
    /// A graph is Laman-rigid in 2D if it can be embedded as a rigid bar-joint framework
    /// Laman's condition: |E| = 2|V| - 3 and every subgraph on k vertices has at most 2k - 3 edges
    fn is_laman_rigid(&self, graph: &FleetGraph) -> (bool, Vec<String>) {
        let v = graph.v() as i64;
        let e = graph.e() as i64;
        let lamancount = 2 * v - 3;

        if e != lamancount {
            return (false, vec![format!("E={} ≠ 2V-3={}", e, lamancount)]);
        }

        // Check subgraphs for Laman condition
        let vertices = &graph.vertices;
        let violations = self.check_laman_subgraphs(graph);

        if violations.is_empty() {
            (true, vec![format!("E={} = 2V-3={}, all subgraphs valid", e, lamancount)])
        } else {
            (false, violations)
        }
    }

    fn check_laman_subgraphs(&self, graph: &FleetGraph) -> Vec<String> {
        let mut violations = Vec::new();
        let v_count = graph.vertices.len();

        // Check all subsets of vertices
        for size in 3..=v_count {
            let indices: Vec<usize> = (0..v_count).collect();
            let combinations = self.combinations(&indices, size);

            for combo in combinations {
                let sub_vertices: Vec<&str> = combo.iter().map(|&i| graph.vertices[i].id.as_str()).collect();
                let sub_edges = graph.edges.iter()
                    .filter(|e| sub_vertices.contains(&e.from.as_str()) && sub_vertices.contains(&e.to.as_str()))
                    .count();

                let expected = 2 * size - 3;
                if sub_edges > expected {
                    violations.push(format!("Subgraph with {} vertices has {} edges (expected ≤ {})", size, sub_edges, expected));
                }
            }
        }

        violations
    }

    fn combinations(&self, arr: &[usize], k: usize) -> Vec<Vec<usize>> {
        let mut result = Vec::new();
        self.combinations_helper(arr, k, 0, &mut Vec::new(), &mut result);
        result
    }

    fn combinations_helper(&self, arr: &[usize], k: usize, start: usize, current: &mut Vec<usize>, result: &mut Vec<Vec<usize>>) {
        if current.len() == k {
            result.push(current.clone());
            return;
        }

        for i in start..arr.len() {
            current.push(arr[i]);
            self.combinations_helper(arr, k, i + 1, current, result);
            current.pop();
        }
    }

    /// Find over-constrained nodes (nodes with too many edges relative to 2V-3)
    fn find_over_constrained_nodes(&self, graph: &FleetGraph) -> Vec<NodeConstraint> {
        let v = graph.v();
        let lamancount = 2 * v - 3;
        let avg_degree = (2 * graph.e()) as f64 / v as f64;

        graph.vertices.iter().map(|vertex| {
            let degree = graph.degree(&vertex.id);
            let expected_degree = 2.0 * avg_degree; // Rough expectation
            let is_over = degree as f64 > expected_degree * 1.5;

            NodeConstraint {
                node_id: vertex.id.clone(),
                degree,
                expected_degree: expected_degree as usize,
                is_over_constrained: is_over,
            }
        }).collect()
    }

    /// Check for rigid components
    fn analyze_rigid_components(&self, graph: &FleetGraph) -> Vec<ComponentRigidity> {
        // Simple per-component analysis
        let mut visited = std::collections::HashSet::new();
        let mut components = Vec::new();

        for vertex in &graph.vertices {
            if visited.contains(&vertex.id) {
                continue;
            }

            let vertices_in_comp = self.extract_component(graph, &vertex.id, &mut visited);
            let v = vertices_in_comp.len();
            let e = graph.edges.iter().filter(|edge| vertices_in_comp.contains(&edge.from) && vertices_in_comp.contains(&edge.to)).count();
            let lamancount = if v >= 2 { 2 * v - 3 } else { 0 };

            let rigidity_status: &str = if v < 3 {
                "too_small"
            } else if lamancount == 0 {
                "too_small"
            } else if e < lamancount {
                "under_constrained"
            } else if e == lamancount {
                "rigid"
            } else {
                "over_constrained"
            };

            components.push(ComponentRigidity {
                vertices: v,
                edges: e,
                lamancount,
                status: rigidity_status.to_string(),
                vertices_list: vertices_in_comp,
            });
        }

        components
    }

    fn extract_component(&self, graph: &FleetGraph, start: &str, visited: &mut std::collections::HashSet<String>) -> Vec<String> {
        let mut vertices_in_comp = Vec::new();
        let mut stack = vec![start.to_string()];

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            vertices_in_comp.push(current.clone());

            if let Some(neighbors) = graph.adjacency.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        stack.push(neighbor.clone());
                    }
                }
            }
        }

        vertices_in_comp
    }
}

#[derive(Debug, serde::Serialize)]
struct NodeConstraint {
    node_id: String,
    degree: usize,
    expected_degree: usize,
    is_over_constrained: bool,
}

#[derive(Debug, serde::Serialize)]
struct ComponentRigidity {
    vertices: usize,
    edges: usize,
    lamancount: usize,
    status: String,
    vertices_list: Vec<String>,
}

impl Default for SystemsSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl Specialist for SystemsSpecialist {
    fn analyze(&self, graph: &FleetGraph) -> SpecialistReport {
        let mut report = SpecialistReport::new("systems");

        let v = graph.v();
        let e = graph.e();

        // Check Laman condition
        let lamancount = 2 * v - 3;

        if v < 3 {
            report.add_unanswered("Graph too small for rigidity analysis (V < 3)".to_string());
            report.add_finding(
                "Graph has insufficient vertices for Laman rigidity (V < 3)".to_string(),
                0.9,
                vec![format!("V = {}", v)],
            );
            report.confidence = 0.5;
            return report;
        }

        // Global rigidity check
        let (is_rigid, rigid_reasons) = self.is_laman_rigid(graph);
        if is_rigid {
            report.add_finding(
                format!("Graph is Laman-rigid: E={} = 2V-3={}", e, lamancount),
                0.9,
                rigid_reasons,
            );
        } else if e > lamancount {
            report.add_finding(
                format!("Graph is over-constrained: E={} > 2V-3={}", e, lamancount),
                0.85,
                rigid_reasons,
            );
        } else {
            report.add_finding(
                format!("Graph is under-constrained: E={} < 2V-3={}", e, lamancount),
                0.85,
                rigid_reasons,
            );
        }

        // Max degree analysis
        let max_degree = graph.max_degree();
        let max_degree_expected = (2 * e) / v;

        report.add_finding(
            format!("Max vertex degree: {} (expected: ~{})", max_degree, max_degree_expected),
            0.7,
            vec![format!("V={}, E={}", v, e)],
        );

        // Over-constrained nodes
        let over_constrained = self.find_over_constrained_nodes(graph);
        let over_count = over_constrained.iter().filter(|n| n.is_over_constrained).count();

        if over_count > 0 {
            report.add_finding(
                format!("{} nodes are over-constrained (high degree relative to average)", over_count),
                0.75,
                vec![format!("{:.1}% of vertices", 100.0 * over_count as f64 / v as f64)],
            );
        }

        // Per-component rigidity
        let components = self.analyze_rigid_components(graph);
        let rigid_components = components.iter().filter(|c| c.status == "rigid").count();
        let under_components = components.iter().filter(|c| c.status == "under_constrained").count();
        let over_components = components.iter().filter(|c| c.status == "over_constrained").count();

        if components.len() > 1 {
            report.add_finding(
                format!("Component analysis: {} rigid, {} under, {} over",
                    rigid_components, under_components, over_components),
                0.8,
                vec![format!("{} total components", components.len())],
            );
        }

        // Redundant edges (edges not needed for rigidity)
        let redundant = graph.redundant_edges();
        if !redundant.is_empty() {
            report.add_finding(
                format!("{} redundant edges detected (not needed for rigidity)", redundant.len()),
                0.7,
                vec!["Edges in cycle basis but not in minimal rigidity subgraph".to_string()],
            );
        }

        report.set_raw_data(serde_json::json!({
            "V": v,
            "E": e,
            "lamancount": lamancount,
            "is_laman_rigid": is_rigid,
            "max_degree": max_degree,
            "max_degree_expected": max_degree_expected,
            "over_constrained_nodes": over_constrained,
            "component_rigidity": components,
            "redundant_edges": redundant.len(),
        }));

        let mut confidence: f64 = 0.8;
        if is_rigid {
            confidence += 0.1;
        }
        if over_count > v / 2 {
            confidence -= 0.1;
        }
        report.confidence = confidence.max(0.5);

        report
    }

    fn id(&self) -> &'static str {
        "systems"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_rigid_systems() {
        let specialist = SystemsSpecialist::new();
        let graph = crate::test_helpers::make_small_rigid();
        let report = specialist.analyze(&graph);

        assert_eq!(report.specialist_id, "systems");
        assert!(report.findings.iter().any(|f| f.claim.contains("Laman-rigid")));
    }

    #[test]
    fn test_over_connected_systems() {
        let specialist = SystemsSpecialist::new();
        let graph = crate::test_helpers::make_over_connected();
        let report = specialist.analyze(&graph);

        assert!(report.findings.iter().any(|f| f.claim.contains("over-constrained")));
    }
}
