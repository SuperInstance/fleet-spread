# Fleet Coordinate Bridge

Developer guide for `fleet_coordinate_bridge.rs`.

## 1. What This Bridge Does

The `CoordinateBridge` wires **fleet-coordinate's mathematical rigor** into **fleet-spread's captain inquiry engine**. It converts a fleet-coordinate `FleetGraph` (agents numbered 0..V with positions + neighbor lists) into a fleet-spread `FleetGraph` (string IDs like `"agent-1"`, full edges with trust values), then runs either a full captain deliberation or a fast coordinate-only consistency check.

In plain terms: **you have a rigid mathematical fleet → you want captain-level fleet intelligence**.

## 2. When to Use It

| Scenario | Name | Use |
|----------|------|-----|
| **Hot-path fleet health check** | `quick_check()` | Tight loop checking fleet health every N seconds. No captain needed—just `is_rigid` + `emergence_detected`. Sub-millisecond. |
| **Captain deliberation on coordinate graph** | `analyze()` | Full captain inquiry: graph → state → specialist signal → captain decision. Used when something changed and you need a recommendation. |
| **Extending an existing Captain** | `CaptainCoordinateExt` trait | Adds `inquire_coordinate()` to any `Captain`. Drops a coordinate graph straight into captain deliberation. |
| **State fusion** | `build_state()` | Combine fleet-coordinate rigidity result + ZHC residual + trust entropy into a single `FleetGraphState` for the gate/captain. |
| **Graph conversion** | `convert_from_coordinate()` | Convert once, use the spread graph anywhere (gate, specialists, captain). |

## 3. The Two Modes

### `analyze()` — Full Pipeline

```
fleet-coordinate FleetGraph
    → convert_from_coordinate() → fleet-spread FleetGraph
    → check_laman_rigidity()    → RigidityResult
    → build_state()            → FleetGraphState
    → all_with_signal()        → Vec<Specialist>
    → captain.deliberate()     → CaptainDecision
```

Returns `CoordinateAnalysis` with:
- `rigidity` — Laman rigidity + β₁ + max_neighbors
- `state` — FleetGraphState (V, E, β₁, ZHC residual, trust entropy)
- `signal_sources` — which specialists have signal given this state
- `decision` — captain's full decision (reports, consulted, violations)

Use when: a fleet event happened, topology changed, or you need the captain's recommendation.

### `quick_check()` — Coordinate Math Only

```
fleet-coordinate FleetGraph
    → check_laman_rigidity()     → RigidityResult
    → EmergenceDetector::detect() → EmergenceResult
```

Returns `QuickConsistencyResult` with:
- `is_rigid` — Laman rigidity boolean
- `emergence_detected` — cluster emergence flag
- `beta_1` — H¹ dimension
- `max_neighbors` — max degree in graph

No captain. No specialists. No gate. Just coordinate math.

Use when: heartbeat health checks, tight loops, fast consistency before expensive operations.

## 4. Code Example

```rust
use fleet_coordinate::graph::FleetGraph;
use fleet_spread::fleet_coordinate_bridge::CoordinateBridge;
use fleet_spread::captain::CaptainDecision;

fn main() {
    // Build a triangle fleet (rigid in 2D)
    let mut cg = FleetGraph::new();
    cg.add_agent(1, [0.0, 0.0], vec![]);
    cg.add_agent(2, [1.0, 0.0], vec![]);
    cg.add_agent(3, [0.5, 0.87], vec![]);
    cg.add_edge(1, 2);
    cg.add_edge(2, 3);
    cg.add_edge(3, 1);

    // Full captain analysis
    let bridge = CoordinateBridge::new();
    let analysis = bridge.analyze(&cg);

    println!("Rigidity: {:?}", analysis.rigidity.is_rigid);
    println!("Beta-1: {}", analysis.rigidity.h1_dimension);
    match &analysis.decision {
        CaptainDecision::Decided { reason, .. } => println!("Decision: Decided — {}", reason),
        CaptainDecision::Constrained { violations, .. } => println!("Decision: Constrained — {}", violations.join(", ")),
        CaptainDecision::Stable { reason, .. } => println!("Decision: Stable — {}", reason),
    }
    // Fast check (no captain)
    let quick = CoordinateBridge::quick_check(&cg);
    println!("Rigid: {}, Emergence: {}", quick.is_rigid, quick.emergence_detected);
}
```

## 5. Architecture Diagram

```
                    ┌─────────────────────────────┐
                    │  fleet-coordinate FleetGraph │
                    │  (u64 IDs, positions, edges)│
                    └──────────────┬──────────────┘
                                   │
                    ┌─────────────┴─────────────┐
                    ▼                           ▼
        ┌───────────────────┐       ┌───────────────────────┐
        │  convert_from_    │       │  check_laman_rigidity  │
        │  coordinate()     │       │  + EmergenceDetector   │
        └────────┬──────────┘       └───────────┬───────────┘
                 │                              │
                 ▼                              ▼
    ┌─────────────────────────┐   ┌──────────────────────────┐
    │ fleet-spread FleetGraph  │   │   RigidityResult         │
    │ (String IDs, TrustVals) │   │   (is_rigid, beta_1,     │
    └────────────┬────────────┘   │    max_neighbors)         │
                 │               └──────────┬─────────────────┘
                 │                          │
                 │         ┌────────────────┘
                 ▼         ▼
    ┌─────────────────────────────────────────┐
    │          FleetGraphState                │
    │  (V, E, beta_1, zhc_residual, entropy) │
    └──────────────────┬────────────────────┘
                       │
          ┌────────────┴────────────┐
          ▼                          ▼
  ┌───────────────┐         ┌──────────────────┐
  │ all_with_     │         │  captain.        │
  │ signal(state)  │         │  deliberate()    │
  └───────┬───────┘         └────────┬─────────┘
          │                          │
          ▼                          ▼
  ┌───────────────────┐   ┌──────────────────┐
  │ Vec<Specialist>    │   │ CaptainDecision  │
  │ (signal sources)  │   │ (verdict, reports,│
  └───────────────────┘   │  consulted,       │
                          │  violations)       │
                          └───────────────────┘

  QUICK CHECK (no captain):
  coordinate graph → check_laman_rigidity() → EmergenceDetector::detect()
                                        ↓
                          QuickConsistencyResult
                          (is_rigid, emergence_detected, beta_1, max_neighbors)
```

## 6. Performance Notes

| Mode | What it does | When to choose |
|------|-------------|----------------|
| `analyze()` | Full conversion + rigidity + state build + gate + captain | Fleet events, topology changes, strategic decisions, when you need specialist reports |
| `quick_check()` | Rigidity + emergence math only | Heartbeats, hot paths, fast consistency checks before expensive ops |
| `convert_from_coordinate()` | Graph conversion only | Once per fleet change, reuse spread graph multiple times |

**Approximate costs:**
- `quick_check()`: ~0.1–0.5ms for typical fleets (100–1000 agents)
- `analyze()`: ~5–50ms depending on specialist count and graph size
- `convert_from_coordinate()`: ~0.5–2ms for typical fleets

**Recommendation:** Use `quick_check()` as your default health check. Switch to `analyze()` when:
- A fleet event (join/leave/reconfiguration) just occurred
- A specialist recommendation is needed
- You're about to make a structural decision (split, merge, reconfigure)

## 7. Testing

### Running Tests

```bash
cd /home/ubuntu/.openclaw/workspace/repos/fleet-spread
cargo test --lib fleet_coordinate_bridge
```

### Existing Tests

| Test | What it verifies |
|------|-----------------|
| `test_coordinate_bridge_analyze` | Full `analyze()` on a rigid triangle: `is_rigid=true`, `beta_1=1` |
| `test_quick_check` | `quick_check()` on same triangle: `is_rigid=true`, `emergence_detected=false` |
| `test_convert_from_coordinate` | Graph conversion: 2 vertices, 1 edge converts correctly |
| `test_captain_inquire_coordinate` | `CaptainCoordinateExt::inquire_coordinate()` returns non-empty consulted list |

### Adding New Test Cases

Add a `#[test]` block inside `#[cfg(test)]` in `fleet_coordinate_bridge.rs`:

```rust
#[test]
fn test_my_scenario() {
    // Build a specific graph topology
    let mut cg = FleetGraph::new();
    cg.add_agent(1, [0.0, 0.0], vec![]);
    cg.add_agent(2, [1.0, 0.0], vec![]);
    // ... more agents and edges ...

    // Test analyze
    let bridge = CoordinateBridge::new();
    let result = bridge.analyze(&cg);
    assert!(result.rigidity.is_rigid); // or whatever invariant you're testing

    // Test quick_check (stateless, no bridge instance needed)
    let quick = CoordinateBridge::quick_check(&cg);
    assert_eq!(quick.beta_1, expected_beta);
}
```

### Key Invariants to Test

- **Rigidity**: A Laman-eligible graph (E = 2V - 3 in a single component) should report `is_rigid = true`
- **Emergence**: A dense graph (E > 2V - 3) with many cycles should detect emergence
- **Graph conversion**: V and E counts must match between coordinate and spread graphs
- **Captain deliberation**: `consulted` should be non-empty when specialists have signal
- **β₁ correctness**: β₁ = E - V + 1 (for a connected graph) should match `rigidity.h1_dimension`
