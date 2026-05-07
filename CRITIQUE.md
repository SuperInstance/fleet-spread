# CRITIQUE.md — fleet-spread Critical Review

## What fleet-spread Does (v3 Architecture)

Fans a fleet graph across 5 specialist analysis dimensions (topological, geometric, algebraic, systems, empirical), produces individual specialist reports, then synthesizes them into a unified view.

**v3 architecture flow:**
```
FleetGraphState
    │
    ├── library_gate.all_with_signal() ──→ "which specialists have signal RIGHT NOW"
    │                                        Returns Vec<Specialist>, not Option<Specialist>
    │
    ├── captain.inquire() ──────────────────→ Runs ALL specialists with signal
    │                                        (wide inquiry phase, not "select one")
    │
    ├── captain.adjudicate() ────────────────→ Weighs findings probabilistically
    │                                        Higher quality wins, safety-critical overrides
    │
    ├── captain.apply_constraints() ────────→ P0 hard constraints (SafetyMargin, SparesRequired, etc.)
    │                                        NON-NEGOTIABLE pass/fail filters
    │
    └── captain.decide() ────────────────────→ Narrow decision from wide inquiry
                                                Decided / Constrained / Stable
```

## What's Exceptional (v3)

- **5 specialist dimensions**: Each dimension brings a genuinely different lens
- **Synthesis layer**: The `synthesis_gain` metric measures additional insight beyond sum of parts
- **Specialist disagreement tracking**: When specialists disagree, explicitly tracked
- **Captain as expert inquiry engine**: Captain consults ALL specialists with signal, not "pick one"
- **Hard constraints as P0 filters**: Safety and spares are never negotiable — pass/fail, not weights
- **Adjudication not averaging**: When specialists contradict, quality wins; safety overrides non-critical

## What's Lacking (Still)

### 1. Specialists Produce Reports, Not Discoveries
The specialists analyze a given graph and produce structured reports. But they don't *discover* anything — they compute known metrics (Betti numbers, Laman conditions, Pythagorean48 encoding). This is measurement, not insight generation.

**Better approach**: Have specialists generate hypotheses that explain anomalies. If S5 (empirical) detects trust drift, S3 (algebraic) should generate a hypothesis about *why*.

### 2. Synthesis Is Aggregation, Not Reconciliation
The synthesis layer aggregates specialist reports. But when specialists *disagree*, the synthesis doesn't adjudicate — it just notes the disagreement. A true synthesis would explain *why* they disagree and which view is likely correct.

**What would fix this**: Add a meta-specialist layer that evaluates the coherence of specialist claims and flags impossible combinations.

### 3. The FleetGraph Input Is Synthetic
The examples use hardcoded small graphs (V=5, E=7). The tool doesn't ingest real fleet data. There's no connection to PLATO rooms, no way to pull a real fleet's trust graph from the keeper API.

**What would fix this**: Add a `KeeperClient` that fetches the actual fleet topology from the keeper service at `:8900`.

### 4. No Counterfactual Analysis
The tool tells you what the current fleet looks like across 5 dimensions. It doesn't tell you what the fleet *should* look like, or what interventions would improve the synthesis_gain.

**What would fix this**: Add a `ProjectedFleet` that models changes (adding/removing edges, changing trust weights) and predicts how synthesis_gain would change.

### 5. Quality Assessment Is Per-Specialist, Not Cross-Specialist
The quality module assesses each specialist report independently. But the most valuable quality signal is *cross-specialist consistency* — do S1 and S4 agree on rigidity? Do S2 and S5 agree on convergence?

## Architecture Evolution

### v1: MoE fanner-out (DEPRECATED)
Original approach: fan out to ALL specialists, synthesize everything, produce unified decision.
**Problem**: Diffuse, not decisive. Too many voices, no adjudication.

### v2: Library gate "select one specialist" (DEPRECATED)
Fix: library gate picks THE ONE correct specialist based on state.
**Problem**: Too blunt. The captain should consult ALL specialists with signal, not pick one.

### v3: Captain inquiry + hard constraints (CURRENT)
Fix: library gate tells you which specialists have signal (plural), captain runs wide inquiry on all of them, then applies P0 hard constraints before deciding.
**Correct model**: Captain (expert inquiry engine) consults ALL specialists with signal → weighs probabilistically → applies hard constraints (safety/spares) → decides narrowly.

## The Dojo Model Applied

The captain is the **expert who runs wide inquiry**:
- Studies every element that has signal
- Listens to all relevant specialists (data feeds, not oracles)
- Builds probabilistic mental model
- Checks for contradictions (adjudication)
- Applies hard P0 constraints (never negotiable)
- Acts decisively within the safe set

The library gate is the **signal detector**, not the decision maker:
- `select()` → returns ONE specialist for action (narrow)
- `all_with_signal()` → returns ALL specialists with signal for captain's inquiry (wide)

## What Would Make It Exceptional
1. **Real fleet ingestion**: Connect to keeper API to analyze actual fleet topologies
2. **Counterfactual modeling**: What-if analysis for topology changes
3. **Anomaly-driven hypotheses**: When S5 detects drift, generate specific hypotheses to test
4. **Cross-specialist consistency scoring**: Not just tracking disagreement but scoring coherence
