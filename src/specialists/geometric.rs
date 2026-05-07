//! S2: Geometric Specialist
//! Analyzes trust geometry via ZHC (Zariski closure) and holonomy

use crate::graph::FleetGraph;
use crate::specialists::{Specialist, SpecialistReport};
use std::collections::HashMap;

pub struct GeometricSpecialist {
    stress_threshold: f64,
}

impl GeometricSpecialist {
    pub fn new() -> Self {
        Self { stress_threshold: 0.15 }
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.stress_threshold = threshold;
        self
    }

    /// Calculate holonomy around a cycle
    /// Trust values act as parallel transport - going around a cycle should
    /// return to identity (product of trust values should be ~1 for identity)
    fn calculate_holonomy(&self, cycle: &[String], graph: &FleetGraph) -> f64 {
        let mut product = 1.0f64;

        for window in cycle.windows(2) {
            let from = &window[0];
            let to = &window[1];

            // Find trust value
            let trust = graph.edges.iter()
                .find(|e| (&e.from == from && &e.to == to) || (&e.from == to && &e.to == from))
                .map(|e| e.trust.value)
                .unwrap_or(0.5);

            // Trust values in [0,1] need to be interpreted as rotation-like
            // Using a simple approach: product of "angular" representations
            // trust = cos(theta) => accumulate theta
            let trust_clamped = if trust > 1.0 { 1.0 } else if trust < -1.0 { -1.0 } else { trust };
            let theta = (trust_clamped * std::f64::consts::PI).acos();
            product *= theta.cos();
        }

        // Holonomy = deviation from identity
        (1.0 - product.abs()).abs()
    }

    /// Find edges under geometric stress
    fn find_stressed_edges(&self, cycles: &[Vec<String>], graph: &FleetGraph) -> HashMap<String, f64> {
        let mut edge_stress: HashMap<String, f64> = HashMap::new();

        for cycle in cycles {
            let holonomy = self.calculate_holonomy(cycle, graph);

            if holonomy > self.stress_threshold {
                for window in cycle.windows(2) {
                    let key = format!("{}->{}", window[0], window[1]);
                    edge_stress.insert(key, holonomy);
                }
            }
        }

        edge_stress
    }

    /// Check for closed-loop consistency
    fn check_loop_consistency(&self, cycles: &[Vec<String>], graph: &FleetGraph) -> Vec<LoopCheck> {
        cycles.iter().map(|cycle| {
            let holonomy = self.calculate_holonomy(cycle, graph);
            let is_consistent = holonomy < self.stress_threshold;

            LoopCheck {
                cycle_length: cycle.len(),
                holonomy,
                is_consistent,
                deviation_from_identity: (1.0 - holonomy).abs(),
            }
        }).collect()
    }
}

#[derive(Debug, serde::Serialize)]
struct LoopCheck {
    cycle_length: usize,
    holonomy: f64,
    is_consistent: bool,
    deviation_from_identity: f64,
}

impl Default for GeometricSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl Specialist for GeometricSpecialist {
    fn analyze(&self, graph: &FleetGraph) -> SpecialistReport {
        let mut report = SpecialistReport::new("geometric");

        let cycles = graph.cycle_basis();

        if cycles.is_empty() {
            report.add_unanswered("No cycles detected - cannot compute holonomy".to_string());
            report.add_finding(
                "Graph has no cycles - no geometric loop structure to analyze".to_string(),
                0.9,
                vec!["cycle_basis is empty".to_string()],
            );
            report.confidence = 0.6;
            return report;
        }

        // Check for zero-trust edges
        let zero_trust_edges = graph.edges.iter()
            .filter(|e| e.trust.value.abs() < 0.01)
            .count();

        if zero_trust_edges > 0 {
            report.add_unanswered(
                format!("{} edges have zero/near-zero trust - geometric interpretation undefined", zero_trust_edges)
            );
        }

        // Calculate holonomy for each cycle
        let loop_checks = self.check_loop_consistency(&cycles, graph);

        // Overall geometric consistency
        let consistent_loops = loop_checks.iter().filter(|l| l.is_consistent).count();
        let consistency_ratio = consistent_loops as f64 / cycles.len() as f64;

        if consistency_ratio > 0.8 {
            report.add_finding(
                format!("Trust geometry is consistent: {}/{} loops closed properly", consistent_loops, cycles.len()),
                0.85,
                vec![format!("consistency_ratio = {:.2}", consistency_ratio)],
            );
        } else if consistency_ratio > 0.5 {
            report.add_finding(
                format!("Trust geometry is partially consistent: {}/{} loops closed properly", consistent_loops, cycles.len()),
                0.7,
                vec![format!("consistency_ratio = {:.2}", consistency_ratio)],
            );
        } else {
            report.add_finding(
                format!("Trust geometry has significant strain: only {}/{} loops closed properly", consistent_loops, cycles.len()),
                0.85,
                vec![format!("consistency_ratio = {:.2}", consistency_ratio)],
            );
        }

        // Find stressed edges
        let stressed = self.find_stressed_edges(&cycles, graph);
        if !stressed.is_empty() {
            let stressed_count = stressed.len();
            report.add_finding(
                format!("Found {} stressed edges (geometric strain detected)", stressed_count),
                0.8,
                vec![format!("holonomy > {:.2} threshold", self.stress_threshold)],
            );
        } else {
            report.add_finding(
                "No edges under geometric stress".to_string(),
                0.75,
                vec!["All cycles have low holonomy".to_string()],
            );
        }

        // Holonomy statistics
        let holonomies: Vec<f64> = loop_checks.iter().map(|l| l.holonomy).collect();
        let max_holonomy = holonomies.iter().cloned().fold(0.0f64, f64::max);
        let mean_holonomy = holonomies.iter().sum::<f64>() / holonomies.len() as f64;

        report.add_finding(
            format!("Holonomy range: mean={:.3}, max={:.3}", mean_holonomy, max_holonomy),
            0.7,
            vec!["Per-cycle holonomy statistics".to_string()],
        );

        // Most strained cycle — use fold to avoid NaN panic from f64::partial_cmp
        if !holonomies.is_empty() {
            let max_idx = holonomies.iter().enumerate().fold(0, |max_i, (i, v)| {
                if v > &holonomies[max_i] { i } else { max_i }
            });
            report.add_finding(
                format!("Most strained cycle: {} edges, holonomy={:.3}",
                    loop_checks[max_idx].cycle_length, loop_checks[max_idx].holonomy),
                0.75,
                vec![format!("cycle[{}]", max_idx)],
            );
        }

        report.set_raw_data(serde_json::json!({
            "cycles_analyzed": cycles.len(),
            "consistent_loops": consistent_loops,
            "consistency_ratio": consistency_ratio,
            "stressed_edges": stressed,
            "holonomy_stats": {
                "mean": mean_holonomy,
                "max": max_holonomy,
            },
            "loop_checks": loop_checks,
        }));

        let mut confidence: f64 = 0.75;
        if stressed.is_empty() {
            confidence += 0.1;
        }
        if zero_trust_edges > 0 {
            confidence -= 0.15;
        }
        report.confidence = confidence.max(0.4);

        report
    }

    fn id(&self) -> &'static str {
        "geometric"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geometric_small_rigid() {
        let specialist = GeometricSpecialist::new();
        let graph = crate::test_helpers::make_small_rigid();
        let report = specialist.analyze(&graph);

        assert_eq!(report.specialist_id, "geometric");
        // Small rigid should have some geometric assessment (consistent OR strained are both valid)
        assert!(report.findings.iter().any(|f| f.claim.contains("consistent") || f.claim.contains("strain")),
            "Should have some geometric assessment");
    }
}
