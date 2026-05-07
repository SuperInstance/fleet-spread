# fleet-spread

[![CI](https://github.com/SuperInstance/fleet-spread/actions/workflows/ci.yml/badge.svg)](https://github.com/SuperInstance/fleet-spread/actions/workflows/ci.yml)

Fleet graph analysis with library gate architecture (v2).

**v2 insight:** Don't run all 5 specialists — select THE ONE that matches.

## Quick Start

```rust
use fleet_spread::{FleetGraph, LibraryGate};

let graph = FleetGraph::from_edges(&[(0,1), (1,2), (0,2)]);
let gate = LibraryGate::new();
let specialist = gate.select(&graph);

// Run one specialist, not all five
let report = specialist.analyze(&graph);
println!("Synthesis gain: {:.2}", report.gain());
```

**What just happened:** The library gate examined the fleet graph (3 vertices, 3 edges, β₁=0, rigid) and selected the **Systems** specialist because V=3. No voting, no reconciliation, no O(5n) cost.

Run tests: `cargo test` — **147 tests** covering specialists, synthesis, quality metrics, and library gate selection logic.

---

## Library Gate Architecture (v2)

Instead of MoE-style "run all 5 and reconcile", v2 uses a **library gate selector** that picks exactly one specialist based on fleet graph state.

```
┌─────────────────────────────────────────────────────────┐
│                    LIBRARY GATE                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐   │
│  │Systems  │  │Algebraic│  │Topological│ │Geometric│   │
│  │  S1     │  │   S2    │  │    S3     │ │   S4    │   │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘   │
│       │            │            │            │         │
│       └────────────┴────────────┴────────────┘         │
│                         │                               │
│                  ┌──────┴──────┐                        │
│                  │ CAPTAIN     │ ← Reads all reports   │
│                  │ Deliberation│   decides final output │
│                  └─────────────┘                        │
└─────────────────────────────────────────────────────────┘
```

**v2 flow:**
```
Task appears → Library Gate checks conditions → ONE specialist runs
     ↓
Captain reads single report → produces synthesis
     ↓
No reconciliation needed (only one specialist ran)
```

### Gate Selection Table

| Condition | Select | Why |
|-----------|--------|-----|
| V < 3 | **Systems** | Insufficient data for topology |
| β₁ = 0 AND rigid (E=2V-3) | **None** | Fleet already stable |
| Trust encoding noisy | **Algebraic** | Encoding stability analysis |
| β₁ rising (emergence) | **Topological** | H¹ cohomology detection |
| ZHC loop degraded | **Geometric** | Holonomy closure failure |
| Agent count changed | **Empirical** | Trust drift detection |

### v1 vs v2

| Aspect | v1 (MoE) | v2 (Library Gate) |
|--------|----------|-------------------|
| Specialists per decision | All 5 | 1 or 0 |
| Reconciliation step | Required | None |
| Cost | O(5n) | O(n) |
| Stable fleet | Runs all 5 | Runs 0 (skip) |

---

## The 5 Specialist Dimensions

| Specialist | Focus | Key Metric | Trigger |
|------------|-------|------------|---------|
| **S1: Topological** | Betti numbers, cycle basis | β₁ = E - V + C | β₁ rising |
| **S2: Geometric** | ZHC closure, holonomy | Stress detection | ZHC loop degraded |
| **S3: Algebraic** | Pythagorean48 encoding | Encoding stability | Trust noisy |
| **S4: Systems** | Laman rigidity | E = 2V - 3 | V < 3 |
| **S5: Empirical** | Trust anomalies, drift | σ detection | Agent count changed |

### Key Concepts

**Deadband Protocol:** Specialists only activate when their metric crosses a threshold. This prevents jitter and reduces unnecessary computation. The deadband is adaptive — it narrows as the fleet matures.

**P0 / P1 / P2 Priorities:**
- **P0** (must have): Fleet is rigid (E=2V-3). Without this, coordination is impossible.
- **P1** (should have): β₁ = 0 means no emergent cycles. Fleet is stable.
- **P2** (nice to have): Trust encoding stable. Agents agree on shared state.

**Why Greedy Fails:** A greedy approach that picks the "best" specialist by local utility creates coordination failures. When the fleet has a rigid core but emergent cycles at the boundary, the topological specialist detects emergence while the geometric specialist sees closure failure. Greedy picks one and misses the other. Library gate runs the specialist that matches the *global* state, not the local optimum.

**Why All Specialists with Signal Matters:** Even when only one specialist runs (v2), the *signal* from other specialists informs the captain's deliberation. The gate's skip decisions (e.g., "skip all — fleet is stable") are informed by what the other specialists would have said. This is why stable fleets skip all five: the absence of signal *is* the signal.

---

## Output

Each analysis produces:
- **Specialist Report** — constraint tile from the selected specialist
- **Captain Synthesis** — unified analysis with:
  - Robust findings (confirmed by ≥3 specialists, or the single active specialist)
  - Tensions (questions with conflicting signals)
  - Blind spots (questions no specialist addressed)
  - Synthesis gain (did the combination add value?)

## Synthesis Gain

`fleet-spread` measures whether the unified analysis adds value over individual specialists:

- `synthesis_gain > 0.3`: Synthesis substantially better than parts
- `synthesis_gain > 0`: Some added value
- `synthesis_gain < 0`: Worse than best single specialist (failure mode)

---

## Graph Types

| Type | Condition | Expected Analysis |
|------|-----------|------------------|
| **Rigid** | E = 2V - 3, connected | Strong consensus, no specialists run |
| **Over-connected** | E > 2V - 3 | Geometric strain |
| **Under-constrained** | E < 2V - 3 | Incomplete analysis |
| **Disconnected** | C > 1 | Per-component, cross-component gap |

---

## Architecture

```
fleet-spread/
├── src/
│   ├── lib.rs              # Library entry
│   ├── main.rs             # CLI
│   ├── graph.rs           # Fleet graph data structures
│   ├── specialists/       # 5 specialist modules
│   │   ├── topological.rs # S1: Topology (β₁, H¹ cohomology)
│   │   ├── geometric.rs   # S2: Geometry (ZHC closure)
│   │   ├── algebraic.rs   # S3: Encoding (Pythagorean48)
│   │   ├── systems.rs    # S4: Rigidity (Laman, E=2V-3)
│   │   └── empirical.rs  # S5: Anomalies (trust drift)
│   ├── synthesis.rs       # Captain deliberation layer
│   ├── library_gate.rs    # Specialist selection logic
│   ├── plato_tile.rs      # PLATO output
│   ├── git_commit.rs      # Git integration
│   └── quality.rs         # Quality metrics
└── tests/                 # Test suites (147 tests)
```

---

## Use Cases

1. **Fleet health monitoring** — Detect over-constrained or anomalous subgraphs
2. **Trust propagation analysis** — Measure encoding stability across hops
3. **Rigidity certification** — Confirm Laman rigidity for formation control
4. **Comparative analysis** — Compare synthesis gain across different fleet configurations

---

## Related

- **[fleet-coordinate](https://github.com/SuperInstance/fleet-coordinate)** — Uses Laman rigidity (E=2V-3) to certify when the fleet constraint graph is rigid enough for ZHC. The topological specialist's β₁ calculation feeds into fleet-coordinate's emergence detection.

- **[holonomy-consensus](https://github.com/SuperInstance/holonomy-consensus)** — Provides the ZHC closure check used by the geometric specialist. When ZHC loop degrades, the geometric specialist activates.

- **[constraint-theory-ecosystem](https://github.com/SuperInstance/constraint-theory-ecosystem)** — The mathematical foundation: Laman's theorem, H¹ cohomology, and the constraint theory that underlies all fleet mathematics.

---

## License

MIT