# fleet-spread

Fleet graph analysis with library gate architecture (v2).

**v2 insight:** Don't run all 5 specialists — select THE ONE that matches.

## Library Gate Architecture (v2)

Instead of MoE-style "run all 5 and reconcile", v2 uses a **library gate selector** that picks exactly one specialist based on fleet graph state.

### Bilateral Constant-Matching

```
Task appears → Library Gate checks → ONE specialist runs
     ↓
Agent has fixed constants
Fleet has current state
Match? → activate
No match? → skip
```

### Gate Table

| Condition | Select | Why |
|-----------|--------|-----|
| V < 3 | **Systems** | Insufficient data |
| β₁ = 0 AND rigid | **None** | Fleet stable |
| Trust noisy | **Algebraic** | Encoding analysis |
| β₁ rising | **Topological** | H¹ emergence |
| ZHC loop degraded | **Geometric** | ZHC closure |
| Agent count changed | **Empirical** | Trust drift |

Priority: V<3 → stable check → noisy → rising β₁ → degraded ZHC → count change

### v1 vs v2

| Aspect | v1 (MoE) | v2 (Library Gate) |
|--------|----------|-------------------|
| Specialists/decision | All 5 | 1 or 0 |
| Reconciliation | Required | None |
| Cost | O(5n) | O(n) |
| Stable fleet | Runs all 5 | Runs 0 |

## The 5 Specialist Dimensions

| Specialist | Focus | Key Metric |
|------------|-------|------------|
| **S1: Topological** | Betti numbers, cycle basis | β₁ = E - V + C |
| **S2: Geometric** | ZHC closure, holonomy | Stress detection |
| **S3: Algebraic** | Pythagorean48 encoding | Encoding stability |
| **S4: Systems** | Laman rigidity | E = 2V - 3 |
| **S5: Empirical** | Trust anomalies, drift | σ detection |

## Quick Start

```bash
# Build
cargo build --release

# Run on built-in test graphs
cargo run -- test

# Analyze a custom graph
cargo run -- analyze --input my-graph.json

# Generate sample graph
cargo run -- sample --graph-type small-rigid --output sample.json
```

## Output

Each analysis produces:
- **5 Specialist Reports** — individual constraint tiles
- **Synthesis Report** — unified analysis with:
  - Robust findings (confirmed by ≥3 specialists)
  - Tensions (disagreements between specialists)
  - Blind spots (questions no specialist addressed)
  - Synthesis gain (did the combination add value?)

## Synthesis Gain

`fleet-spread` measures whether the unified analysis adds value over individual specialists:

- `synthesis_gain > 0.3`: Synthesis substantially better than parts
- `synthesis_gain > 0`: Some added value
- `synthesis_gain < 0`: Worse than best single specialist (failure mode)

## Graph Types

| Type | Condition | Expected Analysis |
|------|-----------|------------------|
| **Rigid** | E = 2V - 3, connected | Strong consensus |
| **Over-connected** | E > 2V - 3 | Geometric strain |
| **Under-constrained** | E < 2V - 3 | Incomplete analysis |
| **Disconnected** | C > 1 | Per-component, cross-component gap |

## Architecture

```
fleet-spread/
├── src/
│   ├── lib.rs              # Library entry
│   ├── main.rs             # CLI
│   ├── graph.rs           # Fleet graph data structures
│   ├── specialists/       # 5 specialist modules
│   │   ├── topological.rs # S1: Topology
│   │   ├── geometric.rs   # S2: Geometry
│   │   ├── algebraic.rs   # S3: Encoding
│   │   ├── systems.rs    # S4: Rigidity
│   │   └── empirical.rs  # S5: Anomalies
│   ├── synthesis.rs       # Synthesis layer
│   ├── plato_tile.rs      # PLATO output
│   ├── git_commit.rs      # Git integration
│   └── quality.rs         # Quality metrics
└── tests/                 # Test suites
```

## Use Cases

1. **Fleet health monitoring** — Detect over-constrained or anomalous subgraphs
2. **Trust propagation analysis** — Measure encoding stability across hops
3. **Rigidity certification** — Confirm Laman rigidity for formation control
4. **Comparative analysis** — Compare synthesis gain across different fleet configurations

## License

MIT
