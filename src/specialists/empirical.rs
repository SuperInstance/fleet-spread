//! S5: Empirical Specialist
//! Analyzes actual trust values from PLATO room, detecting anomalies and drift

use crate::graph::FleetGraph;
use crate::specialists::{Specialist, SpecialistReport};
use std::collections::HashMap;

pub struct EmpiricalSpecialist {
    anomaly_threshold: f64,
    drift_threshold: f64,
    plato_room_url: Option<String>,
}

impl EmpiricalSpecialist {
    pub fn new() -> Self {
        Self {
            anomaly_threshold: 2.0, // 2 standard deviations
            drift_threshold: 0.2,   // 20% change
            plato_room_url: None,
        }
    }

    pub fn with_plato_url(mut self, url: String) -> Self {
        self.plato_room_url = Some(url);
        self
    }

    /// Query PLATO room for historical trust data (placeholder - requires HTTP client)
    #[allow(dead_code)]
    async fn query_plato_room(&self, _graph_id: &str) -> Result<PlatoRoomData, String> {
        Err("PLATO room query requires HTTP client - not available in current build".to_string())
    }

    /// Analyze trust value distribution
    fn analyze_distribution(&self, graph: &FleetGraph) -> TrustDistribution {
        let values: Vec<f64> = graph.edges.iter().map(|e| e.trust.value).collect();
        let n = values.len() as f64;

        if n == 0.0 {
            return TrustDistribution {
                count: 0,
                mean: 0.0,
                std: 0.0,
                min: 0.0,
                max: 0.0,
                skewness: 0.0,
            };
        }

        let mean = values.iter().sum::<f64>() / n;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let std = variance.sqrt();
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        // Skewness: measure of asymmetry
        let skewness = if std > 1e-10 {
            values.iter().map(|v| ((v - mean) / std).powi(3)).sum::<f64>() / n
        } else {
            0.0
        };

        TrustDistribution { count: n as usize, mean, std, min, max, skewness }
    }

    /// Find anomalous trust values (deviation from historical mean > threshold * std)
    fn find_anomalies(&self, graph: &FleetGraph) -> Vec<AnomalyReport> {
        let mut anomalies = Vec::new();

        for edge in &graph.edges {
            if edge.trust.history.len() < 2 {
                continue; // Need at least 2 history points
            }

            let history = &edge.trust.history;
            let mean = history.iter().sum::<f64>() / history.len() as f64;
            let variance = history.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / history.len() as f64;
            let std = variance.sqrt();

            if std < 1e-10 {
                continue; // No meaningful variance
            }

            let z_score = (edge.trust.value - mean).abs() / std;

            if z_score > self.anomaly_threshold {
                anomalies.push(AnomalyReport {
                    edge_id: format!("{}->{}", edge.from, edge.to),
                    current_value: edge.trust.value,
                    historical_mean: mean,
                    historical_std: std,
                    z_score,
                    deviation: edge.trust.value - mean,
                });
            }
        }

        anomalies
    }

    /// Detect trust drift over time
    fn detect_drift(&self, graph: &FleetGraph) -> Vec<DriftReport> {
        let mut drifts = Vec::new();

        for edge in &graph.edges {
            if edge.trust.history.len() < 3 {
                continue;
            }

            let history = &edge.trust.history;
            let first_half = &history[..history.len() / 2];
            let second_half = &history[history.len() / 2..];

            let mean1 = first_half.iter().sum::<f64>() / first_half.len() as f64;
            let mean2 = second_half.iter().sum::<f64>() / second_half.len() as f64;

            let drift = (mean2 - mean1).abs();
            let drift_ratio = drift / mean1.abs().max(0.1);

            if drift_ratio > self.drift_threshold {
                drifts.push(DriftReport {
                    edge_id: format!("{}->{}", edge.from, edge.to),
                    first_half_mean: mean1,
                    second_half_mean: mean2,
                    absolute_drift: drift,
                    relative_drift: drift_ratio,
                    direction: if mean2 > mean1 { "increasing" } else { "decreasing" },
                });
            }
        }

        drifts
    }

    /// Compare with expected trust patterns
    fn check_trust_patterns(&self, graph: &FleetGraph) -> Vec<PatternViolation> {
        let mut violations = Vec::new();
        let distribution = self.analyze_distribution(graph);

        // Check for suspicious values
        for edge in &graph.edges {
            // Perfect trust is suspicious
            if (edge.trust.value - 1.0).abs() < 0.001 {
                violations.push(PatternViolation {
                    edge_id: format!("{}->{}", edge.from, edge.to),
                    pattern: "perfect_trust".to_string(),
                    severity: "medium".to_string(),
                    description: "Trust value is exactly 1.0 - unusual in real deployments".to_string(),
                });
            }

            // Zero trust is suspicious
            if edge.trust.value.abs() < 0.001 {
                violations.push(PatternViolation {
                    edge_id: format!("{}->{}", edge.from, edge.to),
                    pattern: "zero_trust".to_string(),
                    severity: "medium".to_string(),
                    description: "Trust value is exactly 0 - may indicate disconnected edge".to_string(),
                });
            }

            // Low confidence with high trust
            if edge.trust.confidence < 0.3 && edge.trust.value > 0.7 {
                violations.push(PatternViolation {
                    edge_id: format!("{}->{}", edge.from, edge.to),
                    pattern: "high_trust_low_confidence".to_string(),
                    severity: "high".to_string(),
                    description: "High trust value with low confidence - inconsistent".to_string(),
                });
            }

            // Value outside expected range
            if edge.trust.value < distribution.mean - 3.0 * distribution.std ||
               edge.trust.value > distribution.mean + 3.0 * distribution.std {
                violations.push(PatternViolation {
                    edge_id: format!("{}->{}", edge.from, edge.to),
                    pattern: "outlier_value".to_string(),
                    severity: "high".to_string(),
                    description: format!("Value {:.3} is >3σ from mean {:.3}", edge.trust.value, distribution.mean),
                });
            }
        }

        violations
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PlatoRoomData {
    pub graph_id: String,
    pub trust_history: HashMap<String, Vec<f64>>,
    pub last_update: String,
}

#[derive(Debug, serde::Serialize)]
struct TrustDistribution {
    count: usize,
    mean: f64,
    std: f64,
    min: f64,
    max: f64,
    skewness: f64,
}

#[derive(Debug, serde::Serialize)]
struct AnomalyReport {
    edge_id: String,
    current_value: f64,
    historical_mean: f64,
    historical_std: f64,
    z_score: f64,
    deviation: f64,
}

#[derive(Debug, serde::Serialize)]
struct DriftReport {
    edge_id: String,
    first_half_mean: f64,
    second_half_mean: f64,
    absolute_drift: f64,
    relative_drift: f64,
    direction: &'static str,
}

#[derive(Debug, serde::Serialize)]
struct PatternViolation {
    edge_id: String,
    pattern: String,
    severity: String,
    description: String,
}

impl Default for EmpiricalSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl Specialist for EmpiricalSpecialist {
    fn analyze(&self, graph: &FleetGraph) -> SpecialistReport {
        let mut report = SpecialistReport::new("empirical");

        let distribution = self.analyze_distribution(graph);

        // Distribution analysis
        report.add_finding(
            format!("Trust distribution: μ={:.3}, σ={:.3}, range=[{:.3}, {:.3}]",
                distribution.mean, distribution.std, distribution.min, distribution.max),
            0.85,
            vec![format!("{} edges analyzed", distribution.count)],
        );

        // Skewness analysis
        if distribution.skewness.abs() > 0.5 {
            let direction = if distribution.skewness > 0.0 { "right-skewed" } else { "left-skewed" };
            report.add_finding(
                format!("Trust distribution is {} (skewness={:.3})", direction, distribution.skewness),
                0.7,
                vec!["Distribution asymmetry detected".to_string()],
            );
        }

        // Pattern violations
        let violations = self.check_trust_patterns(graph);
        let high_severity = violations.iter().filter(|v| v.severity == "high").count();

        if !violations.is_empty() {
            report.add_finding(
                format!("Found {} trust pattern violations ({} high severity)", violations.len(), high_severity),
                0.8,
                vec![format!("{:.1}% of edges", 100.0 * violations.len() as f64 / graph.e().max(1) as f64)],
            );
        }

        // Anomaly detection
        let anomalies = self.find_anomalies(graph);
        if !anomalies.is_empty() {
            report.add_finding(
                format!("Found {} anomalous trust values (>{:.1}σ from history)", anomalies.len(), self.anomaly_threshold),
                0.85,
                anomalies.iter().map(|a| format!("{}: z={:.2}", a.edge_id, a.z_score)).collect(),
            );
        } else {
            report.add_finding(
                "No anomalous trust values detected".to_string(),
                0.75,
                vec!["All trust values within expected range".to_string()],
            );
        }

        // Drift detection
        let drifts = self.detect_drift(graph);
        if !drifts.is_empty() {
            report.add_finding(
                format!("Found {} edges with significant trust drift", drifts.len()),
                0.8,
                drifts.iter().map(|d| format!("{}: {:.1}% {}", d.edge_id, d.relative_drift * 100.0, d.direction)).collect(),
            );
        }

        // Check for historical data
        let edges_with_history = graph.edges.iter().filter(|e| !e.trust.history.is_empty()).count();
        if edges_with_history == 0 {
            report.add_unanswered("No historical trust data available for drift detection".to_string());
        }

        // Try PLATO room query if configured
        if let Some(_) = &self.plato_room_url {
            // In real implementation, this would be async
            report.add_unanswered("PLATO room query not implemented in sync context".to_string());
        }

        report.set_raw_data(serde_json::json!({
            "distribution": distribution,
            "pattern_violations": violations,
            "anomalies": anomalies,
            "drifts": drifts,
            "edges_with_history": edges_with_history,
        }));

        let mut confidence: f64 = 0.7;
        if anomalies.is_empty() && violations.is_empty() {
            confidence += 0.1;
        }
        if edges_with_history < graph.e() / 2 {
            confidence -= 0.15;
        }
        report.confidence = confidence.max(0.4);

        report
    }

    fn id(&self) -> &'static str {
        "empirical"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distribution_analysis() {
        let specialist = EmpiricalSpecialist::new();
        let graph = crate::test_helpers::make_small_rigid();
        let report = specialist.analyze(&graph);

        assert_eq!(report.specialist_id, "empirical");
        assert!(report.confidence > 0.5);
    }
}
