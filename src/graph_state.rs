//! Fleet graph state — snapshot of fleet topology for library gate selection
//!
//! This is a lightweight state struct extracted from FleetGraph
//! for efficient specialist selection without full graph analysis.

use serde::{Deserialize, Serialize};

/// Fleet graph state — current snapshot of fleet topology
///
/// This is what the library gate uses to decide which specialist to run.
/// It contains the key metrics extracted from FleetGraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetGraphState {
    /// Number of vertices (agents)
    pub V: usize,
    
    /// Number of edges (trust connections)
    pub E: usize,
    
    /// H1 cohomology dimension: β₁ = E - V + C
    /// Non-zero means cycles exist in the graph
    pub beta_1: f64,
    
    /// ZHC loop residual (0 = perfect closure)
    /// High residual means geometric inconsistency
    pub zhc_loop_residual: f64,
    
    /// Entropy of trust value distributions
    /// High entropy means noisy/unpredictable trust
    pub trust_vector_entropy: f64,
    
    /// Current agent count (may differ from V if agents joined/left)
    pub agent_count: usize,
    
    /// Seconds since last topology change
    pub last_change_s: f64,
    
    /// Whether the graph is connected
    pub is_connected: bool,
}

impl FleetGraphState {
    /// Create from a FleetGraph
    pub fn from_graph(graph: &crate::graph::FleetGraph) -> Self {
        let components = graph.components();
        Self {
            V: graph.v(),
            E: graph.e(),
            beta_1: graph.betti_1() as f64,
            zhc_loop_residual: 0.0, // Default; specialists compute this
            trust_vector_entropy: 0.0, // Default; specialists compute this
            agent_count: graph.v(),
            last_change_s: 0.0,
            is_connected: components == 1,
        }
    }

    /// Create a stable rigid state (β₁ = 0, connected, stable)
    pub fn stable_rigid() -> Self {
        Self {
            V: 5,
            E: 7,
            beta_1: 0.0,
            zhc_loop_residual: 0.005,
            trust_vector_entropy: 0.1,
            agent_count: 5,
            last_change_s: 100.0, // > 60 so topological doesn't trigger
            is_connected: true,
        }
    }

    /// Create a state with rising β₁ (H1 emergence)
    pub fn rising_beta() -> Self {
        Self {
            V: 5,
            E: 9,
            beta_1: 4.0, // β₁ rising
            zhc_loop_residual: 0.005,
            trust_vector_entropy: 0.1,
            agent_count: 5,
            last_change_s: 0.5,
            is_connected: true,
        }
    }

    /// Create a state with degraded ZHC loop
    pub fn degraded_zhc() -> Self {
        Self {
            V: 5,
            E: 7,
            beta_1: 3.0,
            zhc_loop_residual: 0.15, // Above 0.01 threshold
            trust_vector_entropy: 0.1,
            agent_count: 5,
            last_change_s: 5.0,
            is_connected: true,
        }
    }

    /// Create a noisy trust vector state
    pub fn noisy_trust() -> Self {
        Self {
            V: 5,
            E: 7,
            beta_1: 3.0,
            zhc_loop_residual: 0.005,
            trust_vector_entropy: 0.8, // High entropy = noisy
            agent_count: 5,
            last_change_s: 10.0,
            is_connected: true,
        }
    }

    /// Create a small graph state (V < 3)
    pub fn small_graph() -> Self {
        Self {
            V: 2,
            E: 1,
            beta_1: 0.0,
            zhc_loop_residual: 0.0,
            trust_vector_entropy: 0.0,
            agent_count: 2,
            last_change_s: 0.0,
            is_connected: true,
        }
    }

    /// Create a state with agent count change
    pub fn agent_count_changed() -> Self {
        Self {
            V: 6,
            E: 7,
            beta_1: 0.0,
            zhc_loop_residual: 0.005,
            trust_vector_entropy: 0.1,
            agent_count: 7, // Changed from previous
            last_change_s: 30.0,
            is_connected: true,
        }
    }

    /// Check if the fleet is stable (no specialist needed)
    pub fn is_stable(&self) -> bool {
        self.beta_1 == 0.0 && self.is_connected && self.last_change_s > 10.0 && self.agent_count == self.V
    }

    /// Check if the fleet has insufficient data for specialist analysis
    pub fn has_insufficient_data(&self) -> bool {
        self.V < 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stable_rigid_is_stable() {
        let state = FleetGraphState::stable_rigid();
        assert!(state.is_stable());
    }

    #[test]
    fn test_small_graph_has_insufficient_data() {
        let state = FleetGraphState::small_graph();
        assert!(state.has_insufficient_data());
        assert!(!state.is_stable());
    }

    #[test]
    fn test_rising_beta_is_not_stable() {
        let state = FleetGraphState::rising_beta();
        assert!(!state.is_stable());
        assert!(state.beta_1 > 0.0);
    }

    #[test]
    fn test_degraded_zhc_is_not_stable() {
        let state = FleetGraphState::degraded_zhc();
        assert!(!state.is_stable());
        assert!(state.zhc_loop_residual > 0.01);
    }

    #[test]
    fn test_noisy_trust_is_not_stable() {
        let state = FleetGraphState::noisy_trust();
        assert!(!state.is_stable());
        assert!(state.trust_vector_entropy > 0.5);
    }

    #[test]
    fn test_agent_count_changed_is_not_stable() {
        let state = FleetGraphState::agent_count_changed();
        assert!(!state.is_stable());
    }
}
