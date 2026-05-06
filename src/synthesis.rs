//! Synthesis layer - combines 5 specialist reports into unified analysis

use crate::specialists::SpecialistReport;
use serde::{Deserialize, Serialize};

/// Synthesis output combining all specialist perspectives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisReport {
    pub robust_findings: Vec<RobustFinding>,
    pub tensions: Vec<Tension>,
    pub blind_spots: Vec<String>,
    pub synthesis_gain: f64,
    pub overall_confidence: f64,
    pub specialist_reports: Vec<SpecialistReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobustFinding {
    pub claim: String,
    pub supporting_specialists: Vec<String>,
    pub confidence: f64,
    pub evidence_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tension {
    pub claim_a: String,
    pub claim_b: String,
    pub specialist_a: String,
    pub specialist_b: String,
    pub description: String,
}

pub struct SynthesisEngine {
    agreement_threshold: usize, // Minimum specialists needed to call something "robust"
    tension_check: bool,
}

impl Default for SynthesisEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SynthesisEngine {
    pub fn new() -> Self {
        Self {
            agreement_threshold: 3,
            tension_check: true,
        }
    }

    /// Synthesize multiple specialist reports
    pub fn synthesize(&self, reports: Vec<SpecialistReport>) -> SynthesisReport {
        let robust = self.find_robust_findings(&reports);
        let tensions = self.find_tensions(&reports);
        let blind_spots = self.collect_blind_spots(&reports);
        let gain = self.calculate_synthesis_gain(&reports);
        let overall_confidence = self.calculate_overall_confidence(&reports);

        SynthesisReport {
            robust_findings: robust,
            tensions,
            blind_spots,
            synthesis_gain: gain,
            overall_confidence,
            specialist_reports: reports,
        }
    }

    /// Find findings that multiple specialists agree on
    fn find_robust_findings(&self, reports: &[SpecialistReport]) -> Vec<RobustFinding> {
        let mut robust = Vec::new();
        let mut claim_to_specialists: std::collections::HashMap<String, Vec<(&str, f64, usize)>> =
            std::collections::HashMap::new();

        // Group claims by semantic similarity
        for report in reports {
            for finding in &report.findings {
                let normalized = self.normalize_claim(&finding.claim);
                let entry = claim_to_specialists.entry(normalized).or_default();
                entry.push((report.specialist_id.as_str(), finding.confidence, finding.evidence.len()));
            }
        }

        // Find claims with agreement from multiple specialists
        for (claim, specialists) in claim_to_specialists {
            if specialists.len() >= self.agreement_threshold {
                let total_confidence: f64 = specialists.iter().map(|s| s.1).sum::<f64>() / specialists.len() as f64;
                let total_evidence: usize = specialists.iter().map(|s| s.2).sum();

                robust.push(RobustFinding {
                    claim,
                    supporting_specialists: specialists.iter().map(|s| s.0.to_string()).collect(),
                    confidence: total_confidence,
                    evidence_count: total_evidence,
                });
            }
        }

        // Sort by number of supporting specialists, then by confidence
        robust.sort_by(|a, b| {
            let specialists_cmp = b.supporting_specialists.len().cmp(&a.supporting_specialists.len());
            if specialists_cmp == std::cmp::Ordering::Equal {
                b.confidence.partial_cmp(&a.confidence).unwrap()
            } else {
                specialists_cmp
            }
        });

        robust
    }

    /// Find disagreements between specialists
    fn find_tensions(&self, reports: &[SpecialistReport]) -> Vec<Tension> {
        let mut tensions = Vec::new();

        // Look for semantic opposites or contradictory claims
        let claims: Vec<(&str, &str, f64)> = reports.iter()
            .flat_map(|r| r.findings.iter().map(|f| (r.specialist_id.as_str(), f.claim.as_str(), f.confidence)))
            .collect();

        for i in 0..claims.len() {
            for j in (i+1)..claims.len() {
                let (sid_a, claim_a, _conf_a) = claims[i];
                let (sid_b, claim_b, _conf_b) = claims[j];

                if sid_a == sid_b {
                    continue;
                }

                if self.are_contradictory(claim_a, claim_b) {
                    tensions.push(Tension {
                        claim_a: claim_a.to_string(),
                        claim_b: claim_b.to_string(),
                        specialist_a: sid_a.to_string(),
                        specialist_b: sid_b.to_string(),
                        description: format!("{} and {} disagree on: {} vs {}",
                            sid_a, sid_b,
                            Self::truncate(claim_a, 50),
                            Self::truncate(claim_b, 50)),
                    });
                }
            }
        }

        tensions
    }

    /// Check if two claims are contradictory
    fn are_contradictory(&self, a: &str, b: &str) -> bool {
        // Look for opposite patterns
        let opposites = [
            ("rigid", "under-constrained"),
            ("rigid", "over-constrained"),
            ("consistent", "strain"),
            ("stable", "drift"),
            ("anomalous", "normal"),
            ("connected", "disconnected"),
            ("high", "low"),
        ];

        for (pos, neg) in opposites {
            let a_lower = a.to_lowercase();
            let b_lower = b.to_lowercase();
            if (a_lower.contains(pos) && b_lower.contains(neg)) ||
               (b_lower.contains(pos) && a_lower.contains(neg)) {
                return true;
            }
        }

        false
    }

    fn normalize_claim(&self, claim: &str) -> String {
        // Extract key terms from claim for matching
        let words: Vec<&str> = claim.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3)
            .collect();
        let key_terms: Vec<&str> = words.iter()
            .filter(|w| !["the", "and", "has", "have", "with", "from"].contains(*w))
            .copied()
            .collect();
        key_terms.join(" ")
    }

    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len])
        }
    }

    /// Collect all unanswered questions from all specialists
    fn collect_blind_spots(&self, reports: &[SpecialistReport]) -> Vec<String> {
        let mut all_unanswered: Vec<String> = reports.iter()
            .flat_map(|r| r.unanswered.clone())
            .collect();

        // Dedupe while preserving order
        let mut seen = std::collections::HashSet::new();
        all_unanswered.retain(|q| seen.insert(q.clone()));

        all_unanswered
    }

    /// Calculate synthesis gain: does unified analysis add value?
    fn calculate_synthesis_gain(&self, reports: &[SpecialistReport]) -> f64 {
        if reports.is_empty() {
            return 0.0;
        }

        let total_info: f64 = reports.iter()
            .map(|r| r.information_content())
            .sum();

        let best_single = reports.iter()
            .map(|r| r.information_content())
            .fold(0.0f64, f64::max);

        if best_single < 1e-10 {
            return 0.0; // No meaningful info in any specialist
        }

        // Synthesis adds value if robust findings exist or tensions highlight interesting structure
        let robust_bonus = (self.find_robust_findings(reports).len() as f64) * 0.1;
        let tension_bonus = (self.find_tensions(reports).len() as f64) * 0.05;
        let blind_spot_penalty = (self.collect_blind_spots(reports).len() as f64) * 0.02;

        let synthesis_value = robust_bonus + tension_bonus - blind_spot_penalty;
        let gain = synthesis_value / best_single;

        // Clamp to reasonable range
        gain.max(-1.0).min(1.0)
    }

    /// Calculate overall confidence in the synthesis
    fn calculate_overall_confidence(&self, reports: &[SpecialistReport]) -> f64 {
        if reports.is_empty() {
            return 0.0;
        }

        let mean_confidence: f64 = reports.iter()
            .map(|r| r.confidence)
            .sum::<f64>() / reports.len() as f64;

        // Penalize for tensions
        let tensions = self.find_tensions(reports);
        let tension_penalty = (tensions.len() as f64) * 0.05_f64.min(mean_confidence * 0.3);

        // Bonus for robust findings
        let robust = self.find_robust_findings(reports);
        let robust_bonus = (robust.len() as f64) * 0.05;

        (mean_confidence - tension_penalty + robust_bonus).max(0.1).min(1.0)
    }
}

/// Interpret synthesis results for a specific graph type
pub fn interpret_synthesis(synthesis: &SynthesisReport, graph_type: &str) -> String {
    let mut parts = Vec::new();

    parts.push(format!("Graph type: {}", graph_type));
    parts.push(format!("Overall confidence: {:.0}%", synthesis.overall_confidence * 100.0));
    parts.push(format!("Synthesis gain: {:.2}", synthesis.synthesis_gain));

    if !synthesis.robust_findings.is_empty() {
        parts.push(format!("\nRobust findings ({} confirmed by ≥3 specialists):", synthesis.robust_findings.len()));
        for finding in &synthesis.robust_findings[..synthesis.robust_findings.len().min(3)] {
            parts.push(format!("  • {} ({})", finding.claim, finding.supporting_specialists.join(", ")));
        }
    }

    if !synthesis.tensions.is_empty() {
        parts.push(format!("\nTensions ({} disagreements):", synthesis.tensions.len()));
        for tension in &synthesis.tensions[..synthesis.tensions.len().min(3)] {
            parts.push(format!("  • {} vs {}", tension.specialist_a, tension.specialist_b));
        }
    }

    if !synthesis.blind_spots.is_empty() {
        parts.push(format!("\nBlind spots ({} unaddressed questions):", synthesis.blind_spots.len()));
        for blind in &synthesis.blind_spots[..synthesis.blind_spots.len().min(3)] {
            parts.push(format!("  • {}", blind));
        }
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_reports() -> Vec<SpecialistReport> {
        vec![
            {
                let mut r = SpecialistReport::new("topological");
                r.add_finding("Graph is Laman-rigid".to_string(), 0.9, vec!["E=2V-3".to_string()]);
                r.add_finding("Graph is connected".to_string(), 0.95, vec!["C=1".to_string()]);
                r.confidence = 0.85;
                r
            },
            {
                let mut r = SpecialistReport::new("systems");
                r.add_finding("Graph is Laman-rigid".to_string(), 0.85, vec!["E=7=2*5-3".to_string()]);
                r.add_finding("No over-constrained nodes".to_string(), 0.8, vec!["max_degree reasonable".to_string()]);
                r.confidence = 0.82;
                r
            },
            {
                let mut r = SpecialistReport::new("geometric");
                r.add_finding("Trust geometry is consistent".to_string(), 0.85, vec!["holonomy < threshold".to_string()]);
                r.add_finding("No edges under stress".to_string(), 0.75, vec!["All cycles low strain".to_string()]);
                r.confidence = 0.8;
                r
            },
            {
                let mut r = SpecialistReport::new("algebraic");
                r.add_finding("Encoding is highly stable".to_string(), 0.88, vec!["stability=0.95".to_string()]);
                r.confidence = 0.83;
                r
            },
            {
                let mut r = SpecialistReport::new("empirical");
                r.add_finding("No anomalous trust values".to_string(), 0.8, vec!["all within 2σ".to_string()]);
                r.confidence = 0.75;
                r
            },
        ]
    }

    #[test]
    fn test_synthesis_finds_robust_findings() {
        let engine = SynthesisEngine::new();
        let reports = make_test_reports();
        let synthesis = engine.synthesize(reports);

        // "Graph is Laman-rigid" should be robust (topological + systems agree)
        assert!(synthesis.robust_findings.iter().any(|f| f.claim.contains("Laman-rigid")));
        assert!(synthesis.synthesis_gain > 0.0);
    }

    #[test]
    fn test_synthesis_confidence() {
        let engine = SynthesisEngine::new();
        let reports = make_test_reports();
        let synthesis = engine.synthesize(reports);

        assert!(synthesis.overall_confidence > 0.7);
    }
}
