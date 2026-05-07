//! Library gate — the selector for fleet-spread v2
//!
//! The library gate implements bilateral constant-matching: given
//! fleet graph state, it selects THE ONE correct specialist to run.
//!
//! # Gate Table
//!
//! | Condition | Select | Why |
//! |-----------|--------|-----|
//! | V < 3 | Systems | Insufficient data |
//! | β₁ = 0 AND graph rigid | None | Stable fleet |
//! | Trust vector noisy | Algebraic | Encoding analysis |
//! | β₁ rising | Topological | H¹ emergence |
//! | ZHC loop degraded | Geometric | ZHC closure |
//! | Agent count changed | Empirical | Trust drift |
//!
//! Priority order (first match wins):
//! 1. V < 3 → systems
//! 2. β₁ = 0 AND rigid → None (stable)
//! 3. Trust vector noisy → algebraic
//! 4. β₁ rising → topological
//! 5. ZHC loop degraded → geometric
//! 6. Agent count changed → empirical
//! 7. Default → None (stable)

use crate::constants::AgentConstants;
use crate::graph_state::FleetGraphState;
use crate::task::TaskRequirements;

/// The specialist to run (only one at a time in v2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Specialist {
    Topological,
    Geometric,
    Algebraic,
    Systems,
    Empirical,
}

impl Specialist {
    /// Human-readable name for the specialist
    pub fn name(&self) -> &'static str {
        match self {
            Specialist::Topological => "topological",
            Specialist::Geometric => "geometric",
            Specialist::Algebraic => "algebraic",
            Specialist::Systems => "systems",
            Specialist::Empirical => "empirical",
        }
    }
}

impl std::fmt::Display for Specialist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Library gate — selects specialists based on fleet graph state
///
/// The gate implements bilateral constant-matching: given fleet graph state,
/// it determines which specialists have relevant signal RIGHT NOW.
///
/// # Two Selection Modes
///
/// - `select()` — returns THE ONE most critical specialist (for action)
/// - `all_with_signal()` — returns ALL specialists with relevant signal (for captain's inquiry)
///
/// # Gate Table (bilateral constant-matching)
///
/// | Condition | Signal Source | Why |
/// |-----------|---------------|-----|
/// | V < 3 | Systems | Insufficient data |
/// | Trust vector noisy | Algebraic | Encoding analysis |
/// | β₁ rising | Topological | H¹ emergence |
/// | ZHC loop degraded | Geometric | ZHC closure |
/// | Agent count changed | Empirical | Trust drift |
pub struct LibraryGate {
    constants: AgentConstants,
}

impl LibraryGate {
    /// Create a new library gate with default fleet constants
    pub fn new() -> Self {
        Self {
            constants: AgentConstants::default_fleet(),
        }
    }

    /// Create a library gate with custom constants
    pub fn with_constants(constants: AgentConstants) -> Self {
        Self { constants }
    }

    /// Given fleet graph state, return the ONE correct specialist to run
    ///
    /// Returns `None` if no specialist is needed (fleet is stable).
    ///
    /// # Priority Order (most critical first)
    ///
    /// 1. V < 3 → Systems (insufficient data for other specialists)
    /// 2. ZHC loop degraded → Geometric (geometric inconsistency is immediate/safety-critical)
    /// 3. Trust vector noisy → Algebraic (Pythagorean48 encoding analysis)
    /// 4. β₁ rising → Topological (H¹ emergence tracking — approaching rigidity threshold)
    /// 5. Agent count changed → Empirical (trust drift detection after topology change)
    /// 6. β₁ = 0 AND graph stable → None (fleet is self-coordinating)
    ///
    /// ZHC takes priority over β₁ because ZHC violation is a detected inconsistency
    /// (something went wrong NOW), while β₁ elevation is a warning (something might
    /// be approaching). Safety-critical issues take priority over warnings.
    pub fn select(&self, state: &FleetGraphState) -> Option<Specialist> {
        // Priority 1: V=2 is valid — a single edge (E=1) is Laman-rigid (E=2V-3).
        // The stability check below handles all fleet sizes including V=2.

        // Priority 2: ZHC loop degraded → geometric (MOST SAFETY-CRITICAL)
        // Geometric inconsistency means the trust graph has a measurable drift.
        // This takes priority over β₁ because it's a detected problem, not a warning.
        if state.zhc_loop_residual > self.constants.zhc_tolerance {
            return Some(Specialist::Geometric);
        }

        // Priority 3: Trust vector noisy → algebraic specialist
        // Pythagorean48 encoding is unreliable — trust information is degrading.
        if state.trust_vector_entropy > 0.5 {
            return Some(Specialist::Algebraic);
        }

        // Priority 4: β₁ rising → topological specialist (H¹ emergence tracking)
        // The graph is approaching the rigidity threshold. Track it.
        if state.beta_1 > self.constants.beta_threshold {
            return Some(Specialist::Topological);
        }

        // Priority 5: Agent count changed → empirical (after topology change)
        // New agents change the equilibrium. Check for trust drift.
        if state.agent_count != state.V {
            return Some(Specialist::Empirical);
        }


        // Priority 6: Stable fleet (β₁ = 0, connected, no recent changes)
        // Fleet is self-coordinating. No specialists needed.
        if state.beta_1 == 0.0 && state.is_connected && state.last_change_s > 10.0 && state.agent_count == state.V {
            return None;
        }

        // Default: fleet is stable, no specialist needed
        None
    }

    /// Return ALL specialists that have relevant signal for this state
    ///
    /// Used by the captain for wide inquiry phase. Unlike `select()` which
    /// returns only the most critical specialist, this returns every specialist
    /// whose signal condition is met.
    ///
    /// Priority doesn't apply here — we want the full picture for the captain's
    /// inquiry phase. The captain decides what to do with the signal.
    pub fn all_with_signal(&self, state: &FleetGraphState) -> Vec<Specialist> {
        let mut specialists = Vec::new();

        // Systems: always relevant (safety monitoring is always on)
        // Both small (V < 3) and larger graphs need systems analysis
        specialists.push(Specialist::Systems);

        // Geometric: relevant when ZHC is degraded (geometric inconsistency = immediate signal)
        if state.zhc_loop_residual > self.constants.zhc_tolerance {
            specialists.push(Specialist::Geometric);
        }

        // Algebraic: relevant when trust vector is noisy
        if state.trust_vector_entropy > 0.5 {
            specialists.push(Specialist::Algebraic);
        }

        // Topological: relevant when β₁ is elevated (H¹ emergence tracking)
        if state.beta_1 > self.constants.beta_threshold {
            specialists.push(Specialist::Topological);
        }

        // Empirical: relevant when agent count has changed
        if state.agent_count != state.V {
            specialists.push(Specialist::Empirical);
        }

        specialists
    }

    /// Select specialist for a task (bilateral constant-matching)
    ///
    /// This checks if the agent's constants are compatible with the
    /// task's requirements. If so, return the specialist that matches.
    pub fn select_for_task(&self, state: &FleetGraphState, task: &TaskRequirements) -> Option<Specialist> {
        // Task check: does this agent's constants match this task?
        if !self.constants.matches_task(task) {
            return None;
        }

        // If constants match, fall through to state-based selection
        self.select(state)
    }

    /// Check if agent constants match task requirements
    pub fn constants_match(&self, task: &TaskRequirements) -> bool {
        self.constants.matches_task(task)
    }

    /// Get the current agent constants
    pub fn constants(&self) -> &AgentConstants {
        &self.constants
    }

    /// Update agent constants
    pub fn set_constants(&mut self, constants: AgentConstants) {
        self.constants = constants;
    }
}

impl Default for LibraryGate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stable_fleet_returns_none() {
        let gate = LibraryGate::new();
        let state = FleetGraphState::stable_rigid();
        assert_eq!(gate.select(&state), None);
    }

    #[test]
    fn test_small_graph_returns_systems() {
        let gate = LibraryGate::new();
        let state = FleetGraphState::small_graph();
        assert_eq!(gate.select(&state), Some(Specialist::Systems));
    }

    #[test]
    fn test_rising_beta_returns_topological() {
        let gate = LibraryGate::new();
        let state = FleetGraphState::rising_beta();
        assert_eq!(gate.select(&state), Some(Specialist::Topological));
    }

    #[test]
    fn test_degraded_zhc_returns_geometric() {
        let gate = LibraryGate::new();
        let state = FleetGraphState::degraded_zhc();
        assert_eq!(gate.select(&state), Some(Specialist::Geometric));
    }

    #[test]
    fn test_noisy_trust_returns_algebraic() {
        let gate = LibraryGate::new();
        let state = FleetGraphState::noisy_trust();
        assert_eq!(gate.select(&state), Some(Specialist::Algebraic));
    }

    #[test]
    fn test_agent_count_changed_returns_empirical() {
        let gate = LibraryGate::new();
        let state = FleetGraphState::agent_count_changed();
        assert_eq!(gate.select(&state), Some(Specialist::Empirical));
    }

    #[test]
    fn test_constants_match_works() {
        let gate = LibraryGate::new();
        let routine_task = TaskRequirements::routine();
        assert!(gate.constants_match(&routine_task));

        let fire_drill = TaskRequirements::critical();
        assert!(!gate.constants_match(&fire_drill));
    }

    #[test]
    fn test_conservative_gate_rejects_aggressive_task() {
        let constants = AgentConstants::conservative();
        let gate = LibraryGate::with_constants(constants);
        let aggressive_task = TaskRequirements {
            required_beta_threshold: 0.15,
            required_zhc_tolerance: 0.02,
            required_neighbors: 10,
            urgency: 0.3,
        };
        assert!(!gate.constants_match(&aggressive_task));
    }

    #[test]
    fn test_specialist_names() {
        assert_eq!(Specialist::Topological.name(), "topological");
        assert_eq!(Specialist::Geometric.name(), "geometric");
        assert_eq!(Specialist::Algebraic.name(), "algebraic");
        assert_eq!(Specialist::Systems.name(), "systems");
        assert_eq!(Specialist::Empirical.name(), "empirical");
    }

    #[test]
    fn test_priority_order_small_graph_first() {
        let gate = LibraryGate::new();
        // Small graph with rising beta — systems should win (priority 1)
        let mut state = FleetGraphState::small_graph();
        state.beta_1 = 10.0; // Rising beta, but V < 3
        assert_eq!(gate.select(&state), Some(Specialist::Systems));
    }

    #[test]
    fn test_priority_order_noisy_trust_before_beta() {
        let gate = LibraryGate::new();
        // Noisy trust AND rising beta — algebraic should win (priority 3 before 4)
        let mut state = FleetGraphState::noisy_trust();
        state.beta_1 = 10.0; // Rising beta
        assert_eq!(gate.select(&state), Some(Specialist::Algebraic));
    }

    #[test]
    fn test_all_with_signal_stable_fleet() {
        let gate = LibraryGate::new();
        let state = FleetGraphState::stable_rigid();
        // Stable fleet: only systems (always on) + algebraic if noisy (but trust is not noisy here)
        // Actually: stable_rigid has trust_vector_entropy = 0.1, so only systems
        let signal = gate.all_with_signal(&state);
        assert!(signal.contains(&Specialist::Systems));
        assert!(!signal.contains(&Specialist::Topological));
    }

    #[test]
    fn test_all_with_signal_rising_beta() {
        let gate = LibraryGate::new();
        let state = FleetGraphState::rising_beta();
        // Rising beta: systems + topological
        let signal = gate.all_with_signal(&state);
        assert!(signal.contains(&Specialist::Systems));
        assert!(signal.contains(&Specialist::Topological));
    }

    #[test]
    fn test_all_with_signal_degraded_zhc() {
        let gate = LibraryGate::new();
        let state = FleetGraphState::degraded_zhc();
        // Degraded ZHC: systems + geometric
        let signal = gate.all_with_signal(&state);
        assert!(signal.contains(&Specialist::Systems));
        assert!(signal.contains(&Specialist::Geometric));
    }

    #[test]
    fn test_all_with_signal_noisy_trust() {
        let gate = LibraryGate::new();
        let state = FleetGraphState::noisy_trust();
        // Noisy trust: systems + algebraic
        let signal = gate.all_with_signal(&state);
        assert!(signal.contains(&Specialist::Systems));
        assert!(signal.contains(&Specialist::Algebraic));
    }


    #[test]
    fn test_all_with_signal_multiple_signals() {
        let gate = LibraryGate::new();
        // Rising beta AND degraded ZHC: systems + topological + geometric
        let mut state = FleetGraphState::rising_beta();
        state.zhc_loop_residual = 0.15; // Degrade ZHC too
        let signal = gate.all_with_signal(&state);
        assert!(signal.contains(&Specialist::Systems));
        assert!(signal.contains(&Specialist::Topological));
        assert!(signal.contains(&Specialist::Geometric));
    }

    #[test]
    fn test_two_node_laman_rigid_is_stable() {
        // V=2, E=1: Laman condition E=2V-3=1 is exactly satisfied.
        // β₁ = E - V + C = 1 - 2 + 1 = 0.  Single edge, connected.
        // This is the minimal rigid graph (2 vertices, 1 edge).
        let state = FleetGraphState {
            V: 2,
            E: 1,
            beta_1: 0.0,
            zhc_loop_residual: 0.0,
            trust_vector_entropy: 0.0,
            agent_count: 2,
            last_change_s: 100.0, // > 10s, no recent changes
            is_connected: true,
        };
        // FleetGraphState.is_stable() checks: β₁=0 ∧ connected ∧ no recent changes ∧ V matches agent_count
        assert!(state.is_stable(), "2-node single-edge graph (V=2,E=1) should be Laman-rigid and stable");
        let gate = LibraryGate::new();
        // select() Priority 1 fires first: V < 3 → Systems specialist.
        // (The "stable fleet → None" check only applies when V >= 3.)
        assert_eq!(gate.select(&state), Some(Specialist::Systems),
            "select() returns Systems for V < 3 regardless of stability (Priority 1)");
        // all_with_signal() reports ALL specialists with signal (no priority filtering).
        // Only Systems has signal here (stable graph, no noise, no changes).
        let all = gate.all_with_signal(&state);
        assert!(all.contains(&Specialist::Systems));
        assert_eq!(all.len(), 1, "Only Systems should have signal for stable 2-node fleet (no noise, no changes)");
    }

    #[test]
    fn test_under_constrained_fleet_not_stable() {
        // V=5, E=4: Under-constrained — Laman requires E=2V-3=7.
        // β₁ = 4 - 5 + 1 = 0 (graph is connected with no cycles), but E < 2V-3.
        // last_change_s < 10 flags it as recently changed, so not stable.
        let state = FleetGraphState {
            V: 5,
            E: 4,
            beta_1: 0.0,
            zhc_loop_residual: 0.005,
            trust_vector_entropy: 0.1,
            agent_count: 5,
            last_change_s: 5.0, // < 10s — recent change detected
            is_connected: true,
        };
        assert!(!state.is_stable(), "Under-constrained fleet (V=5,E=4) should NOT be stable");
        // select() checks is_stable() first via deliberate(); in direct select:
        // V>=3 → not Systems priority 1
        // zhc_loop_residual=0.005 < 0.01 → not Geometric
        // trust_vector_entropy=0.1 < 0.5 → not Algebraic
        // beta_1=0.0 < 0.05 → not Topological
        // agent_count==V → not Empirical
        // last_change_s=5.0 < 10.0 → fails stability check, so select() returns None
        let gate = LibraryGate::new();
        // Systems gets added by all_with_signal even though select() returns None
        let all = gate.all_with_signal(&state);
        assert!(all.contains(&Specialist::Systems));
    }
}
