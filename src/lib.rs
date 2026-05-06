//! fleet-spread: Fleet graph analysis across 5 specialist dimensions with synthesis
//!
//! # Overview
//! fleet-spread fans out a single fleet graph across 5 specialist analysis dimensions,
//! producing constraint tiles and a unified synthesis. The synthesis layer identifies
//! where specialists agree, where they disagree, and what's missing from all 5.
//!
//! # The 5 Specialist Dimensions
//!
//! - **S1: Topological** - Betti numbers, cycle basis, connected components
//! - **S2: Geometric** - ZHC closure, holonomy, stress detection
//! - **S3: Algebraic** - Pythagorean48 encoding, drift analysis
//! - **S4: Systems** - Laman rigidity, constraint analysis
//! - **S5: Empirical** - Trust anomaly detection, drift detection
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

pub use graph::{FleetGraph, Vertex, Edge, TrustValue};
pub use specialists::{SpecialistReport, Specialist};
pub use synthesis::{SynthesisReport, SynthesisEngine, interpret_synthesis};
pub use plato_tile::{PlatoTile, TileWriter, format_tiles_markdown};
pub use quality::{QualityReport, QualityAssessment};

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
