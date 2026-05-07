//! Tests for library gate selector

use fleet_spread::graph_state::FleetGraphState;
use fleet_spread::library_gate::{LibraryGate, Specialist};
use fleet_spread::task::TaskRequirements;

#[test]
fn test_rigid_fleet_returns_none() {
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
fn test_priority_order_small_graph_first() {
    let gate = LibraryGate::new();
    // Small graph with rising beta — systems should win (priority 1)
    let mut state = FleetGraphState::small_graph();
    state.beta_1 = 10.0;
    assert_eq!(gate.select(&state), Some(Specialist::Systems));
}

#[test]
fn test_priority_order_noisy_trust_before_beta() {
    let gate = LibraryGate::new();
    // Noisy trust AND rising beta — algebraic should win (priority 3 before 4)
    let mut state = FleetGraphState::noisy_trust();
    state.beta_1 = 10.0;
    assert_eq!(gate.select(&state), Some(Specialist::Algebraic));
}
