# fleet-spread Specification

## Overview

**fleet-spread** fans out a single fleet graph across 5 specialist analysis dimensions, producing constraint tiles and a unified synthesis. The synthesis layer identifies where specialists agree, where they disagree, and what's missing from all 5 — delivering analysis that's more than the sum of its parts.

## The 5 Specialist Dimensions

### S1: Topological Specialist
- **Input**: `V` (vertices), `E` (edges), adjacency list
- **Output**: Betti numbers (β₀, β₁, β₂), cycle basis, connected components, H¹ dimension
- **Tool**: `fleet-homology` / `fleet-coordinate emergence.rs`
- **Key formula**: β₁ = E - V + C (where C = connected components)
- **Failure modes**: Graph too small (V < 3) → no cycles detectable; graph disconnected → per-component analysis needed

### S2: Geometric Specialist
- **Input**: Edge trust values, cycle list
- **Output**: Holonomy around each cycle, deviation from identity, stressed edges (|holonomy| > threshold)
- **Tool**: `fleet-coordinate zhc.rs`
- **Key insight**: Trust is a parallel transport — going around a cycle should return to identity. Deviation indicates geometric strain.
- **Failure modes**: No cycles → no holonomy to measure; trust values near 0 → undefined direction

### S3: Algebraic Specialist
- **Input**: Trust vectors on edges
- **Output**: Encoding drift after N hops, bit precision analysis, encoding stability score (0-1)
- **Tool**: `fleet-coordinate pythagorean48.rs`
- **Key insight**: Pythagorean48 encoding maps trust to a geometric representation. Drift measures how much the encoding degrades over multiple hops.
- **Failure modes**: No edges → no encoding; precision < 32 bits → drift analysis unreliable

### S4: Systems Specialist
- **Input**: V, E, neighbor degrees
- **Output**: Is Laman-rigid? (E = 2V - 3), redundant edges, over-constrained nodes, max neighbor count
- **Tool**: `fleet-coordinate graph.rs` + `fleet-topology`
- **Key formula**: Laman rigidity requires exactly 2V - 3 edges for generic rigidity in 2D
- **Failure modes**: V < 3 → cannot be Laman-rigid; over-connected (E > 2V - 3) → flexible or redundantly rigid

### S5: Empirical Specialist
- **Input**: Actual trust values from PLATO room query
- **Output**: Anomalous trust values (deviation from historical mean > 2σ), drift detection, outlier edges
- **Tool**: PLATO room HTTP query
- **Failure modes**: No PLATO room accessible → skip this specialist; insufficient history → no drift detection possible

## Synthesis Layer

### SpecialistReport Structure
```rust
struct SpecialistReport {
    specialist_id: &'static str,
    findings: Vec<Finding>,      // concrete claims with confidence
    confidence: f32,             // 0.0 - 1.0
    unanswered: Vec<String>,    // questions this specialist couldn't address
    raw_data: Value,             // specialist-specific output
}

struct Finding {
    claim: String,
    confidence: f32,
    evidence: Vec<String>,
}
```

### Synthesis Operations

**1. Agreement Detection**
- Find findings where ≥ 3 specialists agree on same conclusion
- Mark as "robust finding" — high confidence, multiple independent angles

**2. Disagreement Detection**
- Find findings where specialists contradict each other
- Mark as "interesting tension" — requires human review or more data

**3. Blind Spot Detection**
- Questions no specialist addressed
- Mark as "gap" — potential missing analysis dimension

**4. Synthesis Gain Calculation**
```
synthesis_gain = (max_specialist_info - unified_info) / max_specialist_info
```
- If synthesis_gain < 0: unified analysis is WORSE than best specialist (failure mode)
- If synthesis_gain > 0.3: synthesis is substantially better than parts
- synthesis_gain < 0.1: tool provided minimal value

## Failure Modes

| Mode | Detection | Response |
|------|-----------|----------|
| Specialist disagrees with others | confidence spread > 0.5 | Mark tension, don't force consensus |
| No synthesis possible | specialists disagree on all findings | Return "inconclusive" with per-specialist reports |
| Pathological graph (disconnected) | C > 1 | Run per-component, warn about cross-component analysis |
| Pathological graph (too small) | V < 3 | Skip S1, S4; warn about limited analysis |
| Pathological graph (too dense) | E > 2V - 3 | Mark S4 as "over-constrained" |
| PLATO room unreachable | HTTP error | Skip S5, note in report |
| All specialists low confidence | mean confidence < 0.3 | Mark overall confidence low |

## Graph Pathologies

### Disconnected Graph (C > 1)
- Run each specialist per component
- Warn that cross-component trust cannot be analyzed
- Report per-component rigidity separately

### Too Small (V < 3)
- Skip S1 topological (no cycles possible)
- Skip S4 systems (cannot be Laman-rigid)
- Warn results are limited

### Too Dense (E > 2V - 3)
- S4 marks as "over-constrained" not "rigid"
- S2 may find high holonomy strain
- Note: dense graphs may appear rigid but have internal flexibility

## PLATO Tile Output

Each specialist produces a constraint tile:
```
{
  "type": "fleet-spread.<specialist>",
  "data": { ... specialist output ... },
  "confidence": 0.85,
  "graph_id": "...",
  "timestamp": "..."
}
```

Synthesis produces a master tile:
```
{
  "type": "fleet-spread.synthesis",
  "data": {
    "robust_findings": [...],
    "tensions": [...],
    "blind_spots": [...],
    "synthesis_gain": 0.42
  },
  "confidence": 0.7,
  "graph_id": "..."
}
```

## What Makes fleet-spread Exceptional

1. **Multi-dimensional confirmation**: A finding confirmed by topological AND geometric AND systems is more robust than any single analysis
2. **Conflict detection**: Disagreements between specialists highlight interesting structure
3. **Quantitative synthesis gain**: We measure whether the synthesis actually added value
4. **Blind spot identification**: We explicitly track what nobody looked at
5. **Pathology handling**: Graceful degradation for graphs that break assumptions

## Quality Metrics

- **Novelty**: Does synthesis reveal something none of the specialists saw alone?
- **Correctness**: Are specialist analyses accurate? (Validated against known graph properties)
- **Usefulness**: Would an agent or human prefer the synthesis over raw specialist output?
- **Completeness**: Are there important questions no specialist can answer?

## Example Outputs

### Small Rigid Fleet (V=5, E=7)
- S1: β₁ = 7-5+1 = 3 cycles → moderate complexity
- S2: Low holonomy strain → trust geometry is consistent
- S3: High encoding stability (0.92) → reliable trust propagation
- S4: E=7 = 2*5-3 → Laman-rigid ✓
- S5: No anomalies detected
- **Synthesis**: All 5 agree on rigidity + consistency → robust finding, synthesis_gain = 0.45

### Over-Connected Fleet (V=5, E=20)
- S1: β₁ = 20-5+1 = 16 → highly complex
- S2: High holonomy strain on 8 edges → geometric stress
- S3: Low encoding stability (0.31) → degraded trust propagation
- S4: E=20 > 7 → over-constrained, NOT Laman-rigid
- S5: 4 anomalous trust values
- **Synthesis**: S2/S3/S4 agree on over-constraint; S1/S5 confirm complexity → robust finding of over-constraint, synthesis_gain = 0.61

### Disconnected Fleet (V=8, 2 components)
- Component A: V=5, E=7 (rigid)
- Component B: V=3, E=2 (under-constrained)
- **Synthesis**: Report per-component; note cross-component trust undefined → gap identified, synthesis_gain = 0.38
