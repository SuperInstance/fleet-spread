//! Tests for individual specialists

use fleet_spread::*;
use fleet_spread::specialists::*;
use fleet_spread::test_helpers::{make_small_rigid, make_over_connected, make_disconnected};

#[test]
fn test_topological_small_rigid() {
    let specialist = TopologicalSpecialist::new();
    let graph = make_small_rigid();
    let report = specialist.analyze(&graph);

    assert_eq!(report.specialist_id, "topological");
    assert!(report.confidence > 0.6);
    assert!(report.findings.iter().any(|f| f.claim.contains("3") || f.claim.contains("β₁")));
}

#[test]
fn test_topological_disconnected() {
    let specialist = TopologicalSpecialist::new();
    let graph = make_disconnected();
    let report = specialist.analyze(&graph);

    assert!(report.findings.iter().any(|f| f.claim.contains("disconnected")));
    assert!(!report.unanswered.is_empty());
}

#[test]
fn test_geometric_small_rigid() {
    let specialist = GeometricSpecialist::new();
    let graph = make_small_rigid();
    let report = specialist.analyze(&graph);

    assert_eq!(report.specialist_id, "geometric");
    assert!(report.confidence > 0.5);
}

#[test]
fn test_geometric_over_connected() {
    let specialist = GeometricSpecialist::new();
    let graph = make_over_connected();
    let report = specialist.analyze(&graph);

    // Over-connected graphs may show geometric strain
    assert!(!report.findings.is_empty());
}

#[test]
fn test_algebraic_stability() {
    let specialist = AlgebraicSpecialist::new();
    let graph = make_small_rigid();
    let report = specialist.analyze(&graph);

    assert_eq!(report.specialist_id, "algebraic");
    assert!(report.findings.iter().any(|f| f.claim.contains("stable") || f.claim.contains("stability")));
}

#[test]
fn test_systems_laman_rigid() {
    let specialist = SystemsSpecialist::new();
    let graph = make_small_rigid();
    let report = specialist.analyze(&graph);

    assert_eq!(report.specialist_id, "systems");
    assert!(report.findings.iter().any(|f| f.claim.contains("Laman-rigid") || f.claim.contains("rigid")));
}

#[test]
fn test_systems_over_constrained() {
    let specialist = SystemsSpecialist::new();
    let graph = make_over_connected();
    let report = specialist.analyze(&graph);

    assert!(report.findings.iter().any(|f| f.claim.contains("over-constrained")));
}

#[test]
fn test_empirical_distribution() {
    let specialist = EmpiricalSpecialist::new();
    let graph = make_small_rigid();
    let report = specialist.analyze(&graph);

    assert_eq!(report.specialist_id, "empirical");
    assert!(report.findings.iter().any(|f| f.claim.contains("distribution") || f.claim.contains("Trust")));
}

#[test]
fn test_all_specialists_on_all_graphs() {
    let graphs = vec![
        ("small-rigid", make_small_rigid()),
        ("over-connected", make_over_connected()),
        ("disconnected", make_disconnected()),
    ];

    let specialists: Vec<(&str, Box<dyn Specialist>)> = vec![
        ("topological", Box::new(TopologicalSpecialist::new())),
        ("geometric", Box::new(GeometricSpecialist::new())),
        ("algebraic", Box::new(AlgebraicSpecialist::new())),
        ("systems", Box::new(SystemsSpecialist::new())),
        ("empirical", Box::new(EmpiricalSpecialist::new())),
    ];

    for (graph_name, graph) in graphs {
        for (spec_name, specialist) in &specialists {
            let report = specialist.analyze(&graph);
            assert!(
                report.confidence > 0.0,
                "{} specialist failed on {} graph",
                spec_name,
                graph_name
            );
        }
    }
}
