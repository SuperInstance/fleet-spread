//! Quality metrics for fleet-spread analysis

use crate::synthesis::SynthesisReport;
use serde::{Deserialize, Serialize};

/// Quality assessment of the synthesis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(since = "0.2.0", note = "Use SingleSpecialistQuality for v2 library-gate architecture")]
pub struct QualityReport {
    pub novelty_score: f64,      // Does synthesis reveal something new?
    pub correctness_score: f64, // Are specialist analyses accurate?
    pub usefulness_score: f64, // Would users prefer synthesis?
    pub completeness_score: f64, // Are important questions answered?
    pub overall_score: f64,
    pub assessment: QualityAssessment,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum QualityAssessment {
    Excellent,
    Good,
    Fair,
    Poor,
    Failed,
}

impl QualityReport {
    #[deprecated(since = "0.2.0", note = "Use SingleSpecialistQuality::assess for v2")]
    pub fn assess(synthesis: &SynthesisReport) -> Self {
        Self::calculate(synthesis)
    }

    #[deprecated(since = "0.2.0", note = "Use SingleSpecialistQuality::calculate for v2")]
    pub fn calculate(synthesis: &SynthesisReport) -> Self {
        let novelty = Self::calculate_novelty(synthesis);
        let correctness = Self::calculate_correctness(synthesis);
        let usefulness = Self::calculate_usefulness(synthesis);
        let completeness = Self::calculate_completeness(synthesis);

        let overall = (novelty + correctness + usefulness + completeness) / 4.0;

        let assessment = if overall >= 0.8 {
            QualityAssessment::Excellent
        } else if overall >= 0.6 {
            QualityAssessment::Good
        } else if overall >= 0.4 {
            QualityAssessment::Fair
        } else if overall >= 0.2 {
            QualityAssessment::Poor
        } else {
            QualityAssessment::Failed
        };

        Self {
            novelty_score: novelty,
            correctness_score: correctness,
            usefulness_score: usefulness,
            completeness_score: completeness,
            overall_score: overall,
            assessment,
        }
    }

    /// Novelty: Does synthesis reveal something none of the specialists saw alone?
    #[deprecated(since = "0.2.0")]
    fn calculate_novelty(synthesis: &SynthesisReport) -> f64 {
        // Novelty is high when:
        // 1. Synthesis gain is positive
        // 2. There are tensions (conflicts are interesting)
        // 3. There are robust findings from multiple specialists

        let gain_bonus = synthesis.synthesis_gain.max(0.0) * 0.4;
        let tension_bonus = (synthesis.tensions.len() as f64 * 0.05).min(0.3);
        let robust_bonus = if synthesis.robust_findings.len() >= 3 { 0.3 } else { 0.0 };

        (gain_bonus + tension_bonus + robust_bonus).min(1.0)
    }

    /// Correctness: Are specialist analyses internally consistent?
    #[deprecated(since = "0.2.0")]
    fn calculate_correctness(synthesis: &SynthesisReport) -> f64 {
        // High correctness when:
        // 1. All specialists have high confidence
        // 2. No tensions (or tensions are explained)
        // 3. Findings have strong evidence

        let avg_confidence: f64 = synthesis.specialist_reports.iter()
            .map(|r| r.confidence)
            .sum::<f64>() / synthesis.specialist_reports.len().max(1) as f64;

        let tension_penalty = (synthesis.tensions.len() as f64 * 0.05).min(0.3);

        let evidence_score: f64 = synthesis.specialist_reports.iter()
            .map(|r| r.findings.iter().map(|f| f.evidence.len()).sum::<usize>())
            .sum::<usize>() as f64;
        let evidence_bonus = (evidence_score / 50.0).min(0.2);

        (avg_confidence - tension_penalty + evidence_bonus).max(0.0).min(1.0)
    }

    /// Usefulness: Would an agent/human prefer the synthesis?
    #[deprecated(since = "0.2.0")]
    fn calculate_usefulness(synthesis: &SynthesisReport) -> f64 {
        // Useful when:
        // 1. Synthesis gain > 0 (synthesis adds value)
        // 2. There are robust findings (actionable insights)
        // 3. Blind spots are identified (honest about limitations)

        let gain_score: f64 = if synthesis.synthesis_gain > 0.3 { 0.4 }
            else if synthesis.synthesis_gain > 0.0 { 0.2 }
            else { 0.0 };

        let robust_bonus: f64 = if synthesis.robust_findings.is_empty() { 0.0 }
            else if synthesis.robust_findings.len() >= 3 { 0.3 }
            else { 0.2 };

        // Some blind spots is actually good (shows intellectual honesty)
        let blind_spot_bonus: f64 = if synthesis.blind_spots.is_empty() { 0.1 }
            else if synthesis.blind_spots.len() <= 3 { 0.2 }
            else { 0.15 };

        (gain_score + robust_bonus + blind_spot_bonus).min(1.0)
    }

    /// Completeness: Are important questions answered?
    #[deprecated(since = "0.2.0")]
    fn calculate_completeness(synthesis: &SynthesisReport) -> f64 {
        // Complete when:
        // 1. All 5 specialists provided reports
        // 2. Few blind spots
        // 3. High overall confidence

        let specialist_coverage = synthesis.specialist_reports.len() as f64 / 5.0;

        let blind_spot_penalty = (synthesis.blind_spots.len() as f64 * 0.05).min(0.3);

        let confidence_score = synthesis.overall_confidence;

        (specialist_coverage * 0.4 + confidence_score * 0.4 - blind_spot_penalty + 0.2).max(0.0).min(1.0)
    }
}

impl std::fmt::Display for QualityAssessment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualityAssessment::Excellent => write!(f, "Excellent"),
            QualityAssessment::Good => write!(f, "Good"),
            QualityAssessment::Fair => write!(f, "Fair"),
            QualityAssessment::Poor => write!(f, "Poor"),
            QualityAssessment::Failed => write!(f, "Failed"),
        }
    }
}

// =============================================================================
// V2 Library Gate Quality Gate
// =============================================================================

use crate::specialists::SpecialistReport;

/// Quality assessment for v2 library-gate architecture.
///
/// v2 selects ONE specialist via library gate, then assesses whether that
/// specialist produced useful output. This is PASS/FAIL on usefulness,
/// not COMPARISON like v1's MoE synthesis.
///
/// # Pass Criteria
/// - At least 2 findings
/// - Average confidence >= 0.5
/// - At least 1 finding with significance > 0.3
/// - Findings are internally consistent (no direct contradictions)
///
/// # Fail Criteria
/// - Zero findings
/// - All findings are trivial (significance near 0)
/// - Confidence < 0.3
/// - Internal contradictions detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleSpecialistQuality {
    /// At least one finding exists
    pub has_findings: bool,
    /// Specialist's self-reported confidence
    pub confidence_score: f64,
    /// Are findings non-trivial? (not obvious from topology alone)
    pub finding_significance: f64,
    /// Do findings internally agree?
    pub consistency_score: f64,
    /// Overall quality score [0, 1]
    pub overall_score: f64,
    /// Human-readable assessment
    pub assessment: QualityAssessment,
    /// Why we passed/failed
    pub reason: String,
}

impl SingleSpecialistQuality {
    /// Assess quality of a single specialist's output (v2 library gate)
    pub fn assess(_specialist: &str, report: &SpecialistReport) -> Self {
        Self::calculate(report)
    }

    /// Calculate quality for a single specialist report
    pub fn calculate(report: &SpecialistReport) -> Self {
        let has_findings = !report.findings.is_empty();
        let confidence_score = report.confidence;
        let finding_significance = Self::assess_significance(report);
        let consistency_score = Self::assess_consistency(report);

        // Compute overall score
        let overall_score = Self::compute_overall(
            has_findings,
            confidence_score,
            finding_significance,
            consistency_score,
        );

        // Determine assessment and reason
        let (assessment, reason) = Self::determine_assessment(
            has_findings,
            confidence_score,
            finding_significance,
            consistency_score,
            &report.findings,
        );

        SingleSpecialistQuality {
            has_findings,
            confidence_score,
            finding_significance,
            consistency_score,
            overall_score,
            assessment,
            reason,
        }
    }

    /// Finding significance: is this non-trivial info?
    ///
    /// Trivial = "graph has edges" (obvious from topology)
    /// Significant = "cycle 3 has high holonomy" (specific, non-obvious)
    fn assess_significance(report: &SpecialistReport) -> f64 {
        if report.findings.is_empty() {
            return 0.0;
        }

        // Each finding gets a significance score based on:
        // 1. Does it mention specific values/numbers? (not just generic statements)
        // 2. Does it reference mathematical properties?
        // 3. Does it contain constraints or thresholds?

        let mut total_significance = 0.0;

        for finding in &report.findings {
            let claim_lower = finding.claim.to_lowercase();
            let mut significance = 0.3; // Base: any finding has some value

            // Specific values mentioned (cycles, degrees, thresholds)
            let has_specific_values = claim_lower.contains("cycle")
                || claim_lower.contains("degree")
                || claim_lower.contains("threshold")
                || claim_lower.contains("rigid")
                || claim_lower.contains("stable")
                || claim_lower.contains("constraint")
                || claim_lower.contains("connected");
            if has_specific_values {
                significance += 0.2;
            }

            // Evidence provides concrete support
            if !finding.evidence.is_empty() {
                significance += 0.15;
            }

            // High confidence finding implies specialist worked hard
            significance += finding.confidence * 0.2;

            // Trivial patterns: generic topology without numbers
            let is_trivial = claim_lower.contains("graph has edges")
                || claim_lower.contains("graph is connected")
                || claim_lower.contains("there are vertices")
                || claim_lower.contains("has nodes");
            if is_trivial {
                significance *= 0.3; // Significant penalty for trivial claims
            }

            total_significance += significance.min(1.0);
        }

        (total_significance / report.findings.len() as f64).min(1.0)
    }

    /// Internal consistency: do findings contradict each other?
    fn assess_consistency(report: &SpecialistReport) -> f64 {
        if report.findings.len() <= 1 {
            return 1.0; // Single finding can't contradict itself
        }

        let mut contradiction_count = 0;
        let findings = &report.findings;

        // Check for contradictory patterns
        let opposites = [
            ("rigid", "under-constrained"),
            ("rigid", "over-constrained"),
            ("stable", "drift"),
            ("stable", "unstable"),
            ("consistent", "strain"),
            ("high", "low"),
            ("anomalous", "normal"),
            ("connected", "disconnected"),
        ];

        for i in 0..findings.len() {
            for j in (i + 1)..findings.len() {
                let claim_i = findings[i].claim.to_lowercase();
                let claim_j = findings[j].claim.to_lowercase();

                // Direct contradiction check
                let mut is_contradiction = false;
                for (pos, neg) in &opposites {
                    let i_has_pos = claim_i.contains(pos);
                    let i_has_neg = claim_i.contains(neg);
                    let j_has_pos = claim_j.contains(pos);
                    let j_has_neg = claim_j.contains(neg);

                    // Claim i is positive, claim j is negative on same concept
                    if i_has_pos && j_has_neg && claim_i.contains(pos) && claim_j.contains(neg) {
                        is_contradiction = true;
                    }
                    // Or vice versa
                    if i_has_neg && j_has_pos && claim_i.contains(neg) && claim_j.contains(pos) {
                        is_contradiction = true;
                    }
                }

                if is_contradiction {
                    contradiction_count += 1;
                }
            }
        }

        // Score: 1.0 - penalty for contradictions
        // Max 2 contradictions = 0.0, fewer = proportionally higher
        let penalty = (contradiction_count as f64 * 0.3).min(0.8);
        (1.0 - penalty).max(0.0)
    }

    fn compute_overall(
        has_findings: bool,
        confidence_score: f64,
        finding_significance: f64,
        consistency_score: f64,
    ) -> f64 {
        if !has_findings {
            return 0.0;
        }

        // Weighted combination
        let finding_bonus = if has_findings { 0.1 } else { 0.0 };
        let score = (confidence_score * 0.3)
            + (finding_significance * 0.35)
            + (consistency_score * 0.25)
            + finding_bonus;

        score.min(1.0).max(0.0)
    }

    fn determine_assessment(
        has_findings: bool,
        confidence_score: f64,
        finding_significance: f64,
        consistency_score: f64,
        findings: &[crate::specialists::Finding],
    ) -> (QualityAssessment, String) {
        // FAIL: Zero findings
        if !has_findings {
            return (QualityAssessment::Failed, "No findings produced".to_string());
        }

        // FAIL: Low confidence
        if confidence_score < 0.3 {
            return (
                QualityAssessment::Failed,
                format!("Confidence too low ({:.0}%)", confidence_score * 100.0),
            );
        }

        // FAIL: All findings trivial
        if finding_significance < 0.1 {
            return (
                QualityAssessment::Failed,
                "All findings are trivial (obvious from topology)".to_string(),
            );
        }

        // FAIL: Internal contradictions
        if consistency_score < 0.4 {
            return (
                QualityAssessment::Poor,
                format!("Internal contradictions detected (consistency {:.0}%)", consistency_score * 100.0),
            );
        }

        // Calculate weighted score for pass/fail determination
        let avg_confidence = findings.iter().map(|f| f.confidence).sum::<f64>() / findings.len() as f64;
        let has_significant = findings.iter().any(|f| {
            // Check if finding mentions specific properties
            let claim = f.claim.to_lowercase();
            claim.contains("cycle") || claim.contains("rigid") || claim.contains("stable")
            || claim.contains("constraint") || claim.contains("laman") || claim.contains("holonomy")
        });

        // Pass criteria
        let enough_findings = findings.len() >= 2;
        let good_confidence = avg_confidence >= 0.5;
        let has_significant_finding = finding_significance > 0.3 || has_significant;
        let is_consistent = consistency_score >= 0.7;

        if enough_findings && good_confidence && has_significant_finding && is_consistent {
            let score = (finding_significance + consistency_score + avg_confidence) / 3.0;
            if score >= 0.8 {
                return (QualityAssessment::Excellent, "Excellent single-specialist output".to_string());
            } else {
                return (QualityAssessment::Good, "Good single-specialist output".to_string());
            }
        }

        // Partial pass - Fair or Poor
        if enough_findings && good_confidence {
            return (QualityAssessment::Fair, "Acceptable output, some criteria not met".to_string());
        }

        (QualityAssessment::Poor, "Does not meet quality criteria".to_string())
    }
}

/// Value report for v2 single-specialist output.
/// Replaces SynthesisReport for v2 library-gate architecture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistValueReport {
    /// The specialist that ran
    pub specialist_id: String,
    /// The specialist's report
    pub report: SpecialistReport,
    /// Did this specialist add value? [0, 1]
    pub value_score: f64,
    /// Quality assessment
    pub quality: SingleSpecialistQuality,
}

impl SpecialistValueReport {
    /// Create a value report from a single specialist's output
    pub fn from_specialist(specialist_id: &str, report: SpecialistReport) -> Self {
        let quality = SingleSpecialistQuality::calculate(&report);
        let value_score = Self::specialist_value(&report, &quality);

        SpecialistValueReport {
            specialist_id: specialist_id.to_string(),
            report,
            value_score,
            quality,
        }
    }

    /// Calculate specialist value: did this specialist produce non-trivial output?
    fn specialist_value(report: &SpecialistReport, quality: &SingleSpecialistQuality) -> f64 {
        if report.findings.is_empty() {
            return 0.0;
        }

        let info = report.information_content();
        let significance = quality.finding_significance;
        let confidence = report.confidence;

        (info * significance * confidence).min(1.0)
    }

    /// Check if this specialist passed the quality gate
    pub fn passed(&self) -> bool {
        matches!(
            self.quality.assessment,
            QualityAssessment::Excellent | QualityAssessment::Good | QualityAssessment::Fair
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_assessment_good() {
        // Create a synthesis with good metrics
        #[allow(deprecated)]
        let report = QualityReport {
            novelty_score: 0.7,
            correctness_score: 0.75,
            usefulness_score: 0.65,
            completeness_score: 0.7,
            overall_score: 0.7,
            assessment: QualityAssessment::Good,
        };

        assert_eq!(report.assessment, QualityAssessment::Good);
    }

    #[test]
    fn test_single_specialist_quality_empty_fails() {
        let mut report = SpecialistReport::new("test");
        report.confidence = 0.5;

        let quality = SingleSpecialistQuality::calculate(&report);
        assert_eq!(quality.assessment, QualityAssessment::Failed);
        assert!(!quality.has_findings);
        assert_eq!(quality.overall_score, 0.0);
    }

    #[test]
    fn test_single_specialist_quality_trivial_fails() {
        let mut report = SpecialistReport::new("test");
        report.confidence = 0.5;
        // Trivial claim - obvious from topology
        report.add_finding("Graph has edges".to_string(), 0.8, vec!["E=7".to_string()]);

        let quality = SingleSpecialistQuality::calculate(&report);
        assert!(quality.finding_significance < 0.3);
    }

    #[test]
    fn test_single_specialist_quality_significant_passes() {
        let mut report = SpecialistReport::new("test");
        report.confidence = 0.7;
        // Significant claim - specific, non-obvious
        report.add_finding(
            "Graph is Laman-rigid (E=7=2V-3)".to_string(),
            0.85,
            vec!["V=5, E=7".to_string(), "Laman condition satisfied".to_string()],
        );
        report.add_finding(
            "Cycle 3 has high holonomy".to_string(),
            0.8,
            vec!["holonomy=0.95".to_string()],
        );

        let quality = SingleSpecialistQuality::calculate(&report);
        assert!(quality.overall_score > 0.5);
        assert!(matches!(
            quality.assessment,
            QualityAssessment::Good | QualityAssessment::Excellent
        ));
    }

    #[test]
    fn test_single_specialist_quality_consistency() {
        let mut report = SpecialistReport::new("test");
        report.confidence = 0.7;
        // Two findings that might be contradictory
        report.add_finding("Graph is rigid".to_string(), 0.8, vec!["Laman satisfied".to_string()]);
        report.add_finding("Graph is over-constrained".to_string(), 0.75, vec!["E > 2V-3".to_string()]);

        let quality = SingleSpecialistQuality::calculate(&report);
        // Should detect the contradiction and penalize
        assert!(quality.consistency_score < 1.0);
    }

    #[test]
    fn test_specialist_value_report_passes() {
        let mut report = SpecialistReport::new("topological");
        report.confidence = 0.8;
        report.add_finding(
            "Graph is Laman-rigid".to_string(),
            0.85,
            vec!["V=5, E=7".to_string()],
        );
        report.add_finding(
            "Cycle 3 has high holonomy".to_string(),
            0.8,
            vec!["holonomy=0.95".to_string()],
        );

        let value_report = SpecialistValueReport::from_specialist("topological", report);
        assert!(value_report.passed());
        assert!(value_report.value_score > 0.0);
    }
}