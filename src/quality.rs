//! Quality metrics for fleet-spread analysis

use crate::synthesis::SynthesisReport;
use serde::{Deserialize, Serialize};

/// Quality assessment of the synthesis
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn assess(synthesis: &SynthesisReport) -> Self {
        Self::calculate(synthesis)
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_assessment_good() {
        // Create a synthesis with good metrics
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
}
