//! Tests for v2 library-gate quality gate
//!
//! v2 quality gate assesses: "Did the single selected specialist produce useful output?"
//! This is PASS/FAIL on usefulness, not COMPARISON like v1's MoE synthesis.

use fleet_spread::*;
use fleet_spread::quality::{QualityAssessment, SingleSpecialistQuality, SpecialistValueReport};

#[test]
fn test_quality_empty_report_fails() {
    // Zero findings → Failed
    let report = SpecialistReport::new("topological");
    let quality = SingleSpecialistQuality::calculate(&report);

    assert_eq!(quality.assessment, QualityAssessment::Failed);
    assert!(!quality.has_findings);
    assert_eq!(quality.overall_score, 0.0);
    assert!(quality.reason.contains("No findings"));
}

#[test]
fn test_quality_trivial_findings_fails() {
    // Trivial findings (obvious from topology) → Poor or Failed
    let mut report = SpecialistReport::new("topological");
    report.confidence = 0.7;
    // Trivial claim - obvious from basic topology
    report.add_finding("Graph has edges".to_string(), 0.8, vec!["E=7".to_string()]);
    report.add_finding("Graph is connected".to_string(), 0.85, vec!["C=1".to_string()]);

    let quality = SingleSpecialistQuality::calculate(&report);

    // Should be low significance since claims are trivial
    assert!(quality.finding_significance < 0.3);
}

#[test]
fn test_quality_significant_findings_passes() {
    // Significant findings → Good or Excellent
    let mut report = SpecialistReport::new("topological");
    report.confidence = 0.85;
    // Significant claims - specific, non-obvious
    report.add_finding(
        "Graph is Laman-rigid (E=7=2V-3)".to_string(),
        0.9,
        vec!["V=5, E=7".to_string(), "Laman condition satisfied".to_string()],
    );
    report.add_finding(
        "Cycle 3 has high holonomy (0.95)".to_string(),
        0.85,
        vec!["holonomy=0.95".to_string(), "cycle_length=3".to_string()],
    );

    let quality = SingleSpecialistQuality::calculate(&report);

    assert!(
        matches!(
            quality.assessment,
            QualityAssessment::Excellent | QualityAssessment::Good
        ),
        "Expected Excellent or Good, got {:?}",
        quality.assessment
    );
    assert!(quality.overall_score > 0.5);
    assert!(quality.has_findings);
}

#[test]
fn test_quality_internal_consistency() {
    // Contradictory findings → reduced consistency score
    let mut report = SpecialistReport::new("topological");
    report.confidence = 0.75;
    // Two findings that contradict each other
    report.add_finding(
        "Graph is rigid".to_string(),
        0.8,
        vec!["Laman condition satisfied".to_string()],
    );
    report.add_finding(
        "Graph is over-constrained".to_string(),
        0.75,
        vec!["E > 2V-3".to_string()],
    );

    let quality = SingleSpecialistQuality::calculate(&report);

    // Should detect contradiction and penalize
    assert!(
        quality.consistency_score < 1.0,
        "Expected consistency < 1.0 for contradictory findings, got {}",
        quality.consistency_score
    );
}

#[test]
fn test_quality_confidence_threshold() {
    // Low confidence → penalize or fail
    let mut report = SpecialistReport::new("topological");
    report.confidence = 0.2; // Very low confidence
    report.add_finding(
        "Graph is Laman-rigid".to_string(),
        0.9,
        vec!["V=5, E=7".to_string()],
    );

    let quality = SingleSpecialistQuality::calculate(&report);

    // Low confidence should lead to poor or failed assessment
    assert!(
        matches!(quality.assessment, QualityAssessment::Failed | QualityAssessment::Poor),
        "Expected Failed or Poor for low confidence, got {:?}",
        quality.assessment
    );
}

#[test]
fn test_quality_confidence_threshold_pass() {
    // High confidence with good findings → passes
    let mut report = SpecialistReport::new("topological");
    report.confidence = 0.85;
    report.add_finding(
        "Graph is Laman-rigid (E=7=2V-3)".to_string(),
        0.9,
        vec!["V=5, E=7".to_string()],
    );
    report.add_finding(
        "Cycle 3 has high holonomy".to_string(),
        0.85,
        vec!["holonomy=0.95".to_string()],
    );

    let quality = SingleSpecialistQuality::calculate(&report);

    assert!(quality.confidence_score >= 0.5);
    assert!(
        matches!(
            quality.assessment,
            QualityAssessment::Excellent | QualityAssessment::Good | QualityAssessment::Fair
        ),
        "Expected at least Fair for high confidence with good findings, got {:?}",
        quality.assessment
    );
}

#[test]
fn test_specialist_value_report_passes() {
    // SpecialistValueReport with good output should pass
    let mut report = SpecialistReport::new("topological");
    report.confidence = 0.88;
    report.add_finding(
        "Graph is Laman-rigid (E=7=2V-3)".to_string(),
        0.9,
        vec!["V=5, E=7".to_string()],
    );
    report.add_finding(
        "Cycle 3 has high holonomy".to_string(),
        0.85,
        vec!["holonomy=0.95".to_string()],
    );

    let value_report = SpecialistValueReport::from_specialist("topological", report);

    assert!(value_report.passed());
    assert!(value_report.value_score > 0.0);
    assert_eq!(value_report.specialist_id, "topological");
}

#[test]
fn test_specialist_value_report_fails_on_empty() {
    // Empty report should not pass
    let report = SpecialistReport::new("topological");
    let value_report = SpecialistValueReport::from_specialist("topological", report);

    assert!(!value_report.passed());
    assert_eq!(value_report.value_score, 0.0);
}

#[test]
fn test_specialist_value_report_fails_on_low_confidence() {
    // Low confidence should fail even with findings
    let mut report = SpecialistReport::new("algebraic");
    report.confidence = 0.25;
    report.add_finding(
        "Encoding is stable".to_string(),
        0.9,
        vec!["stability=0.95".to_string()],
    );

    let value_report = SpecialistValueReport::from_specialist("algebraic", report);

    assert!(!value_report.passed());
}

#[test]
fn test_quality_all_pass_criteria_met() {
    // All pass criteria met → Excellent
    let mut report = SpecialistReport::new("systems");
    report.confidence = 0.88;
    // At least 2 findings
    report.add_finding(
        "Graph is Laman-rigid (E=7=2V-3)".to_string(),
        0.9,
        vec!["V=5, E=7".to_string()],
    );
    report.add_finding(
        "No over-constrained nodes".to_string(),
        0.85,
        vec!["max_degree reasonable".to_string()],
    );
    // Average confidence >= 0.5 (we have 0.9 and 0.85, avg = 0.875)
    // At least 1 finding with significance > 0.3 (both should qualify)

    let quality = SingleSpecialistQuality::calculate(&report);

    // All criteria met, should be at least Good
    assert!(quality.overall_score >= 0.6);
}

#[test]
fn test_quality_reason_provided() {
    // Quality report should include a reason string
    let mut report = SpecialistReport::new("geometric");
    report.confidence = 0.8;
    report.add_finding(
        "Trust geometry is consistent".to_string(),
        0.85,
        vec!["holonomy < threshold".to_string()],
    );

    let quality = SingleSpecialistQuality::calculate(&report);

    assert!(!quality.reason.is_empty(), "Quality reason should not be empty");
    println!("Quality reason: {}", quality.reason);
}

#[test]
fn test_single_specialist_quality_no_findings_very_low_confidence() {
    // No findings AND low confidence → definitely Failed
    let mut report = SpecialistReport::new("topological");
    report.confidence = 0.1;

    let quality = SingleSpecialistQuality::calculate(&report);

    assert_eq!(quality.assessment, QualityAssessment::Failed);
    assert!(quality.overall_score < 0.2);
}

#[test]
fn test_findings_with_high_evidence_bonus() {
    // Findings with lots of evidence should score higher on significance
    let mut report = SpecialistReport::new("empirical");
    report.confidence = 0.85;
    report.add_finding(
        "No anomalous trust values".to_string(),
        0.88,
        vec![
            "all within 1σ".to_string(),
            "all within 2σ".to_string(),
            "mean=0.82".to_string(),
            "std=0.05".to_string(),
        ],
    );

    let quality = SingleSpecialistQuality::calculate(&report);

    // Multiple evidence items should boost significance
    assert!(quality.finding_significance >= 0.4);
}

#[test]
fn test_multiple_significant_findings_score_high() {
    // Multiple significant findings → higher overall score
    let mut report = SpecialistReport::new("topological");
    report.confidence = 0.9;
    report.add_finding(
        "Graph is Laman-rigid (E=7=2V-3)".to_string(),
        0.92,
        vec!["V=5, E=7".to_string()],
    );
    report.add_finding(
        "Cycle 3 has high holonomy (0.95)".to_string(),
        0.88,
        vec!["holonomy=0.95".to_string()],
    );
    report.add_finding(
        "Graph is generically rigid".to_string(),
        0.85,
        vec!["generic_position".to_string()],
    );

    let quality = SingleSpecialistQuality::calculate(&report);

    assert!(
        matches!(quality.assessment, QualityAssessment::Excellent | QualityAssessment::Good),
        "Expected at least Good for multiple significant findings, got {:?}",
        quality.assessment
    );
    assert!(quality.overall_score >= 0.7);
}