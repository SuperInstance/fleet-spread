//! PLATO tile output - writes analysis results as constraint tiles

use crate::specialists::SpecialistReport;
use crate::synthesis::SynthesisReport;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatoTile {
    #[serde(rename = "type")]
    pub tile_type: String,
    pub data: serde_json::Value,
    pub confidence: f64,
    pub graph_id: String,
    pub timestamp: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl PlatoTile {
    pub fn new(tile_type: &str, graph_id: &str) -> Self {
        Self {
            tile_type: tile_type.to_string(),
            data: serde_json::Value::Null,
            confidence: 0.0,
            graph_id: graph_id.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

/// Convert a specialist report into a PLATO tile
impl From<(&SpecialistReport, &str)> for PlatoTile {
    fn from((report, graph_id): (&SpecialistReport, &str)) -> Self {
        PlatoTile::new(
            &format!("fleet-spread.{}", report.specialist_id),
            graph_id,
        )
        .with_confidence(report.confidence)
        .with_data(serde_json::json!({
            "specialist_id": report.specialist_id,
            "findings": report.findings,
            "confidence": report.confidence,
            "unanswered": report.unanswered,
            "information_content": report.information_content(),
            "raw_data": report.raw_data,
        }))
    }
}

/// Convert synthesis report into a PLATO tile
impl From<(&SynthesisReport, &str)> for PlatoTile {
    fn from((synthesis, graph_id): (&SynthesisReport, &str)) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("robust_count".to_string(), serde_json::json!(synthesis.robust_findings.len()));
        metadata.insert("tension_count".to_string(), serde_json::json!(synthesis.tensions.len()));
        metadata.insert("blind_spot_count".to_string(), serde_json::json!(synthesis.blind_spots.len()));

        PlatoTile::new("fleet-spread.synthesis", graph_id)
            .with_confidence(synthesis.overall_confidence)
            .with_data(serde_json::json!({
                "synthesis_gain": synthesis.synthesis_gain,
                "overall_confidence": synthesis.overall_confidence,
                "robust_findings": synthesis.robust_findings,
                "tensions": synthesis.tensions,
                "blind_spots": synthesis.blind_spots,
            }))
            .with_metadata("robust_count", serde_json::json!(synthesis.robust_findings.len()))
            .with_metadata("tension_count", serde_json::json!(synthesis.tensions.len()))
    }
}

/// Write all tiles to a directory
pub struct TileWriter {
    output_dir: std::path::PathBuf,
}

impl TileWriter {
    pub fn new(output_dir: std::path::PathBuf) -> Self {
        std::fs::create_dir_all(&output_dir).ok();
        Self { output_dir }
    }

    /// Write a single tile to a JSON file
    pub fn write_tile(&self, tile: &PlatoTile) -> std::io::Result<std::path::PathBuf> {
        let filename = format!("{}_{}.json", tile.tile_type.replace('.', "_"), tile.graph_id);
        let path = self.output_dir.join(&filename);

        let json = serde_json::to_string_pretty(tile)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        std::fs::write(&path, json)?;
        Ok(path)
    }

    /// Write all specialist tiles from a synthesis report
    pub fn write_synthesis_tiles(&self, synthesis: &SynthesisReport, graph_id: &str) -> Vec<std::path::PathBuf> {
        let mut paths = Vec::new();

        // Write synthesis master tile
        let synthesis_tile = PlatoTile::from((synthesis, graph_id));
        if let Ok(path) = self.write_tile(&synthesis_tile) {
            paths.push(path);
        }

        // Write individual specialist tiles
        for report in &synthesis.specialist_reports {
            let tile = PlatoTile::from((report, graph_id));
            if let Ok(path) = self.write_tile(&tile) {
                paths.push(path);
            }
        }

        paths
    }

    /// Write a combined report with all tiles
    pub fn write_combined_report(&self, synthesis: &SynthesisReport, graph_id: &str) -> std::io::Result<std::path::PathBuf> {
        let mut tiles = Vec::new();

        // Synthesis tile
        tiles.push(PlatoTile::from((synthesis, graph_id)));

        // Specialist tiles
        for report in &synthesis.specialist_reports {
            tiles.push(PlatoTile::from((report, graph_id)));
        }

        let path = self.output_dir.join(format!("combined_{}.json", graph_id));
        let json = serde_json::to_string_pretty(&tiles)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        std::fs::write(&path, json)?;
        Ok(path)
    }
}

/// Format tiles for terminal output
pub fn format_tiles_markdown(synthesis: &SynthesisReport, graph_id: &str) -> String {
    let mut output = String::new();

    output.push_str(&format!("# Fleet Spread Analysis: {}\n\n", graph_id));
    output.push_str(&format!("**Overall Confidence:** {:.0}%  \n", synthesis.overall_confidence * 100.0));
    output.push_str(&format!("**Synthesis Gain:** {:.2}  \n\n", synthesis.synthesis_gain));

    // Robust Findings
    if !synthesis.robust_findings.is_empty() {
        output.push_str("## Robust Findings (≥3 specialists agree)\n\n");
        for finding in &synthesis.robust_findings {
            output.push_str(&format!(
                "**{}** ({:.0}% confidence)  \n",
                finding.claim,
                finding.confidence * 100.0
            ));
            output.push_str(&format!("Supported by: {}  \n\n", finding.supporting_specialists.join(", ")));
        }
    }

    // Tensions
    if !synthesis.tensions.is_empty() {
        output.push_str("## Tensions (specialist disagreements)\n\n");
        for tension in &synthesis.tensions {
            output.push_str(&format!(
                "**{} vs {}:** {}  \n\n",
                tension.specialist_a,
                tension.specialist_b,
                tension.description
            ));
        }
    }

    // Blind Spots
    if !synthesis.blind_spots.is_empty() {
        output.push_str("## Blind Spots (unaddressed questions)\n\n");
        for blind in &synthesis.blind_spots {
            output.push_str(&format!("- {}  \n", blind));
        }
        output.push('\n');
    }

    // Per-specialist summaries
    output.push_str("## Specialist Reports\n\n");
    for report in &synthesis.specialist_reports {
        output.push_str(&format!("### {} ({:.0}% confidence)\n\n", report.specialist_id, report.confidence * 100.0));
        for finding in &report.findings {
            output.push_str(&format!("- **{}** ({:.0}%)  \n", finding.claim, finding.confidence * 100.0));
        }
        if !report.unanswered.is_empty() {
            output.push_str("\n_Unanswered:_  \n");
            for q in &report.unanswered {
                output.push_str(&format!("- {}\n", q));
            }
        }
        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_creation() {
        let tile = PlatoTile::new("fleet-spread.test", "graph-1")
            .with_confidence(0.85)
            .with_data(serde_json::json!({"key": "value"}));

        assert_eq!(tile.tile_type, "fleet-spread.test");
        assert_eq!(tile.confidence, 0.85);
        assert_eq!(tile.graph_id, "graph-1");
    }
}
