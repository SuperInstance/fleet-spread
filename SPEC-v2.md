# fleet-spread v2 — Library Gate Architecture

## Overview

v2 replaces the MoE-style "run all 5 specialists and reconcile" with a **library gate selector** that picks THE ONE correct specialist based on fleet graph state.

## Core Insight

Current v1 architecture:
```
FleetGraph → [S1][S2][S3][S4][S5] → Synthesis → reconcile 5 reports
```
This is MoE-style: run everyone, let a router blend outputs. Expensive, conflicting, messy.

v2 architecture:
```
FleetGraph → FleetGraphState → LibraryGate → ONE Specialist → ONE Report
```
Bilateral constant-matching: the gate selects based on what the fleet NEEDS right now.

## What Are "Constants"?

A fleet agent's **constants** are fixed criteria that define when it should be activated:

| Constant | Default | Purpose |
|----------|---------|---------|
| `beta_threshold` | 0.05 | H¹ emergence threshold — above this, topological specialist |
| `zhc_tolerance` | 0.01 | ZHC loop sum tolerance — below this, geometric specialist |
| `min_neighbors` | 3 | Minimum trust neighbors for valid analysis |
| `trust_vector_precision` | 0.1 | Pythagorean48 precision threshold |
| `h1_emergency_lead_s` | 2.7 | Early warning lead time (seconds) |
| `rigidity_check_interval` | 1 | Seconds between rigidity checks |

## What Is "Matching"?

Matching is NOT "find the best specialist." It's **compatibility check**: do the agent's constants fit this task's requirements?

```rust
pub fn matches_task(&self, task: &TaskRequirements) -> bool {
    self.beta_threshold >= task.required_beta_threshold
    && self.zhc_tolerance >= task.required_zhc_tolerance
    && self.min_neighbors <= task.required_neighbors
    && task.urgency < 0.9  // don't match fire drills
}
```

If constants match → the agent bids/accepts. If not → the agent ignores.

## What Is a "Library Gate"?

The library gate is a **selector** with the signature:
```rust
pub fn select(&self, state: &FleetGraphState) -> Option<Specialist>
```

Given current fleet graph state, it returns ONE specialist to run, or `None` if the fleet is stable.

## The Gate Table

| Condition | Select | Why |
|-----------|--------|-----|
| β₁ = 0 AND graph rigid | **None** | Fleet stable, no specialist needed |
| β₁ rising | **Topological** | H¹ emergence — need cycle tracking |
| β₁ > threshold AND ZHC loop degraded | **Geometric** | ZHC closure check needed |
| Trust vector noisy | **Algebraic** | Pythagorean48 encoding analysis |
| V < 3 | **Systems** | Insufficient data for specialists |
| All clear but agent count changed | **Empirical** | Trust drift detection |

Priority order (first match wins):
1. V < 3 → systems
2. β₁ = 0 AND rigid → None (stable)
3. Trust vector noisy → algebraic
4. β₁ rising → topological
5. ZHC loop degraded → geometric
6. Agent count changed → empirical
7. Default → None (stable)

## Bilateral Constant-Matching Diagram

```
                    ┌─────────────────────────────────────┐
                    │         TASK APPEARS                 │
                    │  (FleetGraphState from telemetry)   │
                    └──────────────┬──────────────────────┘
                                   │
                                   ▼
              ┌──────────────────────────────────────────┐
              │          LIBRARY GATE                    │
              │  ┌──────────────────────────────────┐  │
              │  │  AgentConstants:                  │  │
              │  │  • beta_threshold = 0.05         │  │
              │  │  • zhc_tolerance = 0.01         │  │
              │  │  • min_neighbors = 3             │  │
              │  │  • trust_vector_precision = 0.1   │  │
              │  └──────────────────────────────────┘  │
              │                  │                      │
              │                  ▼                      │
              │  ┌──────────────────────────────────┐  │
              │  │  MATCHING LOGIC:                   │  │
              │  │  For each state condition,        │  │
              │  │  does agent's constant COMPAT?    │  │
              │  └──────────────────────────────────┘  │
              └──────────────────┬───────────────────────┘
                                 │
                    ┌───────────┴───────────┐
                    │                       │
                    ▼                       ▼
            ┌───────────────┐       ┌───────────────┐
            │  MATCH:       │       │  NO MATCH:    │
            │  Return that │       │  Return None  │
            │  Specialist  │       │  (stable)    │
            └───────────────┘       └───────────────┘
```

## What v1 Got Wrong

v1 synthesis layer tried to:
1. Run all 5 specialists simultaneously
2. Collect 5 reports
3. Find where they agree (robust findings)
4. Find where they disagree (tensions)
5. Reconcile into a synthesis

**Problems:**
- Expensive: 5x the computation for each decision
- Conflicting: specialists disagree on what "rigid" means
- Reconciliation is arbitrary: which finding wins?
- No clear selector: when do you run all 5 vs. just 1?

v2 fixes this with **one specialist, one report, no reconciliation needed**.

## FleetGraphState

```rust
pub struct FleetGraphState {
    pub V: usize,                    // vertex count
    pub E: usize,                    // edge count
    pub beta_1: f64,                 // H1 cohomology dimension
    pub zhc_loop_residual: f64,     // ZHC loop sum (0=perfect)
    pub trust_vector_entropy: f64,   // entropy of trust distributions
    pub agent_count: usize,         // current agent count
    pub last_change_s: f64,          // seconds since topology change
    pub is_connected: bool,
}
```

## Specialist Reports (v2)

v2 specialists take `FleetGraphState` instead of `FleetGraph`, returning one focused `SpecialistReport`:

| Specialist | When Selected | Report Focus |
|------------|--------------|--------------|
| **Topological** | β₁ rising | H¹ emergence tracking, cycle basis changes |
| **Geometric** | ZHC degraded | ZHC closure check, holonomy analysis |
| **Algebraic** | Trust noisy | Pythagorean48 encoding analysis |
| **Systems** | V < 3 | Insufficient data assessment |
| **Empirical** | Agent count changed | Trust drift detection |

## Comparison: v1 vs v2

| Aspect | v1 (MoE-style) | v2 (Library Gate) |
|--------|----------------|-------------------|
| Specialists per decision | All 5 | Exactly 1 |
| Reconciliation | Required | None needed |
| Compute cost | O(5n) | O(n) |
| Conflict resolution | Arbitrary routing | Bilateral matching |
| Selector logic | None | Gate table |
| When to run | Always | Only when needed |
| Stable fleet | Run 5 anyway | Run 0 (None) |

## Leveling Up = Refining Constants

In the dojo model, a greenhorn learns by matching the right boats. In fleet-spread v2, an agent levels up by **refining its constants** so the right specialists keep matching:

- Too sensitive (low thresholds) → matches everything, over-triggers
- Too tolerant (high thresholds) → matches nothing, misses signals
- Right constants → matches the exact situations where it can contribute

The fleet evolves not through better routing algorithms, but through better constant calibration.
