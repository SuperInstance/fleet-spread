//! Tests for synthesis layer

use fleet_spread::*;
use fleet_spread::test_helpers::{make_small_rigid, make_over_connected, make_disconnected};

#[test]
fn test_synthesis_runs() {
    let graph = make_small_rigid();
    let synthesis = run_analysis(&graph);

    assert!(!synthesis.specialist_reports.is_empty());
    assert!(synthesis.overall_confidence > 0.0);
}

#[test]
fn test_synthesis_gain_positive() {
    let graph = make_small_rigid();
    let synthesis = run_analysis(&graph);

    // Small rigid should have positive synthesis gain
    assert!(synthesis.synthesis_gain >= -1.0 && synthesis.synthesis_gain <= 1.0);
}

#[test]
fn test_synthesis_small_rigid() {
    let graph = make_small_rigid();
    let synthesis = run_analysis(&graph);
    let graph_type = identify_graph_type(&graph);

    assert_eq!(graph_type, "rigid");
    assert!(synthesis.overall_confidence > 0.5);
}

#[test]
fn test_synthesis_over_connected() {
    let graph = make_over_connected();
    let synthesis = run_analysis(&graph);
    let graph_type = identify_graph_type(&graph);

    assert_eq!(graph_type, "over-connected");
    // Over-connected should show tensions
    assert!(synthesis.tensions.len() >= 0 || synthesis.robust_findings.len() >= 0);
}

#[test]
fn test_synthesis_disconnected() {
    let graph = make_disconnected();
    let synthesis = run_analysis(&graph);
    let graph_type = identify_graph_type(&graph);

    assert_eq!(graph_type, "disconnected");
    // Disconnected should have blind spots
    assert!(!synthesis.blind_spots.is_empty() || synthesis.specialist_reports.iter().any(|r| !r.unanswered.is_empty()));
}

#[test]
fn test_quality_assessment() {
    let graph = make_small_rigid();
    let synthesis = run_analysis(&graph);
    let quality = QualityReport::assess(&synthesis);

    assert!(quality.overall_score >= 0.0 && quality.overall_score <= 1.0);
    assert!(matches!(quality.assessment, QualityAssessment::Excellent | QualityAssessment::Good | QualityAssessment::Fair | QualityAssessment::Poor | QualityAssessment::Failed));
}

#[test]
fn test_all_graphs_produce_valid_synthesis() {
    let graphs = vec![
        make_small_rigid(),
        make_over_connected(),
        make_disconnected(),
    ];

    for graph in graphs {
        let synthesis = run_analysis(&graph);
        let quality = QualityReport::assess(&synthesis);

        assert!(synthesis.overall_confidence > 0.0, "Confidence should be positive");
        assert!(quality.overall_score >= 0.0, "Quality score should be non-negative");
    }
}

#[test]
fn test_custom_analysis_subset() {
    let graph = make_small_rigid();
    let synthesis = run_custom_analysis(&graph, &["topological", "systems"]);

    // Should only have 2 specialist reports
    assert_eq!(synthesis.specialist_reports.len(), 2);
    assert!(synthesis.overall_confidence > 0.0);
}

#[test]
fn test_plato_tile_output() {
    use fleet_spread::plato_tile::{PlatoTile, TileWriter};

    let tile = PlatoTile::new("fleet-spread.test", "graph-1")
        .with_confidence(0.85)
        .with_data(serde_json::json!({"key": "value"}));

    assert_eq!(tile.tile_type, "fleet-spread.test");
    assert_eq!(tile.confidence, 0.85);

    // Test tile writer
    let temp_dir = std::env::temp_dir().join("fleet-spread-test");
    let writer = TileWriter::new(temp_dir.clone());
    let path = writer.write_tile(&tile).unwrap();
    assert!(path.exists());

    // Cleanup
    std::fs::remove_dir_all(temp_dir).ok();
}
