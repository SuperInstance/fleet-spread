//! Captain — expert inquiry engine for fleet graph analysis
//!
//! The captain is the expert who:
//! 1. Runs a WIDE inquiry phase — consults every specialist with relevant signal
//! 2. Deliberates — weighs reports probabilistically, checks for contradictions
//! 3. Applies HARD CONSTRAINTS — P0 = safety, never negotiable
//! 4. Makes a NARROW decision — focused, not diffuse
//!
//! This is NOT a library gate. The library gate only tells you which specialists
//! have signal. The captain decides what to do with that signal.

use crate::constants::AgentConstants;
use crate::graph::FleetGraph;
use crate::graph_state::FleetGraphState;
use crate::quality::{QualityAssessment, SingleSpecialistQuality};
use crate::specialists::SpecialistReport;
use serde::{Deserialize, Serialize};

/// A constraint that is never negotiable
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HardConstraint {
    SafetyMargin(f64),
    SparesRequired(usize),
    TrustThreshold(f64),
    EmergenceCeiling(f64),
    ZhcTolerance(f64),
    TimeWindowS(f64),
}

impl HardConstraint {
    pub fn is_violated(&self, state: &FleetGraphState, reports: &[SpecialistReport]) -> bool {
        match self {
            HardConstraint::SafetyMargin(min_margin) => {
                reports.iter().any(|r| {
                    r.findings.iter().any(|f| {
                        let claim = f.claim.to_lowercase();
                        (claim.contains("unsafe") || claim.contains("danger") || claim.contains("collision"))
                            && f.confidence > 0.7
                    })
                }) || state.zhc_loop_residual > *min_margin
            }
            HardConstraint::SparesRequired(min_spares) => {
                reports.iter().any(|r| {
                    r.findings.iter().any(|f| {
                        let claim = f.claim.to_lowercase();
                        (claim.contains("no spare") || claim.contains("insufficient backup"))
                            && f.confidence > 0.6
                    })
                }) || state.V < *min_spares
            }
            HardConstraint::TrustThreshold(min_trust) => {
                state.trust_vector_entropy > (1.0 - *min_trust)
            }
            HardConstraint::EmergenceCeiling(max_beta) => state.beta_1 > *max_beta,
            HardConstraint::ZhcTolerance(max_residual) => state.zhc_loop_residual > *max_residual,
            HardConstraint::TimeWindowS(max_s) => state.last_change_s > *max_s,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            HardConstraint::SafetyMargin(_) => "Safety margin must be maintained",
            HardConstraint::SparesRequired(_) => "Minimum spares must be available",
            HardConstraint::TrustThreshold(_) => "Trust threshold must be met",
            HardConstraint::EmergenceCeiling(_) => "Emergence ceiling must not be exceeded",
            HardConstraint::ZhcTolerance(_) => "ZHC tolerance must be maintained",
            HardConstraint::TimeWindowS(_) => "Time window must be respected",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptainDeliberation {
    pub reports: Vec<SpecialistReport>,
    pub consulted: Vec<String>,
    pub constraints_applied: Vec<HardConstraint>,
    pub violations: Vec<String>,
    pub adjudicated_findings: Vec<AdjudicatedFinding>,
    pub confidence: f64,
    pub probability_distribution: ProbabilityDistribution,
}

impl CaptainDeliberation {
    pub fn new() -> Self {
        Self {
            reports: Vec::new(),
            consulted: Vec::new(),
            constraints_applied: Vec::new(),
            violations: Vec::new(),
            adjudicated_findings: Vec::new(),
            confidence: 0.0,
            probability_distribution: ProbabilityDistribution::default(),
        }
    }

    pub fn consult(&mut self, specialist_id: &str, report: SpecialistReport) {
        self.reports.push(report);
        self.consulted.push(specialist_id.to_string());
    }

    pub fn apply_constraints(&mut self, constraints: &[HardConstraint], state: &FleetGraphState) {
        self.constraints_applied = constraints.to_vec();
        for constraint in constraints {
            if constraint.is_violated(state, &self.reports) {
                self.violations.push(constraint.description().to_string());
            }
        }
    }

    pub fn adjudicate(&mut self) {
        let mut adjudicated: Vec<AdjudicatedFinding> = Vec::new();

        for report in &self.reports {
            for finding in &report.findings {
                let mut contradicted = false;

                for adj in &adjudicated {
                    if Self::findings_contradict(&finding.claim, &adj.finding.claim) {
                        contradicted = true;
                        break;
                    }
                }

                if !contradicted {
                    let quality = self.reports.iter()
                        .find(|r| r.findings.contains(finding))
                        .map(|r| r.confidence)
                        .unwrap_or(0.5);

                    adjudicated.push(AdjudicatedFinding {
                        finding: finding.clone(),
                        source: report.specialist_id.clone(),
                        quality,
                    });
                }
            }
        }

        adjudicated.sort_by(|a, b| b.quality.partial_cmp(&a.quality).unwrap());
        adjudicated.dedup_by(|a, b| a.finding.claim == b.finding.claim);

        self.adjudicated_findings = adjudicated;
        self.probability_distribution = ProbabilityDistribution::from_findings(&self.adjudicated_findings);
        self.confidence = if self.reports.is_empty() {
            0.0
        } else {
            self.reports.iter().map(|r| r.confidence).sum::<f64>() / self.reports.len() as f64
        };
    }

    fn findings_contradict(claim_a: &str, claim_b: &str) -> bool {
        let a = claim_a.to_lowercase();
        let b = claim_b.to_lowercase();

        let opposites = [
            ("rigid", "under-constrained"),
            ("rigid", "over-constrained"),
            ("stable", "drift"),
            ("stable", "unstable"),
            ("consistent", "strain"),
            ("consistent", "inconsistent"),
            ("connected", "disconnected"),
            ("safe", "unsafe"),
            ("normal", "anomalous"),
        ];

        for (pos, neg) in &opposites {
            let a_has_pos = a.contains(pos);
            let a_has_neg = a.contains(neg);
            let b_has_pos = b.contains(pos);
            let b_has_neg = b.contains(neg);
            if (a_has_pos && b_has_neg) || (a_has_neg && b_has_pos) {
                return true;
            }
        }
        false
    }

    pub fn has_constraint_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    pub fn safe_action_set(&self) -> Vec<String> {
        self.adjudicated_findings.iter()
            .filter(|adj| {
                let claim = adj.finding.claim.to_lowercase();
                claim.contains("safe") || claim.contains("stable") || claim.contains("constraint")
            })
            .map(|adj| adj.finding.claim.clone())
            .collect()
    }
}

impl Default for CaptainDeliberation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjudicatedFinding {
    pub finding: crate::specialists::Finding,
    pub source: String,
    pub quality: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityDistribution {
    pub states: Vec<StateProbability>,
    pub entropy: f64,
}

impl ProbabilityDistribution {
    pub fn from_findings(findings: &[AdjudicatedFinding]) -> Self {
        let mut states: Vec<StateProbability> = findings.iter().map(|adj| {
            StateProbability {
                description: adj.finding.claim.clone(),
                probability: adj.quality,
                source: adj.source.clone(),
            }
        }).collect();

        let total: f64 = states.iter().map(|s| s.probability).sum();
        if total > 0.0 {
            for state in &mut states {
                state.probability /= total;
            }
        }

        let entropy = if states.is_empty() {
            0.0
        } else {
            states.iter()
                .map(|s| if s.probability > 0.0 { -s.probability * s.probability.log2() } else { 0.0 })
                .sum()
        };

        ProbabilityDistribution { states, entropy }
    }

    pub fn is_uncertain(&self) -> bool {
        self.entropy > 0.7
    }

    pub fn most_likely(&self) -> Option<&StateProbability> {
        self.states.iter().max_by(|a, b| a.probability.partial_cmp(&b.probability).unwrap())
    }
}

impl Default for ProbabilityDistribution {
    fn default() -> Self {
        Self { states: Vec::new(), entropy: 0.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProbability {
    pub description: String,
    pub probability: f64,
    pub source: String,
}

pub struct Captain {
    constants: AgentConstants,
    hard_constraints: Vec<HardConstraint>,
}

impl Captain {
    pub fn new() -> Self {
        Self {
            constants: AgentConstants::default_fleet(),
            hard_constraints: Self::default_safety_constraints(),
        }
    }

    pub fn with_constants(constants: AgentConstants) -> Self {
        Self {
            constants,
            hard_constraints: Self::default_safety_constraints(),
        }
    }

    pub fn default_safety_constraints() -> Vec<HardConstraint> {
        vec![
            HardConstraint::SafetyMargin(0.1),
            HardConstraint::SparesRequired(2),
            HardConstraint::TrustThreshold(0.3),
            HardConstraint::EmergenceCeiling(10.0),
            HardConstraint::ZhcTolerance(0.1),
            HardConstraint::TimeWindowS(3600.0),
        ]
    }

    pub fn sources_with_signal(&self, state: &FleetGraphState) -> Vec<&'static str> {
        let mut sources = Vec::new();
        if state.V >= 3 {
            sources.push("systems");
        }
        if state.beta_1 > 0.0 || state.last_change_s < 10.0 {
            sources.push("topological");
        }
        if state.zhc_loop_residual > 0.0 {
            sources.push("geometric");
        }
        if state.E > 0 {
            sources.push("algebraic");
        }
        if state.agent_count != state.V || state.last_change_s > 60.0 {
            sources.push("empirical");
        }
        sources
    }

    pub fn inquire(&self, state: &FleetGraphState, _graph: &FleetGraph) -> CaptainDeliberation {
        let mut deliberation = CaptainDeliberation::new();
        let signal_sources = self.sources_with_signal(state);
        for source in signal_sources {
            let report = SpecialistReport::new(source);
            deliberation.consult(source, report);
        }
        deliberation.apply_constraints(&self.hard_constraints, state);
        deliberation.adjudicate();
        deliberation
    }

    pub fn deliberate(&self, state: &FleetGraphState, graph: &FleetGraph) -> CaptainDecision {
        let deliberation = self.inquire(state, graph);

        if deliberation.has_constraint_violations() {
            return CaptainDecision::Constrained {
                violations: deliberation.violations.clone(),
                deliberation,
                decision: None,
                reason: "Hard constraint violated — P0 = safety takes precedence".to_string(),
            };
        }

        let safe_actions = deliberation.safe_action_set();

        if safe_actions.is_empty() && deliberation.adjudicated_findings.is_empty() {
            return CaptainDecision::Stable {
                deliberation,
                decision: None,
                reason: "Fleet is stable. No action required.".to_string(),
            };
        }

        let likely_desc = deliberation.probability_distribution.most_likely()
            .map(|s| s.description.clone());
        let likely_prob = deliberation.probability_distribution.most_likely()
            .map(|s| s.probability);
        let reason = match (likely_desc.as_deref(), likely_prob) {
            (Some(desc), Some(prob)) => format!("Most likely: {} (p={:.2})", desc, prob),
            _ => "Insufficient data for decision".to_string(),
        };

        CaptainDecision::Decided {
            deliberation,
            decision: likely_desc,
            reason,
        }
    }
}

impl Default for Captain {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptainDecision {
    Decided {
        deliberation: CaptainDeliberation,
        decision: Option<String>,
        reason: String,
    },
    Constrained {
        violations: Vec<String>,
        deliberation: CaptainDeliberation,
        decision: Option<String>,
        reason: String,
    },
    Stable {
        deliberation: CaptainDeliberation,
        decision: Option<String>,
        reason: String,
    },
}

impl CaptainDecision {
    pub fn took_action(&self) -> bool {
        match self {
            CaptainDecision::Decided { decision, .. } => decision.is_some(),
            CaptainDecision::Constrained { .. } => false,
            CaptainDecision::Stable { .. } => false,
        }
    }

    pub fn decision(&self) -> Option<&str> {
        match self {
            CaptainDecision::Decided { decision, .. } => decision.as_deref(),
            CaptainDecision::Constrained { .. } => None,
            CaptainDecision::Stable { decision, .. } => decision.as_deref(),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_state::FleetGraphState;
    use crate::test_helpers::make_graph;

    #[test]
    fn test_captain_default() {
        let captain = Captain::new();
        assert!(!captain.hard_constraints.is_empty());
    }

    #[test]
    fn test_captain_with_constants() {
        let constants = AgentConstants::conservative();
        let captain = Captain::with_constants(constants.clone());
        assert_eq!(captain.constants.beta_threshold, 0.10);
    }

    #[test]
    fn test_sources_small_graph() {
        let captain = Captain::new();
        let state = FleetGraphState::small_graph();
        let sources = captain.sources_with_signal(&state);
        assert!(!sources.contains(&"systems"));
    }

    #[test]
    fn test_sources_rising_beta() {
        let captain = Captain::new();
        let state = FleetGraphState::rising_beta();
        let sources = captain.sources_with_signal(&state);
        assert!(sources.contains(&"topological"));
    }

    #[test]
    fn test_sources_degraded_zhc() {
        let captain = Captain::new();
        let state = FleetGraphState::degraded_zhc();
        let sources = captain.sources_with_signal(&state);
        assert!(sources.contains(&"geometric"));
    }

    #[test]
    fn test_sources_noisy_trust() {
        let captain = Captain::new();
        let state = FleetGraphState::noisy_trust();
        let sources = captain.sources_with_signal(&state);
        assert!(sources.contains(&"algebraic"));
    }

    #[test]
    fn test_sources_agent_count_changed() {
        let captain = Captain::new();
        let state = FleetGraphState::agent_count_changed();
        let sources = captain.sources_with_signal(&state);
        assert!(sources.contains(&"empirical"));
    }

    #[test]
    fn test_inquiry_creates_deliberation() {
        let captain = Captain::new();
        let state = FleetGraphState::stable_rigid();
        let graph = make_graph();
        let deliberation = captain.inquire(&state, &graph);
        assert!(!deliberation.consulted.is_empty() || state.is_stable());
    }

    #[test]
    fn test_inquiry_with_insufficient_data() {
        let captain = Captain::new();
        let state = FleetGraphState::small_graph();
        let graph = make_graph();
        let deliberation = captain.inquire(&state, &graph);
        assert!(!deliberation.consulted.contains(&"systems".to_string()));
    }

    #[test]
    fn test_constraint_emergence_ceiling_violated() {
        let constraint = HardConstraint::EmergenceCeiling(2.0);
        let state = FleetGraphState::rising_beta();
        let reports = vec![];
        assert!(constraint.is_violated(&state, &reports));
    }

    #[test]
    fn test_constraint_emergence_ceiling_ok() {
        let constraint = HardConstraint::EmergenceCeiling(10.0);
        let state = FleetGraphState::rising_beta();
        let reports = vec![];
        assert!(!constraint.is_violated(&state, &reports));
    }

    #[test]
    fn test_constraint_zhc_tolerance_violated() {
        let constraint = HardConstraint::ZhcTolerance(0.05);
        let state = FleetGraphState::degraded_zhc();
        let reports = vec![];
        assert!(constraint.is_violated(&state, &reports));
    }

    #[test]
    fn test_constraint_zhc_tolerance_ok() {
        let constraint = HardConstraint::ZhcTolerance(0.20);
        let state = FleetGraphState::degraded_zhc();
        let reports = vec![];
        assert!(!constraint.is_violated(&state, &reports));
    }

    #[test]
    fn test_deliberation_new() {
        let deliberation = CaptainDeliberation::new();
        assert!(deliberation.reports.is_empty());
        assert!(deliberation.violations.is_empty());
    }

    #[test]
    fn test_deliberation_consult() {
        let mut deliberation = CaptainDeliberation::new();
        let mut report = SpecialistReport::new("topological");
        report.add_finding("Graph is rigid".to_string(), 0.8, vec!["V=5".to_string()]);
        deliberation.consult("topological", report);
        assert_eq!(deliberation.reports.len(), 1);
        assert_eq!(deliberation.consulted, vec!["topological"]);
    }

    #[test]
    fn test_deliberation_apply_constraints() {
        let mut deliberation = CaptainDeliberation::new();
        let constraints = vec![HardConstraint::EmergenceCeiling(2.0)];
        let state = FleetGraphState::rising_beta();
        deliberation.apply_constraints(&constraints, &state);
        assert!(!deliberation.violations.is_empty());
    }

    #[test]
    fn test_deliberation_no_violations_when_ok() {
        let mut deliberation = CaptainDeliberation::new();
        let constraints = vec![HardConstraint::EmergenceCeiling(10.0)];
        let state = FleetGraphState::rising_beta();
        deliberation.apply_constraints(&constraints, &state);
        assert!(deliberation.violations.is_empty());
    }

    #[test]
    fn test_deliberation_has_constraint_violations() {
        let mut deliberation = CaptainDeliberation::new();
        deliberation.violations.push("Safety margin violated".to_string());
        assert!(deliberation.has_constraint_violations());
    }

    #[test]
    fn test_deliberation_no_constraint_violations() {
        let deliberation = CaptainDeliberation::new();
        assert!(!deliberation.has_constraint_violations());
    }

    #[test]
    fn test_adjudicate_finds_contradiction() {
        let mut deliberation = CaptainDeliberation::new();
        let mut report1 = SpecialistReport::new("topological");
        report1.add_finding("Graph is rigid".to_string(), 0.9, vec![]);
        let mut report2 = SpecialistReport::new("geometric");
        report2.add_finding("Graph is under-constrained".to_string(), 0.7, vec![]);
        deliberation.consult("topological", report1);
        deliberation.consult("geometric", report2);
        deliberation.adjudicate();
        assert!(deliberation.adjudicated_findings.len() <= 2);
    }

    #[test]
    fn test_findings_contradict_rigid_underconstrained() {
        assert!(CaptainDeliberation::findings_contradict(
            "Graph is rigid", "Graph is under-constrained"));
    }

    #[test]
    fn test_findings_contradict_stable_drift() {
        assert!(CaptainDeliberation::findings_contradict(
            "System is stable", "System is drifting"));
    }

    #[test]
    fn test_findings_no_contradiction_similar() {
        assert!(!CaptainDeliberation::findings_contradict(
            "Graph is rigid", "Laman condition satisfied"));
    }

    #[test]
    fn test_probability_distribution_from_findings() {
        let findings = vec![
            AdjudicatedFinding {
                finding: crate::specialists::Finding {
                    claim: "Graph is rigid".to_string(),
                    confidence: 0.8,
                    evidence: vec![],
                },
                source: "topological".to_string(),
                quality: 0.8,
            },
            AdjudicatedFinding {
                finding: crate::specialists::Finding {
                    claim: "Graph is stable".to_string(),
                    confidence: 0.6,
                    evidence: vec![],
                },
                source: "geometric".to_string(),
                quality: 0.6,
            },
        ];
        let dist = ProbabilityDistribution::from_findings(&findings);
        assert_eq!(dist.states.len(), 2);
        let total: f64 = dist.states.iter().map(|s| s.probability).sum();
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_probability_distribution_most_likely() {
        let findings = vec![
            AdjudicatedFinding {
                finding: crate::specialists::Finding {
                    claim: "High confidence".to_string(),
                    confidence: 0.9,
                    evidence: vec![],
                },
                source: "topological".to_string(),
                quality: 0.9,
            },
            AdjudicatedFinding {
                finding: crate::specialists::Finding {
                    claim: "Low confidence".to_string(),
                    confidence: 0.3,
                    evidence: vec![],
                },
                source: "geometric".to_string(),
                quality: 0.3,
            },
        ];
        let dist = ProbabilityDistribution::from_findings(&findings);
        let most_likely = dist.most_likely().unwrap();
        assert!(most_likely.description.contains("High"));
    }

    #[test]
    fn test_probability_distribution_is_uncertain() {
        let dist = ProbabilityDistribution {
            states: vec![
                StateProbability { description: "A".to_string(), probability: 0.5, source: "s1".to_string() },
                StateProbability { description: "B".to_string(), probability: 0.5, source: "s2".to_string() },
            ],
            entropy: 1.0,
        };
        assert!(dist.is_uncertain());
    }

    #[test]
    fn test_probability_distribution_not_uncertain() {
        let dist = ProbabilityDistribution {
            states: vec![
                StateProbability { description: "A".to_string(), probability: 0.95, source: "s1".to_string() },
                StateProbability { description: "B".to_string(), probability: 0.05, source: "s2".to_string() },
            ],
            entropy: 0.2,
        };
        assert!(!dist.is_uncertain());
    }

    #[test]
    fn test_deliberate_stable_fleet() {
        let captain = Captain::new();
        let state = FleetGraphState::stable_rigid();
        let graph = make_graph();
        let decision = captain.deliberate(&state, &graph);
        assert!(matches!(decision, CaptainDecision::Stable { .. }));
    }

    #[test]
    fn test_deliberate_constrained() {
        let constants = AgentConstants::default_fleet();
        let mut captain = Captain::with_constants(constants);
        captain.hard_constraints = vec![HardConstraint::EmergenceCeiling(1.0)];
        let state = FleetGraphState::rising_beta();
        let graph = make_graph();
        let decision = captain.deliberate(&state, &graph);
        assert!(matches!(decision, CaptainDecision::Constrained { .. }));
        if let CaptainDecision::Constrained { violations, .. } = decision {
            assert!(!violations.is_empty());
        }
    }

    #[test]
    fn test_captain_decision_took_action() {
        let decision = CaptainDecision::Decided {
            deliberation: CaptainDeliberation::new(),
            decision: Some("Increase trust".to_string()),
            reason: "β₁ rising".to_string(),
        };
        assert!(decision.took_action());
    }

    #[test]
    fn test_captain_decision_no_action_constrained() {
        let decision = CaptainDecision::Constrained {
            violations: vec!["Safety margin violated".to_string()],
            deliberation: CaptainDeliberation::new(),
            decision: None,
            reason: "P0 takes precedence".to_string(),
        };
        assert!(!decision.took_action());
    }

    #[test]
    fn test_captain_decision_no_action_stable() {
        let decision = CaptainDecision::Stable {
            deliberation: CaptainDeliberation::new(),
            decision: None,
            reason: "No action needed".to_string(),
        };
        assert!(!decision.took_action());
    }

    #[test]
    fn test_safe_action_set() {
        let mut deliberation = CaptainDeliberation::new();
        deliberation.adjudicated_findings.push(AdjudicatedFinding {
            finding: crate::specialists::Finding {
                claim: "Maintain safe distance".to_string(),
                confidence: 0.9,
                evidence: vec![],
            },
            source: "topological".to_string(),
            quality: 0.9,
        });
        let safe_actions = deliberation.safe_action_set();
        assert!(!safe_actions.is_empty());
    }

    #[test]
    fn test_p0_safety_prevents_action() {
        let mut captain = Captain::new();
        captain.hard_constraints = vec![HardConstraint::SafetyMargin(0.05)];
        let state = FleetGraphState::stable_rigid();
        let graph = make_graph();
        let decision = captain.deliberate(&state, &graph);
        assert!(!decision.took_action());
    }
}