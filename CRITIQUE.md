# CRITIQUE.md — fleet-spread Critical Review

## What fleet-spread Does
Fans a fleet graph across 5 specialist analysis dimensions (topological, geometric, algebraic, systems, empirical), produces individual specialist reports, then synthesizes them into a unified view with a `synthesis_gain` score.

## What's Exceptional
- **5 specialist dimensions**: Each dimension (topological, geometric, algebraic, systems, empirical) brings a genuinely different lens to the same graph
- **Synthesis layer**: The `synthesis_gain` metric measures how much additional insight the synthesis adds beyond the sum of individual specialists
- **Specialist disagreement tracking**: When specialists disagree, this is explicitly tracked and reported

## What's Lacking

### 1. Specialists Produce Reports, Not Discoveries
The specialists analyze a given graph and produce structured reports. But they don't *discover* anything — they compute known metrics (Betti numbers, Laman conditions, Pythagorean48 encoding). This is measurement, not insight generation.

**Better approach**: Have specialists generate hypotheses that explain anomalies. If S5 (empirical) detects trust drift, S3 (algebraic) should generate a hypothesis about *why*.

### 2. Synthesis Is Aggregation, Not Reconciliation
The synthesis layer aggregates specialist reports. But when specialists *disagree*, the synthesis doesn't adjudicate — it just notes the disagreement. A true synthesis would explain *why* they disagree and which view is likely correct.

**What would fix this**: Add a meta-specialist layer that evaluates the coherence of specialist claims and flags impossible combinations (e.g., ZHC claims zero holonomy but empirical shows drift — one of them is wrong).

### 3. The FleetGraph Input Is Synthetic
The examples use hardcoded small graphs (V=5, E=7). The tool doesn't ingest real fleet data. There's no connection to PLATO rooms, no way to pull a real fleet's trust graph from the keeper API.

**What would fix this**: Add a `KeeperClient` that fetches the actual fleet topology from the keeper service at `:8900`.

### 4. No Counterfactual Analysis
The tool tells you what the current fleet looks like across 5 dimensions. It doesn't tell you what the fleet *should* look like, or what interventions would improve the synthesis_gain.

**What would fix this**: Add a `ProjectedFleet` that models changes (adding/removing edges, changing trust weights) and predicts how synthesis_gain would change.

### 5. Quality Assessment Is Per-Specialist, Not Cross-Specialist
The quality module assesses each specialist report independently. But the most valuable quality signal is *cross-specialist consistency* — do S1 and S4 agree on rigidity? Do S2 and S5 agree on convergence?

## The Real Question: Does This Produce Insights a Human Wouldn't?

After reviewing the specialist implementations, the honest answer is: **mostly no for routine cases, possibly yes for edge cases**.

**What humans would find easily**:
- Computing Betti numbers from adjacency matrix
- Checking Laman conditions (already in fleet-coordinate)
- Encoding trust as Pythagorean48 vectors

**What might surprise a human**:
- Cross-specialist anomalies detected by comparing S5 (empirical trust drift) against S3 (algebraic encoding stability)
- The synthesis_gain score objectively measuring "depth of insight"

## What Would Make It Exceptional
1. **Real fleet ingestion**: Connect to keeper API to analyze actual fleet topologies
2. **Counterfactual modeling**: What-if analysis for topology changes
3. **Anomaly-driven hypotheses**: When S5 detects drift, generate specific hypotheses to test
4. **Cross-specialist consistency scoring**: Not just tracking disagreement but scoring the coherence of the overall picture
