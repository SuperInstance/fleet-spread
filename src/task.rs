//! Task requirements for bilateral constant-matching
//!
//! When a task appears, it carries requirements. The library gate
//! checks if agent constants match those requirements.

use serde::{Deserialize, Serialize};

/// Task requirements — what a task needs from an agent
///
/// In bilateral constant-matching:
/// - Task has fixed requirements (what it needs)
/// - Agent has fixed constants (what it can handle)
/// - Match check: are they compatible?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequirements {
    /// Required H1 emergence threshold sensitivity
    pub required_beta_threshold: f64,
    
    /// Required ZHC loop tolerance
    pub required_zhc_tolerance: f64,
    
    /// Required minimum trust neighbors
    pub required_neighbors: usize,
    
    /// Task urgency (0.0 = routine, 1.0 = critical)
    /// Fire drills (urgency >= 0.9) match all agents
    pub urgency: f64,
}

impl TaskRequirements {
    /// Create a routine task with default requirements
    pub fn routine() -> Self {
        Self {
            required_beta_threshold: 0.05,
            required_zhc_tolerance: 0.01,
            required_neighbors: 3,
            urgency: 0.3,
        }
    }

    /// Create an urgent task
    pub fn urgent() -> Self {
        Self {
            required_beta_threshold: 0.05,
            required_zhc_tolerance: 0.01,
            required_neighbors: 3,
            urgency: 0.8,
        }
    }

    /// Create a critical task (fire drill)
    pub fn critical() -> Self {
        Self {
            required_beta_threshold: 0.0,
            required_zhc_tolerance: 0.0,
            required_neighbors: 0,
            urgency: 1.0,
        }
    }

    /// Check if this task is a fire drill (everyone responds)
    pub fn is_fire_drill(&self) -> bool {
        self.urgency >= 0.9
    }

    /// Check if this task has sane requirements
    pub fn is_valid(&self) -> bool {
        self.urgency >= 0.0 && self.urgency <= 1.0
    }
}

impl Default for TaskRequirements {
    fn default() -> Self {
        Self::routine()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routine_task_is_valid() {
        let task = TaskRequirements::routine();
        assert!(task.is_valid());
        assert!(!task.is_fire_drill());
    }

    #[test]
    fn test_critical_task_is_fire_drill() {
        let task = TaskRequirements::critical();
        assert!(task.is_valid());
        assert!(task.is_fire_drill());
    }

    #[test]
    fn test_urgency_bounds() {
        let mut task = TaskRequirements::routine();
        task.urgency = 1.5;
        assert!(!task.is_valid());
        
        task.urgency = -0.1;
        assert!(!task.is_valid());
    }
}
