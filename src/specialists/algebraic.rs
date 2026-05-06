//! S3: Algebraic Specialist
//! Analyzes encoding stability using Pythagorean48 representation

use crate::graph::FleetGraph;
use crate::specialists::{Specialist, SpecialistReport};

pub struct AlgebraicSpecialist {
    precision_bits: usize,
    max_hops: usize,
}

impl AlgebraicSpecialist {
    pub fn new() -> Self {
        Self {
            precision_bits: 48,
            max_hops: 5,
        }
    }

    pub fn with_precision(mut self, bits: usize) -> Self {
        self.precision_bits = bits;
        self
    }

    /// Pythagorean48 encoding: maps trust [0,1] to a point on a 48-bit precision sphere
    /// The encoding uses trigonometric representation: trust -> (cos(θ), sin(θ)) at 48-bit precision
    fn pythagorean_encode(&self, trust: f64) -> (f64, f64) {
        // Normalize trust to angle [0, π]
        let theta = trust * std::f64::consts::PI;
        let cos_val = theta.cos();
        let sin_val = theta.sin();

        // Apply 48-bit precision truncation
        let factor = 2f64.powi(self.precision_bits as i32);
        let truncated_cos = (cos_val * factor).trunc() / factor;
        let truncated_sin = (sin_val * factor).trunc() / factor;

        // Renormalize to unit length (compensate for truncation)
        let len = (truncated_cos * truncated_cos + truncated_sin * truncated_sin).sqrt();
        if len > 1e-10 {
            (truncated_cos / len, truncated_sin / len)
        } else {
            (truncated_cos, truncated_sin)
        }
    }

    /// Simulate N-hop trust propagation and measure encoding drift
    fn measure_drift(&self, start_trust: f64, hops: usize) -> DriftResult {
        let (mut x, mut y) = self.pythagorean_encode(start_trust);

        for _ in 0..hops {
            // Propagate: multiply trust vectors
            let new_trust = (x * x + y * y).sqrt();
            let (nx, ny) = self.pythagorean_encode(new_trust);
            x = nx;
            y = ny;
        }

        // Drift = angular distance from starting point
        let final_trust = (x * x + y * y).sqrt();
        let drift = (start_trust - final_trust).abs();

        DriftResult {
            start_trust,
            hops,
            final_trust,
            drift,
            final_encoding: (x, y),
        }
    }

    /// Analyze encoding stability across all edges
    fn analyze_encoding_stability(&self, graph: &FleetGraph) -> EncodingAnalysis {
        let mut results = Vec::new();

        for edge in &graph.edges {
            let encoded = self.pythagorean_encode(edge.trust.value);
            results.push(EdgeEncoding {
                edge_id: format!("{}->{}", edge.from, edge.to),
                trust: edge.trust.value,
                confidence: edge.trust.confidence,
                encoding_x: encoded.0,
                encoding_y: encoded.1,
                encoding_magnitude: (encoded.0.powi(2) + encoded.1.powi(2)).sqrt(),
            });
        }

        // Stability = how close magnitudes are to 1.0
        let magnitudes: Vec<f64> = results.iter().map(|r| r.encoding_magnitude).collect();
        let mean_magnitude = magnitudes.iter().sum::<f64>() / magnitudes.len() as f64;
        let stability = if (mean_magnitude - 1.0).abs() < 0.01 {
            1.0
        } else {
            (1.0 - (mean_magnitude - 1.0).abs()).max(0.0)
        };

        EncodingAnalysis {
            edges_analyzed: results.len(),
            stability_score: stability,
            mean_magnitude,
            edge_encodings: results,
        }
    }

    /// Multi-hop drift analysis
    fn analyze_hop_drift(&self, graph: &FleetGraph) -> Vec<HopDriftSummary> {
        let trust_values: Vec<f64> = graph.edges.iter().map(|e| e.trust.value).collect();
        let unique_trusts: Vec<f64> = {
            let mut v = trust_values.clone();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v.deduce();
            v
        };

        unique_trusts.iter().map(|&trust| {
            let results: Vec<DriftResult> = (1..=self.max_hops)
                .map(|hops| self.measure_drift(trust, hops))
                .collect();

            HopDriftSummary {
                trust_value: trust,
                drift_by_hop: results.iter().map(|r| r.drift).collect(),
                max_drift: results.iter().map(|r| r.drift).fold(0.0f64, f64::max),
            }
        }).collect()
    }
}

#[derive(Debug, serde::Serialize)]
struct DriftResult {
    start_trust: f64,
    hops: usize,
    final_trust: f64,
    drift: f64,
    final_encoding: (f64, f64),
}

#[derive(Debug, serde::Serialize)]
struct EdgeEncoding {
    edge_id: String,
    trust: f64,
    confidence: f64,
    encoding_x: f64,
    encoding_y: f64,
    encoding_magnitude: f64,
}

#[derive(Debug, serde::Serialize)]
struct EncodingAnalysis {
    edges_analyzed: usize,
    stability_score: f64,
    mean_magnitude: f64,
    edge_encodings: Vec<EdgeEncoding>,
}

#[derive(Debug, serde::Serialize)]
struct HopDriftSummary {
    trust_value: f64,
    drift_by_hop: Vec<f64>,
    max_drift: f64,
}

impl Default for AlgebraicSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl Specialist for AlgebraicSpecialist {
    fn analyze(&self, graph: &FleetGraph) -> SpecialistReport {
        let mut report = SpecialistReport::new("algebraic");

        if graph.edges.is_empty() {
            report.add_unanswered("No edges to analyze - cannot compute encoding".to_string());
            report.add_finding(
                "Graph has no edges - encoding analysis impossible".to_string(),
                0.9,
                vec!["edges.is_empty()".to_string()],
            );
            report.confidence = 0.5;
            return report;
        }

        // Encoding stability analysis
        let encoding_analysis = self.analyze_encoding_stability(graph);

        let stability = encoding_analysis.stability_score;
        if stability > 0.9 {
            report.add_finding(
                format!("Encoding is highly stable (score: {:.2})", stability),
                0.85,
                vec![format!("mean_magnitude = {:.6}", encoding_analysis.mean_magnitude)],
            );
        } else if stability > 0.7 {
            report.add_finding(
                format!("Encoding has moderate stability (score: {:.2})", stability),
                0.75,
                vec![format!("mean_magnitude = {:.6}", encoding_analysis.mean_magnitude)],
            );
        } else {
            report.add_finding(
                format!("Encoding stability is low (score: {:.2})", stability),
                0.8,
                vec![format!("mean_magnitude = {:.6}", encoding_analysis.mean_magnitude)],
            );
        }

        // Multi-hop drift analysis
        let hop_drift = self.analyze_hop_drift(graph);

        let avg_max_drift: f64 = hop_drift.iter().map(|h| h.max_drift).sum::<f64>() / hop_drift.len() as f64;
        let max_drift_any = hop_drift.iter().map(|h| h.max_drift).fold(0.0f64, f64::max);

        report.add_finding(
            format!("Max drift after {} hops: avg={:.4}, worst={:.4}", self.max_hops, avg_max_drift, max_drift_any),
            0.8,
            vec![format!("drift analysis on {} trust values", hop_drift.len())],
        );

        // Precision analysis
        if self.precision_bits < 32 {
            report.add_unanswered(
                format!("Low precision ({}) may cause unreliable drift measurements", self.precision_bits)
            );
        }

        let drift_growth_rate = if hop_drift.len() > 1 {
            let first = hop_drift[0].drift_by_hop.last().copied().unwrap_or(0.0);
            let last = hop_drift.last().unwrap().drift_by_hop.last().copied().unwrap_or(0.0);
            (last - first).max(0.0)
        } else {
            0.0
        };

        if drift_growth_rate > 0.5 {
            report.add_finding(
                format!("Significant encoding drift with distance (rate: {:.3})", drift_growth_rate),
                0.85,
                vec!["High drift indicates trust propagation degrades".to_string()],
            );
        } else if drift_growth_rate < 0.1 {
            report.add_finding(
                "Encoding drift is negligible with distance".to_string(),
                0.8,
                vec!["Trust propagation is stable".to_string()],
            );
        }

        report.set_raw_data(serde_json::json!({
            "precision_bits": self.precision_bits,
            "max_hops": self.max_hops,
            "encoding_analysis": encoding_analysis,
            "hop_drift": hop_drift,
            "drift_growth_rate": drift_growth_rate,
        }));

        let mut confidence: f64 = 0.75;
        if stability > 0.9 {
            confidence += 0.1;
        }
        if self.precision_bits < 32 {
            confidence -= 0.2;
        }
        report.confidence = confidence.max(0.4);

        report
    }

    fn id(&self) -> &'static str {
        "algebraic"
    }
}

trait Deduce {
    fn deduce(&mut self);
}

impl<T: PartialEq> Deduce for Vec<T> {
    fn deduce(&mut self) {
        self.dedup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pythagorean_encoding() {
        let specialist = AlgebraicSpecialist::new();

        let (x, y) = specialist.pythagorean_encode(0.5);
        assert!((x.powi(2) + y.powi(2) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_encoding_stability() {
        let specialist = AlgebraicSpecialist::new();
        let graph = crate::test_helpers::make_small_rigid();
        let report = specialist.analyze(&graph);

        assert_eq!(report.specialist_id, "algebraic");
        assert!(report.confidence > 0.5);
    }
}
