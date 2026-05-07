//! Test utilities for creating sample fleet graphs
//! This module is public for use by integration tests

use crate::graph::{FleetGraph, TrustValue, Vertex, Edge};
use std::collections::HashMap;

/// Create a small rigid fleet graph (V=5, E=7, C=1: Laman rigid)
pub fn make_small_rigid() -> FleetGraph {
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

    FleetGraph::new("test-small-rigid".into(), vertices, edges)
}

/// Create an over-connected fleet graph (V=5, E=20)
pub fn make_over_connected() -> FleetGraph {
    let vertices = (0..5).map(|i| Vertex {
        id: format!("agent-{}", i),
        metadata: HashMap::new(),
    }).collect();

    let mut edges = Vec::new();

    // Complete graph K5 (10 edges)
    for i in 0..5 {
        for j in (i+1)..5 {
            edges.push(Edge {
                from: format!("agent-{}", i),
                to: format!("agent-{}", j),
                trust: TrustValue::new(0.5, 0.7),
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

    FleetGraph::new("test-over-connected".into(), vertices, edges)
}

/// Create a default test graph (alias for make_small_rigid)
pub fn make_graph() -> FleetGraph {
    make_small_rigid()
}

/// Create a disconnected fleet graph (V=8, 2 components)
pub fn make_disconnected() -> FleetGraph {
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

    FleetGraph::new("test-disconnected".into(), vertices, all_edges)
}
