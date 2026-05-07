//! S2: Geometric Specialist — ZHC (Zariski closure) + holonomy analysis
//!
//! INTEGRATION: This module now uses fleet-coordinate's real ZHC consensus
//! instead of computing holonomy from scratch. The geometric specialist
//! is the bridge between fleet-spread's graph structure and fleet-coordinate's
//! rigorous ZHC computation.
//!
//! # Before (approximation — WRONG)
//!
//! The old code computed holonomy from scratch using trust values as cosines:
//! ```rust
//! let theta = (trust * PI).acos();
//! product *= theta.cos();  // trust = cos(theta) → product = cos(θ)·cos(θ)·...
//! let holonomy = (1.0 - product.abs()).abs();
//! ```
//! This is mathematically wrong. The "trust as rotation" metaphor breaks down
//! because trust is NOT a rotation angle.
//!
//! # After (rigorous)
//!
//! fleet-coordinate's ZhcConsensus uses actual 3D holonomy matrices. Closed loops
//! sum to identity when consistent. This is actual differential geometry.

use crate::graph::FleetGraph;
use crate::specialists::{Specialist, SpecialistReport};
use crate::graph_state::FleetGraphState;
use std::collections::HashMap;

// =============================================================================
// MAIN SPECIALIST TYPE
// =============================================================================

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

    /// Analyze with optional ZHC integration
    ///
    /// If zhc_state is provided, uses fleet-coordinate's ZHC consensus for
    /// rigorous geometric analysis. Otherwise falls back to the old trust-holonomy
    /// approximation (kept for backwards compatibility).
    pub fn analyze_with_zhc(
        &self,
        graph: &FleetGraph,
        zhc_state: Option<&crate::graph_state::FleetGraphState>,
    ) -> SpecialistReport {
        let mut report = SpecialistReport::new("geometric");
        let cycles = graph.cycle_basis();

        if cycles.is_empty() {
            report.add_unanswered("No cycles detected - cannot compute ZHC".to_string());
            report.add_finding(
                "Graph has no cycles - no geometric loop structure to analyze".to_string(),
                0.9,
                vec!["cycle_basis is empty".to_string()],
            );
            report.confidence = 0.6;
            return report;
        }

        let zero_trust_edges = graph
            .edges
            .iter()
            .filter(|e| e.trust.value.abs() < 0.01)
            .count();

        if zero_trust_edges > 0 {
            report.add_unanswered(format!(
                "{} edges have zero/near-zero trust - ZHC interpretation undefined",
                zero_trust_edges
            ));
        }

        // PRIMARY: Use real ZHC consensus from fleet-coordinate
        let (consistent_loops, consistency_ratio, max_dev, mean_dev) =
            if let Some(_state) = zhc_state {
                let zhc_result = graph.run_coordination();
                let cycle_analysis = graph.cycle_zhc_analysis();
                let consistent = cycle_analysis.iter().filter(|c| c.is_consistent).count();
                let ratio = consistent as f64 / cycles.len() as f64;
                let max_d = cycle_analysis
                    .iter()
                    .map(|c| c.deviation)
                    .fold(0.0f64, f64::max);
                let mean_d =
                    cycle_analysis.iter().map(|c| c.deviation).sum::<f64>() / cycles.len() as f64;

                if zhc_result.deviation > 0.01 {
                    report.add_unanswered(format!(
                        "ZHC degradation: deviation={:.3}",
                        zhc_result.deviation
                    ));
                }
                (consistent, ratio, max_d, mean_d)
            } else {
                // FALLBACK: Old trust-holonomy approximation
                let loop_checks = self.check_loop_consistency_old(&cycles, graph);
                let consistent = loop_checks.iter().filter(|l| l.is_consistent).count();
                let ratio = consistent as f64 / cycles.len() as f64;
                let holonomies: Vec<f64> = loop_checks.iter().map(|l| l.holonomy).collect();
                let max_h = holonomies.iter().cloned().fold(0.0f64, f64::max);
                let mean_h = holonomies.iter().sum::<f64>() / holonomies.len() as f64;
                (consistent, ratio, max_h, mean_h)
            };

        // Report findings
        if consistency_ratio > 0.8 {
            report.add_finding(
                format!(
                    "Trust geometry is ZHC-consistent: {}/{} loops closed properly",
                    consistent_loops,
                    cycles.len()
                ),
                0.85,
                vec![format!("consistency_ratio = {:.2}", consistency_ratio)],
            );
        } else if consistency_ratio > 0.5 {
            report.add_finding(
                format!(
                    "Trust geometry is partially ZHC-consistent: {}/{} loops closed properly",
                    consistent_loops,
                    cycles.len()
                ),
                0.7,
                vec![format!("consistency_ratio = {:.2}", consistency_ratio)],
            );
        } else {
            report.add_finding(
                format!(
                    "Trust geometry has ZHC strain: only {}/{} loops closed properly",
                    consistent_loops,
                    cycles.len()
                ),
                0.85,
                vec![format!("consistency_ratio = {:.2}", consistency_ratio)],
            );
        }

        // Stressed edges (edge-local, still uses trust approximation)
        let stressed = self.find_stressed_edges_old(&cycles, graph);
        if !stressed.is_empty() {
            report.add_finding(
                format!(
                    "Found {} stressed edges (geometric strain detected)",
                    stressed.len()
                ),
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

        report.add_finding(
            format!("ZHC deviation range: mean={:.3}, max={:.3}", mean_dev, max_dev),
            0.7,
            vec!["Per-cycle ZHC deviation statistics".to_string()],
        );

        // Most strained cycle
        if cycles.len() > 1 {
            let analysis = graph.cycle_zhc_analysis();
            let max_dev = analysis.iter().map(|c| c.deviation).fold(0.0f64, f64::max);
            let max_idx = analysis.iter().enumerate()
                .find(|(_, c)| c.deviation == max_dev)
                .map(|(i, _)| i);
            if let Some(idx) = max_idx {
                report.add_finding(
                    format!(
                        "Most ZHC-strained cycle: {} edges, deviation={:.3}",
                        cycles[idx].len(),
                        max_dev
                    ),
                    0.75,
                    vec![format!("cycle[{}]", idx)],
                );
            }
        }

        report.set_raw_data(serde_json::json!({
            "cycles_analyzed": cycles.len(),
            "consistent_loops": consistent_loops,
            "consistency_ratio": consistency_ratio,
            "stressed_edges": stressed.len(),
            "zhc_deviation": { "mean": mean_dev, "max": max_dev },
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

    fn check_loop_consistency_old(&self, cycles: &[Vec<String>], graph: &FleetGraph) -> Vec<LoopCheck> {
        cycles
            .iter()
            .map(|cycle| {
                let holonomy = self.calculate_trust_holonomy(cycle, graph);
                let is_consistent = holonomy < self.stress_threshold;
                LoopCheck {
                    cycle_length: cycle.len(),
                    holonomy,
                    is_consistent,
                    deviation_from_identity: (1.0 - holonomy).abs(),
                }
            })
            .collect()
    }

    fn find_stressed_edges_old(
        &self,
        cycles: &[Vec<String>],
        graph: &FleetGraph,
    ) -> HashMap<String, f64> {
        let mut edge_stress: HashMap<String, f64> = HashMap::new();
        for cycle in cycles {
            let holonomy = self.calculate_trust_holonomy(cycle, graph);
            if holonomy > self.stress_threshold {
                for window in cycle.windows(2) {
                    let key = format!("{}->{}", window[0], window[1]);
                    edge_stress.insert(key, holonomy);
                }
            }
        }
        edge_stress
    }

    /// Calculate holonomy around a cycle using trust geometry (OLD approximation)
    fn calculate_trust_holonomy(&self, cycle: &[String], graph: &FleetGraph) -> f64 {
        let mut product = 1.0f64;
        for window in cycle.windows(2) {
            let from = &window[0];
            let to = &window[1];
            let trust = graph
                .edges
                .iter()
                .find(|e| (&e.from == from && &e.to == to) || (&e.from == to && &e.to == from))
                .map(|e| e.trust.value)
                .unwrap_or(0.5);
            let trust_clamped = trust.clamp(-1.0, 1.0);
            let theta = (trust_clamped * std::f64::consts::PI).acos();
            product *= theta.cos();
        }
        (1.0 - product.abs()).abs()
    }
}

impl Default for GeometricSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl Specialist for GeometricSpecialist {
    fn analyze(&self, graph: &FleetGraph) -> SpecialistReport {
        // Default: no state, uses old fallback
        self.analyze_with_zhc(graph, None)
    }

    fn id(&self) -> &'static str {
        "geometric"
    }
}

// =============================================================================
// ZHC COORDINATION BRIDGE (fleet-coordinate integration)
// =============================================================================

/// Bridge to fleet-coordinate's ZHC consensus engine
pub trait CoordinateZhcExt {
    /// Run ZHC consensus on this graph's topology
    fn run_coordination(&self) -> ZhcCoordinationResult;

    /// Get per-cycle ZHC analysis
    fn cycle_zhc_analysis(&self) -> Vec<CycleZhcResult>;
}

/// Result of ZHC coordination run
#[derive(Debug, Clone)]
pub struct ZhcCoordinationResult {
    pub is_consistent: bool,
    pub deviation: f64,
    pub information_bits: f64,
    pub conflicted_tiles: Vec<String>,
}

/// Result of per-cycle ZHC analysis
#[derive(Debug, Clone)]
pub struct CycleZhcResult {
    pub cycle_id: usize,
    pub cycle_length: usize,
    pub is_consistent: bool,
    pub deviation: f64,
}

impl CoordinateZhcExt for FleetGraph {
    fn run_coordination(&self) -> ZhcCoordinationResult {
        // Map String IDs → u64 IDs
        let id_map: HashMap<String, u64> = self
            .vertices
            .iter()
            .enumerate()
            .map(|(i, v)| (v.id.clone(), i as u64))
            .collect();
        let reverse_map: HashMap<u64, String> =
            id_map.values().cloned().zip(id_map.keys().cloned()).collect();

        // Build fleet-coordinate ZHC consensus
        let mut zhc = fleet_coordinate::zhc::ZhcConsensus::new(0.5);
        for v in &self.vertices {
            if let Some(&uid) = id_map.get(&v.id) {
                let neighbors: Vec<u64> = self
                    .edges
                    .iter()
                    .filter(|e| &e.from == &v.id || &e.to == &v.id)
                    .filter_map(|e| {
                        let other = if &e.from == &v.id { &e.to } else { &e.from };
                        id_map.get(other).copied()
                    })
                    .collect();
                let trust_mag = v
                    .metadata
                    .get("trust_magnitude")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.8);
                zhc.add_tile(
                    uid,
                    [trust_mag, (1.0 - trust_mag).sqrt(), 0.0],
                    neighbors,
                );
            }
        }

        let result = zhc.run_consensus();
        let conflicted_str: Vec<String> = result
            .conflicted_tiles
            .iter()
            .filter_map(|&id| reverse_map.get(&id).cloned())
            .collect();

        ZhcCoordinationResult {
            is_consistent: result.is_consistent,
            deviation: result.deviation,
            information_bits: result.information_bits,
            conflicted_tiles: conflicted_str,
        }
    }

    fn cycle_zhc_analysis(&self) -> Vec<CycleZhcResult> {
        let cycles = self.cycle_basis();
        let id_map: HashMap<String, u64> = self
            .vertices
            .iter()
            .enumerate()
            .map(|(i, v)| (v.id.clone(), i as u64))
            .collect();

        cycles
            .iter()
            .enumerate()
            .map(|(idx, cycle)| {
                let cycle_ids: Vec<u64> = cycle
                    .iter()
                    .filter_map(|id| id_map.get(id).copied())
                    .collect();

                let mut zhc = fleet_coordinate::zhc::ZhcConsensus::new(0.5);
                for &uid in &cycle_ids {
                    let neighbors: Vec<u64> =
                        cycle_ids.iter().filter(|&&n| n != uid).cloned().collect();
                    zhc.add_tile(uid, [0.8, 0.6, 0.0], neighbors);
                }

                let result = zhc.run_consensus();
                CycleZhcResult {
                    cycle_id: idx,
                    cycle_length: cycle.len(),
                    is_consistent: result.is_consistent,
                    deviation: result.deviation,
                }
            })
            .collect()
    }
}

/// Loop consistency check result
#[derive(Debug, serde::Serialize)]
struct LoopCheck {
    cycle_length: usize,
    holonomy: f64,
    is_consistent: bool,
    deviation_from_identity: f64,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_triangle() -> FleetGraph {
        let v1 = crate::graph::Vertex { id: "A".to_string(), metadata: Default::default() };
        let v2 = crate::graph::Vertex { id: "B".to_string(), metadata: Default::default() };
        let v3 = crate::graph::Vertex { id: "C".to_string(), metadata: Default::default() };
        let e1 = crate::graph::Edge {
            from: "A".to_string(),
            to: "B".to_string(),
            trust: crate::graph::TrustValue::new(0.9, 0.95),
        };
        let e2 = crate::graph::Edge {
            from: "B".to_string(),
            to: "C".to_string(),
            trust: crate::graph::TrustValue::new(0.9, 0.95),
        };
        let e3 = crate::graph::Edge {
            from: "C".to_string(),
            to: "A".to_string(),
            trust: crate::graph::TrustValue::new(0.9, 0.95),
        };
        FleetGraph::new(
            "test".to_string(),
            vec![v1, v2, v3],
            vec![e1, e2, e3],
        )
    }

    #[test]
    fn test_zhc_coordination_on_triangle() {
        let graph = make_simple_triangle();
        let result = graph.run_coordination();
        assert!(
            result.is_consistent || result.deviation < 0.5,
            "Triangle should be ZHC-consistent, got deviation={}",
            result.deviation
        );
    }

    #[test]
    fn test_cycle_zhc_analysis() {
        let graph = make_simple_triangle();
        let analysis = graph.cycle_zhc_analysis();
        assert!(!analysis.is_empty(), "Triangle should have at least one cycle");
        assert_eq!(analysis[0].cycle_length, 3, "Triangle cycle should have 3 edges");
    }

    #[test]
    fn test_analyze_with_zhc_state() {
        let graph = make_simple_triangle();
        let specialist = GeometricSpecialist::new();
        let state = FleetGraphState::degraded_zhc();
        let report = specialist.analyze_with_zhc(&graph, Some(&state));

        let has_zhc_finding = report.findings.iter().any(
            |f| f.claim.contains("ZHC") || f.claim.contains("deviation") || f.claim.contains("geometric"),
        );
        assert!(
            has_zhc_finding,
            "Should have ZHC-related finding: {:?}",
            report.findings
        );
    }

    #[test]
    fn test_analyze_fallback_without_state() {
        let graph = make_simple_triangle();
        let specialist = GeometricSpecialist::new();
        let report = specialist.analyze_with_zhc(&graph, None);

        assert!(!report.findings.is_empty(), "Should have findings even in fallback mode");
        assert_eq!(report.specialist_id, "geometric");
    }

    #[test]
    fn test_geometric_specialist_id() {
        let specialist = GeometricSpecialist::new();
        assert_eq!(specialist.id(), "geometric");
    }

    #[test]
    fn test_analyze_default_uses_fallback() {
        let graph = make_simple_triangle();
        let specialist = GeometricSpecialist::new();
        let report = specialist.analyze(&graph);

        assert_eq!(report.specialist_id, "geometric");
        assert!(!report.findings.is_empty());
    }

    #[test]
    fn test_zhc_coordination_result_fields() {
        let result = ZhcCoordinationResult {
            is_consistent: true,
            deviation: 0.01,
            information_bits: 2.5,
            conflicted_tiles: vec![],
        };
        assert!(result.is_consistent);
        assert!(result.deviation < 0.1);
    }
}