//! Specialist modules for fleet graph analysis

mod topological;
mod geometric;
mod algebraic;
mod systems;
mod empirical;

pub use topological::TopologicalSpecialist;
pub use geometric::GeometricSpecialist;
pub use algebraic::AlgebraicSpecialist;
pub use systems::SystemsSpecialist;
pub use empirical::EmpiricalSpecialist;

use crate::graph::FleetGraph;
use serde::{Deserialize, Serialize};

/// A finding from a specialist analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    pub claim: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

/// Output from any specialist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistReport {
    pub specialist_id: String,
    pub findings: Vec<Finding>,
    pub confidence: f64,
    pub unanswered: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_data: Option<serde_json::Value>,
}

impl SpecialistReport {
    pub fn new(id: &'static str) -> Self {
        Self {
            specialist_id: id.to_string(),
            findings: Vec::new(),
            confidence: 0.0,
            unanswered: Vec::new(),
            raw_data: None,
        }
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn add_finding(&mut self, claim: String, confidence: f64, evidence: Vec<String>) {
        self.findings.push(Finding { claim, confidence, evidence });
    }

    pub fn add_unanswered(&mut self, question: String) {
        self.unanswered.push(question);
    }

    pub fn set_raw_data(&mut self, data: serde_json::Value) {
        self.raw_data = Some(data);
    }

    /// Calculate information content (entropy-like measure)
    pub fn information_content(&self) -> f64 {
        if self.findings.is_empty() {
            return 0.0;
        }
        let finding_info: f64 = self.findings.iter()
            .map(|f| f.confidence * f.evidence.len() as f64)
            .sum();
        let unanswered_penalty = self.unanswered.len() as f64 * 0.1;
        (finding_info - unanswered_penalty).max(0.0)
    }
}

pub trait Specialist {
    fn analyze(&self, graph: &FleetGraph) -> SpecialistReport;
    fn id(&self) -> &'static str;
}
