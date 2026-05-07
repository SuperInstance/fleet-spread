//! fleet-spread: Fleet graph analysis across 5 specialist dimensions with synthesis
//!
//! # Overview
//! fleet-spread v2 implements a library gate architecture:
//! given fleet graph state, SELECT THE ONE CORRECT specialist and run ONLY that one.
//! This is bilateral constant-matching, not MoE-style "run all 5 and reconcile".
//!
//! ## Library Gate Architecture (v2)
//!
//! The library gate selects based on:
//! - V < 3 → Systems (insufficient data)
//! - β₁ = 0 AND rigid → None (stable)
//! - Trust vector noisy → Algebraic
//! - β₁ rising → Topological
//! - ZHC loop degraded → Geometric
//! - Agent count changed → Empirical
//!
//! ## Bilateral Constant-Matching
//!
//! - Agent has fixed criteria (its "constants")
//! - Task appears with its requirements
//! - Agent evaluates: does this task match my constants?
//! - Fleet graph state evaluates: which specialist does this situation need?
//! - No routing server. No auction. No TOP-K blending.
//!
//! # Example
//!
//! ```rust
//! use fleet_spread::{FleetGraph, TrustValue, Vertex, Edge, run_analysis};
//!
//! // Create a small rigid fleet (V=5, E=7)
//! let vertices = (0..5).map(|i| Vertex { id: format!("agent-{}", i), metadata: Default::default() }).collect();
//! let edges = vec![
//!     Edge { from: "agent-0".into(), to: "agent-1".into(), trust: TrustValue::new(0.9, 0.8) },
//!     // ... more edges
//! ];
//! let graph = FleetGraph::new("my-fleet".into(), vertices, edges);
//!
//! // Run full analysis
//! let synthesis = run_analysis(&graph);
//! println!("Synthesis gain: {:.2}", synthesis.synthesis_gain);
//! ```

pub mod graph;
pub mod specialists;
pub mod synthesis;
pub mod plato_tile;
pub mod git_commit;
pub mod quality;
pub mod test_helpers;
pub mod constants;
pub mod task;
pub mod graph_state;
pub mod library_gate;

pub use graph::{FleetGraph, Vertex, Edge, TrustValue};
pub use specialists::{SpecialistReport, Specialist};
pub use synthesis::{SynthesisReport, SynthesisEngine, interpret_synthesis, assess_single_specialist, specialist_passed};
pub use plato_tile::{PlatoTile, TileWriter, format_tiles_markdown};
pub use quality::{QualityReport, QualityAssessment, SingleSpecialistQuality, SpecialistValueReport};
pub use constants::AgentConstants;
pub use task::TaskRequirements;
pub use graph_state::FleetGraphState;
pub use library_gate::LibraryGate;

use specialists::{TopologicalSpecialist, GeometricSpecialist, AlgebraicSpecialist, SystemsSpecialist, EmpiricalSpecialist};


/// Run the full 5-specialist analysis on a fleet graph
pub fn run_analysis(graph: &FleetGraph) -> SynthesisReport {
    let specialists: Vec<Box<dyn Specialist>> = vec![
        Box::new(TopologicalSpecialist::new()),
        Box::new(GeometricSpecialist::new()),
        Box::new(AlgebraicSpecialist::new()),
        Box::new(SystemsSpecialist::new()),
        Box::new(EmpiricalSpecialist::new()),
    ];

    let reports: Vec<SpecialistReport> = specialists
        .into_iter()
        .map(|s| s.analyze(graph))
        .collect();

    let engine = SynthesisEngine::new();
    engine.synthesize(reports)
}

/// Quick analysis with custom specialists
pub fn run_custom_analysis(graph: &FleetGraph, specialist_ids: &[&str]) -> SynthesisReport {
    let mut reports = Vec::new();

    for id in specialist_ids {
        let report = match *id {
            "topological" => TopologicalSpecialist::new().analyze(graph),
            "geometric" => GeometricSpecialist::new().analyze(graph),
            "algebraic" => AlgebraicSpecialist::new().analyze(graph),
            "systems" => SystemsSpecialist::new().analyze(graph),
            "empirical" => EmpiricalSpecialist::new().analyze(graph),
            _ => continue,
        };
        reports.push(report);
    }

    let engine = SynthesisEngine::new();
    engine.synthesize(reports)
}

/// Identify graph type based on structure
pub fn identify_graph_type(graph: &FleetGraph) -> &'static str {
    if graph.components() > 1 {
        "disconnected"
    } else if graph.is_over_constrained() {
        "over-connected"
    } else if graph.is_under_constrained() {
        "under-constrained"
    } else if graph.is_laman_candidate() {
        "rigid"
    } else {
        "unknown"
    }
}
