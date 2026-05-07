//! fleet-coordinate bridge — wires fleet-coordinate math to fleet-spread captain
//!
//! This is the "low-level to high-level" integration layer that unifies:
//! - fleet-coordinate: rigorous mathematical results (Laman rigidity, H¹ cohomology, ZHC, Pythagorean48)
//! - fleet-spread: captain inquiry engine + 5 specialist dimensions

use crate::captain::{Captain, CaptainDecision, CaptainDeliberation};
use crate::graph::{FleetGraph as SpreadGraph, Vertex as SpreadVertex, Edge as SpreadEdge, TrustValue as SpreadTrust};
use crate::graph_state::FleetGraphState;
use crate::library_gate::{LibraryGate, Specialist};
use crate::specialists::Specialist as _;

/// Bridge between fleet-coordinate's FleetGraph and fleet-spread's FleetGraph
pub struct CoordinateBridge {
    captain: Captain,
    gate: LibraryGate,
}

impl CoordinateBridge {
    pub fn new() -> Self {
        Self {
            captain: Captain::new(),
            gate: LibraryGate::new(),
        }
    }

    /// Convert fleet-coordinate's FleetGraph to fleet-spread's FleetGraph
    ///
    /// Uses public API only: V(), get_neighbors(id) — never accesses private fields
    pub fn convert_from_coordinate(cg: &fleet_coordinate::graph::FleetGraph) -> SpreadGraph {
        // Build vertex list from agent IDs (we know the range 0..V)
        let agent_ids: Vec<u64> = (0..cg.V() as u64).collect();

        let vertices: Vec<SpreadVertex> = agent_ids.iter().map(|&id| {
            SpreadVertex {
                id: format!("agent-{}", id),
                metadata: Default::default(),
            }
        }).collect();

        // Build edges from adjacency using get_neighbors public API
        let mut edge_list = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for &id in &agent_ids {
            let neighbors = cg.get_neighbors(id);
            for &neighbor_id in &neighbors {
                if neighbor_id > id { // Only add once per undirected edge
                    let key = (id, neighbor_id);
                    if seen.insert(key) {
                        edge_list.push(SpreadEdge {
                            from: format!("agent-{}", id),
                            to: format!("agent-{}", neighbor_id),
                            trust: SpreadTrust::new(0.8, 0.9), // Default trust
                        });
                    }
                }
            }
        }

        SpreadGraph::new("coordinated-fleet".to_string(), vertices, edge_list)
    }

    /// Build FleetGraphState from fleet-coordinate's graph + rigidity result
    pub fn build_state(
        cg: &fleet_coordinate::graph::FleetGraph,
        rigidity: &fleet_coordinate::graph::RigidityResult,
        zhc_residual: f64,
        trust_entropy: f64,
    ) -> FleetGraphState {
        FleetGraphState {
            V: cg.V(),
            E: cg.E(),
            beta_1: rigidity.h1_dimension as f64,
            zhc_loop_residual: zhc_residual,
            trust_vector_entropy: trust_entropy,
            agent_count: cg.V(),
            last_change_s: 0.0,
            is_connected: rigidity.h1_dimension == 0 || cg.V() >= 3,
        }
    }

    /// Full analysis: coordinate graph → captain decision
    pub fn analyze(&self, cg: &fleet_coordinate::graph::FleetGraph) -> CoordinateAnalysis {
        let spread_graph = Self::convert_from_coordinate(cg);
        let rigidity = cg.check_laman_rigidity();
        let state = Self::build_state(cg, &rigidity, 0.005, 0.1);
        let signal_sources = self.gate.all_with_signal(&state);
        let decision = self.captain.deliberate(&state, &spread_graph);

        CoordinateAnalysis {
            rigidity,
            state,
            signal_sources,
            decision,
        }
    }

    /// Quick consistency check using only fleet-coordinate math (no captain inquiry)
    pub fn quick_check(cg: &fleet_coordinate::graph::FleetGraph) -> QuickConsistencyResult {
        let rigidity = cg.check_laman_rigidity();
        let emergence = fleet_coordinate::emergence::EmergenceDetector::detect(cg.V(), cg.E(), 1);

        QuickConsistencyResult {
            is_rigid: rigidity.is_rigid,
            emergence_detected: emergence.emergence_detected,
            beta_1: rigidity.h1_dimension,
            max_neighbors: rigidity.max_neighbors,
        }
    }
}

impl Default for CoordinateBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of coordinate bridge analysis
#[derive(Debug, Clone)]
pub struct CoordinateAnalysis {
    pub rigidity: fleet_coordinate::graph::RigidityResult,
    pub state: FleetGraphState,
    pub signal_sources: Vec<Specialist>,
    pub decision: CaptainDecision,
}

/// Quick consistency check (coordinate math only, no captain)
#[derive(Debug, Clone)]
pub struct QuickConsistencyResult {
    pub is_rigid: bool,
    pub emergence_detected: bool,
    pub beta_1: usize,
    pub max_neighbors: usize,
}

/// Extension trait: add fleet-coordinate analysis to fleet-spread's Captain
pub trait CaptainCoordinateExt {
    fn inquire_coordinate(&self, cg: &fleet_coordinate::graph::FleetGraph) -> CaptainDeliberation;
}

impl CaptainCoordinateExt for Captain {
    fn inquire_coordinate(&self, cg: &fleet_coordinate::graph::FleetGraph) -> CaptainDeliberation {
        let bridge = CoordinateBridge::new();
        let spread_graph = CoordinateBridge::convert_from_coordinate(cg);
        let rigidity = cg.check_laman_rigidity();
        let state = CoordinateBridge::build_state(cg, &rigidity, 0.005, 0.1);
        self.inquire(&state, &spread_graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fleet_coordinate::graph::FleetGraph;

    #[test]
    fn test_coordinate_bridge_analyze() {
        let mut cg = FleetGraph::new();
        cg.add_agent(1, [0.0, 0.0], vec![]);
        cg.add_agent(2, [1.0, 0.0], vec![]);
        cg.add_agent(3, [0.5, 0.87], vec![]);
        cg.add_edge(1, 2);
        cg.add_edge(2, 3);
        cg.add_edge(3, 1);

        let bridge = CoordinateBridge::new();
        let result = bridge.analyze(&cg);

        assert!(result.rigidity.is_rigid);
        assert_eq!(result.rigidity.h1_dimension, 1);
    }

    #[test]
    fn test_quick_check() {
        let mut cg = FleetGraph::new();
        cg.add_agent(1, [0.0, 0.0], vec![]);
        cg.add_agent(2, [1.0, 0.0], vec![]);
        cg.add_agent(3, [0.5, 0.87], vec![]);
        cg.add_edge(1, 2);
        cg.add_edge(2, 3);
        cg.add_edge(3, 1);

        let result = CoordinateBridge::quick_check(&cg);
        assert!(result.is_rigid);
        assert!(!result.emergence_detected);
        assert_eq!(result.beta_1, 1);
    }

    #[test]
    fn test_convert_from_coordinate() {
        let mut cg = FleetGraph::new();
        cg.add_agent(1, [0.0, 0.0], vec![]);
        cg.add_agent(2, [1.0, 0.0], vec![]);
        cg.add_edge(1, 2);

        let spread = CoordinateBridge::convert_from_coordinate(&cg);
        assert_eq!(spread.v(), 2);
        assert_eq!(spread.e(), 1);
    }

    #[test]
    fn test_captain_inquire_coordinate() {
        let mut cg = FleetGraph::new();
        for i in 1..=5 {
            cg.add_agent(i, [i as f64, 0.0], vec![]);
        }
        cg.add_edge(1, 2);
        cg.add_edge(2, 3);
        cg.add_edge(3, 1);
        cg.add_edge(1, 4);
        cg.add_edge(2, 5);
        cg.add_edge(3, 4);
        cg.add_edge(4, 5);

        let captain = Captain::new();
        let deliberation = captain.inquire_coordinate(&cg);
        assert!(!deliberation.consulted.is_empty());
    }
}
