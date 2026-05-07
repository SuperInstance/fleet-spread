//! Tests for agent constants (bilateral constant-matching)

use fleet_spread::constants::AgentConstants;
use fleet_spread::task::TaskRequirements;

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
    let task = TaskRequirements {
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
    let fire_drill = TaskRequirements {
        required_beta_threshold: 0.0,
        required_zhc_tolerance: 0.0,
        required_neighbors: 0,
        urgency: 0.95, // Fire drill
    };
    assert!(!constants.matches_task(&fire_drill));
}
