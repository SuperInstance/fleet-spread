//! fleet-spread CLI
//!
//! Run fleet graph analysis across 5 specialist dimensions

use std::collections::HashMap;
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use fleet_spread::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "fleet-spread")]
#[command(about = "Fleet graph analysis across 5 specialist dimensions")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a fleet graph from a JSON file
    Analyze {
        /// Path to graph JSON file
        #[arg(short, long)]
        input: Option<PathBuf>,
        /// Output directory for tiles
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,
        /// Git commit the results
        #[arg(short, long)]
        commit: bool,
        /// Enable JSON output
        #[arg(short, long)]
        json: bool,
    },
    /// Run built-in test cases
    Test {
        /// Specific test to run (small-rigid, over-connected, disconnected, all)
        #[arg(default_value = "all")]
        test_case: String,
    },
    /// Generate sample graph
    Sample {
        /// Graph type (small-rigid, over-connected, disconnected)
        #[arg(default_value = "small-rigid")]
        graph_type: String,
        /// Output path
        #[arg(short, long, default_value = "sample-graph.json")]
        output: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Analyze { input, output, commit, json } => {
            let graph = if let Some(path) = input {
                read_graph_from_file(&path)?
            } else {
                // Read from stdin
                let stdin = std::io::read_to_string(std::io::stdin())?;
                serde_json::from_str(&stdin)?
            };

            let synthesis = run_analysis(&graph);
            let graph_type = identify_graph_type(&graph);

            if json {
                println!("{}", serde_json::to_string_pretty(&synthesis)?);
            } else {
                println!("{}", interpret_synthesis(&synthesis, graph_type));
            }

            // Write tiles
            let writer = TileWriter::new(output.clone());
            let tile_path = writer.write_combined_report(&synthesis, &graph.id)?;
            println!("\nTiles written to: {:?}", tile_path);

            // Quality assessment
            let quality = QualityReport::assess(&synthesis);
            println!("\nQuality Assessment: {} ({:.0}%)",
                quality.assessment, quality.overall_score * 100.0);

            // Git commit if requested
            if commit {
                if git_commit::git_available() {
                    let msg = git_commit::generate_commit_message(
                        &graph.id,
                        synthesis.robust_findings.len(),
                        synthesis.tensions.len(),
                        synthesis.blind_spots.len(),
                        synthesis.synthesis_gain,
                    );

                    let committer = git_commit::GitCommitter::new(".");
                    if let Ok(has_changes) = committer.has_changes() {
                        if has_changes {
                            committer.add(&tile_path)?;
                            if let Ok(hash) = committer.commit(&msg) {
                                println!("\nCommitted: {}", hash);
                            }
                        }
                    }
                } else {
                    eprintln!("Git not available, skipping commit");
                }
            }

            Ok(())
        }

        Commands::Test { test_case } => {
            let test_cases = match test_case.as_str() {
                "small-rigid" => vec!["small-rigid"],
                "over-connected" => vec!["over-connected"],
                "disconnected" => vec!["disconnected"],
                _ => vec!["small-rigid", "over-connected", "disconnected"],
            };

            for tc in test_cases {
                println!("\n{}", "=".repeat(60));
                println!("TEST: {}", tc.to_uppercase());
                println!("{}\n", "=".repeat(60));

                let graph = match tc {
                    "small-rigid" => create_small_rigid(),
                    "over-connected" => create_over_connected(),
                    "disconnected" => create_disconnected(),
                    _ => continue,
                };

                println!("Graph: V={}, E={}, C={}", graph.v(), graph.e(), graph.components());
                println!("Betti β₁ = {}\n", graph.betti_1());

                let synthesis = run_analysis(&graph);
                let graph_type = identify_graph_type(&graph);

                println!("{}", interpret_synthesis(&synthesis, graph_type));

                let quality = QualityReport::assess(&synthesis);
                println!("\nQuality: {} ({:.0}%)\n", quality.assessment, quality.overall_score * 100.0);
                println!("  Novelty:      {:.0}%", quality.novelty_score * 100.0);
                println!("  Correctness: {:.0}%", quality.correctness_score * 100.0);
                println!("  Usefulness:  {:.0}%", quality.usefulness_score * 100.0);
                println!("  Completeness:{:.0}%", quality.completeness_score * 100.0);
            }

            Ok(())
        }

        Commands::Sample { graph_type, output } => {
            let graph = match graph_type.as_str() {
                "small-rigid" => create_small_rigid(),
                "over-connected" => create_over_connected(),
                "disconnected" => create_disconnected(),
                _ => anyhow::bail!("Unknown graph type: {}", graph_type),
            };

            let json = serde_json::to_string_pretty(&graph)?;
            std::fs::write(&output, json)?;
            println!("Sample graph written to: {:?}", output);
            Ok(())
        }
    }
}

fn read_graph_from_file(path: &PathBuf) -> anyhow::Result<FleetGraph> {
    let content = std::fs::read_to_string(path)?;
    let graph: FleetGraph = serde_json::from_str(&content)?;
    Ok(graph)
}

// Test graph builders
fn create_small_rigid() -> FleetGraph {
    let vertices = (0..5).map(|i| Vertex {
        id: format!("agent-{}", i),
        metadata: HashMap::new(),
    }).collect();

    let edges = vec![
        Edge { from: "agent-0".into(), to: "agent-1".into(), trust: TrustValue::new(0.9, 0.8) },
        Edge { from: "agent-1".into(), to: "agent-2".into(), trust: TrustValue::new(0.85, 0.8) },
        Edge { from: "agent-2".into(), to: "agent-0".into(), trust: TrustValue::new(0.88, 0.8) },
        Edge { from: "agent-0".into(), to: "agent-3".into(), trust: TrustValue::new(0.75, 0.8) },
        Edge { from: "agent-3".into(), to: "agent-4".into(), trust: TrustValue::new(0.82, 0.8) },
        Edge { from: "agent-4".into(), to: "agent-2".into(), trust: TrustValue::new(0.78, 0.8) },
        Edge { from: "agent-1".into(), to: "agent-4".into(), trust: TrustValue::new(0.71, 0.8) },
    ];

    FleetGraph::new("small-rigid".into(), vertices, edges)
}

fn create_over_connected() -> FleetGraph {
    let vertices = (0..5).map(|i| Vertex {
        id: format!("agent-{}", i),
        metadata: HashMap::new(),
    }).collect();

    let mut edges = Vec::new();

    // Complete graph K5 (10 edges) + extra edges
    for i in 0..5 {
        for j in (i+1)..5 {
            edges.push(Edge {
                from: format!("agent-{}", i),
                to: format!("agent-{}", j),
                trust: TrustValue::new(0.5 + (i * j) as f64 * 0.05, 0.7),
            });
        }
    }

    // Add 10 more edges to exceed 2V-3 = 7
    for i in 0..5 {
        for _ in 0..2 {
            let j = (i + 1) % 5;
            edges.push(Edge {
                from: format!("agent-{}", i),
                to: format!("agent-{}", j),
                trust: TrustValue::new(0.4, 0.5),
            });
        }
    }

    FleetGraph::new("over-connected".into(), vertices, edges)
}

fn create_disconnected() -> FleetGraph {
    let vertices = (0..8).map(|i| Vertex {
        id: format!("agent-{}", i),
        metadata: HashMap::new(),
    }).collect();

    // Component A: 5 vertices, 7 edges (rigid)
    let edges_a = vec![
        Edge { from: "agent-0".into(), to: "agent-1".into(), trust: TrustValue::new(0.9, 0.8) },
        Edge { from: "agent-1".into(), to: "agent-2".into(), trust: TrustValue::new(0.85, 0.8) },
        Edge { from: "agent-2".into(), to: "agent-0".into(), trust: TrustValue::new(0.88, 0.8) },
        Edge { from: "agent-0".into(), to: "agent-3".into(), trust: TrustValue::new(0.75, 0.8) },
        Edge { from: "agent-3".into(), to: "agent-4".into(), trust: TrustValue::new(0.82, 0.8) },
        Edge { from: "agent-4".into(), to: "agent-2".into(), trust: TrustValue::new(0.78, 0.8) },
        Edge { from: "agent-1".into(), to: "agent-4".into(), trust: TrustValue::new(0.71, 0.8) },
    ];

    // Component B: 3 vertices, 2 edges (under-constrained)
    let edges_b = vec![
        Edge { from: "agent-5".into(), to: "agent-6".into(), trust: TrustValue::new(0.6, 0.7) },
        Edge { from: "agent-6".into(), to: "agent-7".into(), trust: TrustValue::new(0.65, 0.7) },
    ];

    let mut all_edges = edges_a;
    all_edges.extend(edges_b);

    FleetGraph::new("disconnected".into(), vertices, all_edges)
}
