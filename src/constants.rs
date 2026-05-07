//! Agent constants for bilateral constant-matching
//!
//! Constants define when an agent should be activated for a task.
//! They represent fixed criteria — not the task itself.

use serde::{Deserialize, Serialize};

/// Agent constants — fixed criteria for fleet agent activation
///
/// These are the "constants" in bilateral constant-matching:
/// an agent's fixed criteria that determine when it should
/// be activated for a task or decision point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConstants {
    /// H1 emergence threshold — above this β₁, topological specialist needed
    pub beta_threshold: f64,
    
    /// ZHC loop sum tolerance — above this residual, geometric specialist needed
    pub zhc_tolerance: f64,
    
    /// Minimum trust neighbors for valid analysis
    pub min_neighbors: usize,
    
    /// Pythagorean48 precision threshold
    pub trust_vector_precision: f64,
    
    /// Early warning lead time for H1 emergence (seconds)
    pub h1_emergency_lead_s: f64,
    
    /// Seconds between rigidity checks
    pub rigidity_check_interval: u64,
}

impl AgentConstants {
    /// Default fleet constants
    pub fn default_fleet() -> Self {
        Self {
            beta_threshold: 0.05,
            zhc_tolerance: 0.01,
            min_neighbors: 3,
            trust_vector_precision: 0.1,
            h1_emergency_lead_s: 2.7,
            rigidity_check_interval: 1,
        }
    }

    /// Conservative fleet constants (higher thresholds, less triggering)
    pub fn conservative() -> Self {
        Self {
            beta_threshold: 0.10,
            zhc_tolerance: 0.02,
            min_neighbors: 5,
            trust_vector_precision: 0.15,
            h1_emergency_lead_s: 5.0,
            rigidity_check_interval: 2,
        }
    }

    /// Aggressive fleet constants (lower thresholds, more triggering)
    pub fn aggressive() -> Self {
        Self {
            beta_threshold: 0.02,
            zhc_tolerance: 0.005,
            min_neighbors: 2,
            trust_vector_precision: 0.05,
            h1_emergency_lead_s: 1.0,
            rigidity_check_interval: 1,
        }
    }

    /// Check if agent constants are compatible with task requirements
    /// Returns true if this agent should be activated for the given task.
    pub fn matches_task(&self, task: &crate::task::TaskRequirements) -> bool {
        // Agent's threshold must be >= task's requirement (agent is sensitive enough)
        // Agent's tolerance must be >= task's tolerance (agent is precise enough)
        // Agent's neighbors must be <= task's requirement (agent has enough neighbors)
        // Don't match fire drills (urgency >= 0.9 is "everyone respond")
        self.beta_threshold >= task.required_beta_threshold
            && self.zhc_tolerance >= task.required_zhc_tolerance
            && self.min_neighbors <= task.required_neighbors
            && task.urgency < 0.9
    }

    /// Check if these constants are valid (sane ranges)
    pub fn is_valid(&self) -> bool {
        self.beta_threshold > 0.0
            && self.zhc_tolerance > 0.0
            && self.min_neighbors >= 1
            && self.trust_vector_precision > 0.0
            && self.h1_emergency_lead_s > 0.0
            && self.rigidity_check_interval >= 1
    }
}

impl Default for AgentConstants {
    fn default() -> Self {
        Self::default_fleet()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_constants_are_valid() {
        let constants = AgentConstants::default_fleet();
        assert!(constants.is_valid());
    }

    #[test]
    fn test_conservative_constants_are_valid() {
        let constants = AgentConstants::conservative();
        assert!(constants.is_valid());
    }

    #[test]
    fn test_aggressive_constants_are_valid() {
        let constants = AgentConstants::aggressive();
        assert!(constants.is_valid());
    }

    #[test]
    fn test_constants_match_task() {
        let constants = AgentConstants::default_fleet();
        let task = crate::task::TaskRequirements {
            required_beta_threshold: 0.03,
            required_zhc_tolerance: 0.005,
            required_neighbors: 4,
            urgency: 0.5,
        };
        assert!(constants.matches_task(&task));
    }

    #[test]
    fn test_constants_reject_fire_drill() {
        let constants = AgentConstants::default_fleet();
        let fire_drill = crate::task::TaskRequirements {
            required_beta_threshold: 0.0,
            required_zhc_tolerance: 0.0,
            required_neighbors: 0,
            urgency: 0.95, // Fire drill
        };
        assert!(!constants.matches_task(&fire_drill));
    }

    #[test]
    fn test_conservative_rejects_aggressive_task() {
        let constants = AgentConstants::conservative();
        let aggressive_task = crate::task::TaskRequirements {
            required_beta_threshold: 0.15, // Higher than conservative's 0.10
            required_zhc_tolerance: 0.02,
            required_neighbors: 10,
            urgency: 0.3,
        };
        assert!(!constants.matches_task(&aggressive_task));
    }
}
