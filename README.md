# fleet-spread

Fleet graph analysis across 5 specialist dimensions with synthesis.

## The Core Idea

`fleet-spread` fans out a single fleet graph across 5 specialist analysis dimensions, producing constraint tiles and a unified synthesis. The synthesis layer identifies where specialists agree, where they disagree, and what's missing from all 5 — delivering analysis that's more than the sum of its parts.

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
