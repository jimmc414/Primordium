# Milestones: Voxel Ecosystem Simulator

**Version:** 0.1.0-draft
**Status:** Draft — pending review before project-structure phase
**Prerequisites:** requirements.md v0.1.0, architecture.md v0.1.0

---

## Dependency Graph

```
M1 ──→ M2 ──→ M3 ──→ M4 ──→ M6 ──→ M8
               │             │       ↗
               └──→ M5 ──────┘    M7
                                   │
                              M9 ──┘ (stretch)
```

**Critical path:** M1 → M2 → M3 → M4 → M6 → M8

**Parallel track after M2:** M5 (temperature) can begin after M2 and merges at M6.

**Independent:** M7 (stats/UI) depends only on the buffer format from M1 and can be developed in parallel from M3 onward, but is positioned late because it's not blocking any simulation work.

**Read the graph as:** An arrow from A to B means "B requires A to be complete." A milestone cannot begin until all its inbound dependencies pass acceptance criteria.

---

## M1: GPU Bootstrap and Static Rendering

**Goal:** Prove the full platform stack works end-to-end: Rust → WASM → WebGPU → pixels on screen. Establish the buffer layout, camera controls, and ray marcher. No simulation logic.

**Produces:** A browser window showing a 128³ bounding box with a handful of manually placed colored voxels, navigable with orbit/zoom/pan camera.

### Deliverables

1. Rust/WASM project scaffolding with `wgpu`, `wasm-bindgen`, `wasm-pack` build pipeline.
2. WebGPU device initialization with adapter capability detection. Error message if WebGPU unavailable.
3. `voxel_buf_a` allocated (64 MB). Initialized with ~100 hand-placed voxels of each type (WALL, NUTRIENT, ENERGY_SOURCE, PROTOCELL with dummy genome, WASTE, HEAT_SOURCE, COLD_SOURCE) at known coordinates.
4. `render_tex` — 128³ RGBA8 3D texture.
5. Compute shader: `update_render_texture`. Reads `voxel_buf_a`, writes `render_tex` with the color mapping from architecture §7.1.
6. Ray march fragment shader: full-screen triangle, ray-AABB intersection, front-to-back compositing against `render_tex`. Step size = 0.5 voxels.
7. Bounding box wireframe (12 line segments).
8. Orbit/zoom/pan camera. Inverse view-projection matrix passed as uniform. Mouse drag = orbit, scroll = zoom, middle-drag or shift-drag = pan.
9. Cross-section clip plane: keyboard shortcut cycles axis (X/Y/Z/off), mouse scroll with modifier adjusts clip position.
10. HTML page that loads the WASM module and displays the canvas.

### Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| 1 | Page loads in Chrome without errors, canvas displays colored voxels | Manual: open in browser |
| 2 | Each voxel type renders with the correct distinct color from architecture §7.1 | Manual: compare against color spec |
| 3 | Camera orbit rotates the view. Zoom changes distance. Pan translates. | Manual: mouse interaction |
| 4 | Cross-section clip plane exposes interior voxels along selected axis | Manual: toggle clip, verify interior visible |
| 5 | Empty voxels are transparent — only occupied voxels render | Manual: interior of sparse grid is see-through |
| 6 | `wasm-pack build --release` produces a .wasm file < 2 MB uncompressed | Automated: file size check |
| 7 | GPU buffer allocation succeeds and total memory ≤ 80 MB (no double buffer or temp buffer yet) | Automated: log allocated sizes |

---

## M2: Simulation Loop, Metabolism, and Death

**Goal:** Establish the double-buffered simulation tick loop, timing model, and the simplest biologically meaningful behavior: protocells consume energy, gain energy from adjacent nutrients/energy sources, and die when depleted. No movement, no replication, no conflicts.

**Produces:** A running simulation where seeded protocells glow (energy high), dim (energy draining), and wink out (death → waste → decay). Nutrients near protocells deplete. The world visibly changes each tick with no player input.

### Deliverables

1. `voxel_buf_b` allocated. Double-buffer swap logic with two bind groups per architecture §3.1.
2. `sim_params` uniform buffer with all metabolism-related parameters: `nutrient_spawn_rate`, `waste_decay_ticks`, `nutrient_recycle_rate`, `movement_energy_cost` (unused yet, but present), `base_ambient_temp` (constant 0.5, no temperature field yet).
3. Simulation tick timer with configurable tick rate (default 10/sec). Render loop decoupled per architecture §8.1. Pause/resume via keyboard.
4. Compute shader: `ca_update` (initial version). Handles:
   - PROTOCELL: subtract metabolic cost from energy. Gain energy from adjacent NUTRIENT (passive consumption per architecture §4.5). Gain energy from adjacent ENERGY_SOURCE. Die → WASTE if energy == 0.
   - NUTRIENT: decrement concentration when adjacent protocell consumes. Convert to EMPTY when depleted. Spawn new nutrients in random EMPTY voxels at global rate.
   - WASTE: decrement decay countdown. Convert to NUTRIENT or EMPTY per config.
   - All other types: copy unchanged.
5. PRNG implementation: PCG hash per architecture §9.1. Used for nutrient spawning positions.
6. Initial seeding: host writes a cluster of ~50 protocells with random genomes and high initial energy, surrounded by a field of nutrients, with 2–3 energy sources nearby.

### Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| 1 | Simulation runs at target tick rate (10/sec) and render at ≥ 60 FPS simultaneously | Automated: frame time + tick counter logging |
| 2 | Protocells near energy sources maintain energy; those far from sources drain and die | Manual: observe over ~100 ticks |
| 3 | Dead protocells become WASTE, WASTE decays to EMPTY or NUTRIENT | Manual: observe lifecycle |
| 4 | Nutrient voxels adjacent to protocells deplete over time | Manual: observe nutrient field shrinking near protocells |
| 5 | New nutrients spawn in empty space at the configured rate | Manual: observe replenishment in vacant areas |
| 6 | Pausing stops simulation ticks; rendering continues. Resume restarts ticks. | Manual: pause, rotate camera, resume |
| 7 | Same initial state + same tick count produces identical output (determinism) | Automated: run 100 ticks twice with same seed, compare voxel buffer checksums |
| 8 | Total GPU memory ≤ 145 MB (both voxel buffers + render tex, no temp yet) | Automated: log allocation |

---

## M3: Replication, Mutation, and the Intent System

**Goal:** Introduce the intent/resolve architecture and the first behavior that creates conflicts: replication. Protocells that accumulate enough energy replicate into adjacent empty space. Mutation produces genome variation. Species differentiation becomes visible through color.

**Produces:** Starting from a single seed cluster, the population expands outward. Over hundreds of ticks, the uniform initial color fractures into distinct species clusters. Population grows until nutrient equilibrium constrains it.

### Deliverables

1. `intent_buf` allocated (8 MB). Cleared at start of each tick.
2. Compute shader: `intent_declaration` (initial version). Protocells evaluate replication threshold. If met, declare REPLICATE intent targeting a random adjacent EMPTY voxel with energy-proportional bid. Otherwise declare IDLE.
3. Compute shader: refactor `ca_update` into `resolve_and_execute`. Each EMPTY voxel checks neighbor intents for incoming REPLICATE. Highest bid wins. Winner's thread and target thread independently compute the same comparison (redundant-but-correct pattern from architecture §5).
4. Replication mechanics: energy split, genome copy with per-byte mutation, species_id computation via hash.
5. Mutation: 16 PRNG advances per replication regardless of branch, per architecture §9.3. Temperature modifier stubbed to 1.0 (no temperature field yet).
6. Species coloring: `update_render_texture` now maps `species_id` to hue via golden-ratio spacing per architecture §7.1.
7. Fixed PRNG consumption (21 advances per protocell per tick) to maintain determinism.

### Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| 1 | Protocells replicate when energy exceeds threshold | Manual: observe population growth from seed cluster |
| 2 | Offspring appear in adjacent empty voxels, not at arbitrary positions | Manual: observe expansion front |
| 3 | Parent energy decreases after replication (split applied) | Automated: snapshot parent energy before/after tick containing replication |
| 4 | Offspring genomes differ from parent (mutation visible) | Automated: read back two adjacent protocells, compare genomes — they should differ with probability proportional to mutation rate |
| 5 | Different species produce visibly different colors | Manual: after 200+ ticks, multiple distinct colors present |
| 6 | Population stabilizes when nutrients are exhausted (no infinite growth) | Manual: observe population plateau over 500+ ticks |
| 7 | Conflict resolution is correct: when two protocells target the same EMPTY voxel, exactly one offspring appears there | Automated: seed two high-energy protocells adjacent to a single EMPTY voxel, tick once, verify exactly one new protocell at the target |
| 8 | Determinism preserved: same seed + 200 ticks = same final state | Automated: checksum comparison |

---

## M4: Movement, Chemotaxis, and Player Commands

**Goal:** Protocells can now move, creating dynamic spatial behavior. Chemotaxis biases movement toward resources, producing visible foraging patterns. Player tools allow environmental manipulation, closing the core interaction loop.

**Produces:** Protocells visibly migrate toward nutrient-rich and energy-rich areas. Player can place/remove walls, energy sources, nutrients, seed protocells, and apply toxins. The simulation responds to player interventions in real time.

### Deliverables

1. Extend `intent_declaration` with movement logic: MOVE intent with direction biased by chemotaxis toward adjacent NUTRIENT/ENERGY_SOURCE. Movement probability controlled by `movement_bias` genome parameter.
2. Intent priority: DIE > REPLICATE > MOVE > IDLE (replication takes precedence over movement when both conditions are met).
3. Extend `resolve_and_execute`: EMPTY voxels now resolve both REPLICATE and MOVE intents from neighbors. Source voxel becomes EMPTY on successful move. Movement energy cost deducted.
4. `command_buf` allocated. Command struct per architecture §4.2.
5. Compute shader: `apply_player_commands`. Processes up to 64 commands per tick.
6. Host-side command pipeline: JavaScript mouse click → ray cast → command construction → `queue.writeBuffer` to `command_buf`.
7. Player tools implemented:
   - Place/remove WALL
   - Place/remove ENERGY_SOURCE
   - Place NUTRIENT (brush radius)
   - Seed protocells (brush radius, random genomes)
   - Toxin (brush radius, kills protocells below toxin resistance threshold)
8. Tool selector UI: HTML toolbar with buttons for each tool. Active tool highlighted. Brush radius slider.

### Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| 1 | Protocells move toward nutrients when chemotaxis strength > 0 | Manual: place nutrients away from protocell cluster, observe migration |
| 2 | Movement costs energy (protocells that move excessively die faster) | Automated: compare lifespan of high-movement vs low-movement genomes in controlled setup |
| 3 | Replication takes priority over movement (protocell above replication threshold replicates, doesn't move) | Automated: seed high-energy protocell adjacent to one empty voxel and one nutrient. Verify replication occurs, not movement. |
| 4 | Player-placed walls block movement and replication | Manual: build wall enclosure, verify protocells don't cross it |
| 5 | Player-placed energy sources cause local population growth | Manual: place energy source in empty area with few protocells, observe colonization |
| 6 | Toxin kills low-resistance protocells and spares high-resistance ones | Automated: seed protocells with known toxin_resistance values, apply toxin, verify selective survival |
| 7 | Player commands apply within 2 simulation ticks of click | Manual: place wall, observe it appears within ~200ms |
| 8 | All tools functional: wall, energy source, nutrient, seed, toxin | Manual: exercise each tool, verify effect |

---

## M5: Temperature System

**Goal:** Add the spatial temperature field, diffusion, and temperature modulation of protocell behavior. This creates environmental niches and the selection pressure needed for sustained diversity.

**Can begin after M2.** Does not require M3 or M4 — temperature diffusion and storage are independent of the intent system. However, temperature *modulation* of mutation and metabolism requires the replication and metabolism systems from M3 and M2 respectively, so full integration testing requires M3+M4.

**Produces:** Visible temperature gradients radiating from heat/cold sources. Protocells near heat sources mutate faster and burn energy quicker. Cold-zone protocells are slow, stable, and efficient. Distinct populations emerge in different thermal zones.

### Deliverables

1. `temp_buf_a` and `temp_buf_b` allocated (8 MB each). Initialized to ambient (0.5). Double-buffer swap synced with voxel buffers.
2. Compute shader: `temperature_diffusion` per architecture §4.3. Diffusion with wall insulation and heat/cold source anchoring.
3. Player tools: place/remove HEAT_SOURCE and COLD_SOURCE. Added to tool selector.
4. Temperature modulation in `resolve_and_execute`: metabolic cost multiplied by `1.0 + temp_sensitivity × (local_temp - 0.5)`. Mutation rate multiplied by same factor. `temp_sensitivity` added to `sim_params`.
5. Temperature overlay: alternate `update_render_texture` mode that maps temperature to a blue→red color gradient. Toggled by UI button.
6. Simulation dispatch order updated to 5-pass pipeline per architecture §4.1: commands → diffusion → intent → execute → stats (stats stub — actual reduction in M7).

### Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| 1 | Temperature diffuses outward from heat sources and inward from cold sources | Manual: place heat source, toggle temperature overlay, observe gradient |
| 2 | Walls block temperature diffusion | Manual: place wall between heat source and observation area, verify sharp temperature boundary |
| 3 | Heat/cold sources maintain their target temperature after diffusion | Automated: read back temperature at source position, verify it equals target |
| 4 | Protocells in hot regions have higher metabolic cost (die faster without food) | Automated: seed identical protocells in hot and cold zones with equal nutrients, compare survival time |
| 5 | Protocells in hot regions mutate faster (higher species diversity near heat) | Automated: seed identical protocells in hot and cold zones, run 500 ticks, compare species_id variance |
| 6 | Temperature overlay accurately reflects the diffused temperature field | Manual: compare overlay colors to expected gradient pattern |
| 7 | Diffusion is stable — no oscillation or divergence at max diffusion rate (0.25) | Automated: run 1000 ticks of diffusion-only (no protocells), verify temperatures converge monotonically toward equilibrium |

---

## M6: Predation

**Goal:** Add the final simulation mechanic: protocells with predation capability can consume adjacent protocells, creating food webs. This produces the predator-prey oscillations required by acceptance criteria §9.5 of requirements.md.

**Requires M4** (intent system with full priority chain) **and M5** (temperature modulation of all behaviors including predation energy dynamics).

**Produces:** Observable predator-prey dynamics. A "predator species" (high predation_capability, high aggression) visibly expands into and consumes a "prey species" cluster, followed by predator die-off when prey is depleted, followed by prey recovery. Coupled oscillation.

### Deliverables

1. Extend `intent_declaration` with predation logic: if `predation_capability > 0`, scan neighbors for protocells with energy below `predation_aggression` threshold. If target found, declare PREDATE with direction and energy-weighted bid.
2. Intent priority updated: DIE > PREDATE > REPLICATE > MOVE > IDLE.
3. Extend `resolve_and_execute` for predation: simplified model per architecture §5. Predation always succeeds — prey's movement intent is cancelled if predation wins the bid contest at the prey's position. Prey converts to WASTE. Predator gains `predation_energy_fraction × prey_energy`. `predation_energy_fraction` added to `sim_params`.
4. Predation conflict: if two predators target the same prey, highest bid wins (same probabilistic resolution as movement/replication conflicts).
5. Predator visual distinction: protocells with `predation_capability > 128` render with a subtle marker (brighter saturation or size boost in render texture alpha channel).

### Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| 1 | Predator protocells consume adjacent prey protocells | Manual: seed predators adjacent to prey, observe prey converting to WASTE |
| 2 | Predator gains energy from successful predation | Automated: snapshot predator energy before/after predation tick, verify increase |
| 3 | Non-predator protocells (predation_capability == 0) never predate | Automated: seed non-predator adjacent to low-energy protocell, run 100 ticks, verify no predation |
| 4 | Predation cancelled if prey is simultaneously predated by a higher-bid predator | Automated: two predators targeting same prey, verify only one gains energy |
| 5 | Predator-prey oscillation: population of predator species and prey species show anticorrelated fluctuation over 1000+ ticks | Automated: seed separate predator and prey clusters near shared nutrient source. Log populations. Verify negative correlation coefficient between predator and prey population time series. |
| 6 | Predation interacts correctly with temperature (hot-zone predators burn more energy) | Manual: compare predator viability in hot vs cold zones |
| 7 | All prior milestone behaviors still work (no regressions in metabolism, replication, movement, temperature) | Automated: re-run M2–M5 acceptance tests |

---

## M7: Statistics, Inspector, and UI

**Goal:** Provide the observation tools needed to understand the simulation: real-time stats, population graphs, voxel inspector, and parameter controls. This is the "make it legible" milestone.

**Can begin development after M3** (species IDs exist). Full integration requires M6 (predation data in stats). Positioned here because it doesn't block simulation work, and the value of stats increases as simulation complexity increases.

### Deliverables

1. Compute shader: `stats_reduction` per architecture §4.6. Two-stage reduction: per-workgroup → final. Outputs: total protocell count, total energy, species counts (top 64 species by population).
2. `stats_buf` and `stats_staging` allocated. Async readback pipeline: `mapAsync` on staging buffer, copy when ready, forward to JavaScript.
3. Stats display: HTML overlay showing population count, species count, average energy, simulation tick rate (actual, not target), render FPS.
4. Population graph: line chart showing top 5 species populations over the last 500 ticks. Rendered with an HTML canvas (not GPU) using the stats readback data. Updates every 10 ticks.
5. Voxel inspector: GPU ray-cast compute shader per architecture §7.4. On click, dispatches single-workgroup shader, reads back hit position, reads back voxel state at that position. Displays tooltip with: type, energy, age, species_id, full genome (decoded parameter names and values).
6. Parameter controls: HTML sliders for all `sim_params` values. Changes take effect on the next tick via `queue.writeBuffer` to the uniform buffer.
7. Overlay mode selector: buttons for Normal, Temperature, Energy Density, Population Density. Each mode uses a different color mapping in `update_render_texture`.
8. Tick rate slider (1–60 ticks/sec). Single-step button.

### Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| 1 | Population count matches actual protocell count (within 1% due to async lag) | Automated: pause simulation, count protocells in readback buffer, compare to displayed count |
| 2 | Species count reflects distinct species_id values present | Automated: same as above for species |
| 3 | Population graph shows recognizable predator-prey oscillation when predation is active | Manual: observe graph during predation scenario |
| 4 | Voxel inspector shows correct data for clicked voxel (verified against known test state) | Automated: place protocell with known genome at known position, click it, verify tooltip data matches |
| 5 | Parameter changes take effect immediately (e.g., increasing nutrient spawn rate produces visible nutrient increase within 10 ticks) | Manual: adjust slider, observe |
| 6 | Overlay modes display distinct visualizations (temperature overlay differs from energy overlay) | Manual: toggle modes, compare |
| 7 | Stats readback does not impact simulation or render performance (no frame drops during readback) | Automated: measure frame times with stats enabled vs disabled, verify < 1ms difference |

---

## M8: Polish, Performance, and Fallback Tiers

**Goal:** Harden the application for real-world use. Detect GPU capabilities, select appropriate grid resolution, optimize hot paths, and ensure graceful degradation. This is the "make it shippable" milestone.

**Requires all of M1–M7.**

### Deliverables

1. GPU capability detection at startup: query adapter limits (maxBufferSize, maxStorageBufferBindingSize, device type). Select grid tier:
   - Discrete GPU, ≥ 256 MB VRAM budget: 128³
   - Discrete GPU, < 256 MB: 96³
   - Integrated GPU: 64³
   - Insufficient: display error with hardware requirements
2. Grid resolution parameterized throughout the codebase: all buffer sizes, dispatch dimensions, shader constants, and render texture dimensions derive from a single `GRID_SIZE` constant set at initialization.
3. Performance profiling pass: measure per-dispatch timing using WebGPU timestamp queries (where supported). Identify bottlenecks. Target: `intent_declaration` + `resolve_and_execute` combined ≤ 50 ms at 128³ on RTX 3060 class.
4. Loading screen during pipeline compilation and buffer allocation.
5. Error recovery: if a buffer allocation fails, retry at the next lower grid tier.
6. Keyboard shortcut reference overlay (toggle with `?` key).
7. Initial seeding presets: "Petri Dish" (central cluster + surrounding nutrients), "Gradient" (hot left / cold right + scattered protocells), "Arena" (walled enclosures with different conditions). Selectable from UI.

### Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| 1 | Application auto-selects appropriate grid tier on integrated GPU (verified on Intel UHD or equivalent) | Manual: test on integrated GPU hardware or GPU emulation |
| 2 | 128³ grid sustains ≥ 10 ticks/sec simulation and ≥ 60 FPS render on RTX 3060 class at < 30% occupancy | Automated: benchmark harness — seed protocells to 30% occupancy, measure tick rate and frame rate over 100 ticks |
| 3 | 64³ grid sustains ≥ 10 ticks/sec and ≥ 30 FPS on integrated GPU | Automated: same benchmark at 64³ |
| 4 | WASM binary < 5 MB gzipped | Automated: build and measure |
| 5 | All three seeding presets produce visibly distinct ecosystem dynamics within 200 ticks | Manual: try each preset, observe |
| 6 | No browser console errors or WebGPU validation errors during 10-minute continuous run | Automated: run with WebGPU error logging enabled, assert zero errors |
| 7 | All requirements from requirements.md §9 (Acceptance Criteria Summary) are met | Manual + automated: run full acceptance suite |

---

## M9: Sparse Grid (Stretch Goal)

**Goal:** Implement the brick-based sparse representation from architecture §12, enabling a 256³ logical grid at ≤ 10% occupancy.

**Requires M8** (parameterized grid size, performance baseline established).

**Gating condition from requirements FS-3:** The dense 128³ implementation MUST pass all acceptance criteria before this work begins.

### Deliverables

1. Brick data structure: 8³ voxel bricks (16 KB each). Brick pool buffer pre-allocated for target occupancy.
2. Spatial hash map on GPU: maps brick coordinates → brick pool offset. Implemented as an open-addressing hash table in a storage buffer.
3. Brick allocation/deallocation: allocation on first non-EMPTY voxel in a brick region. Deallocation when all voxels in a brick are EMPTY (checked during stats reduction, amortized over ticks).
4. Modified CA shaders: voxel indexing goes through brick lookup. Cross-brick neighbor access requires secondary hash lookup.
5. Modified render texture generation: only processes allocated bricks.
6. Modified temperature diffusion: only processes allocated bricks and their face-adjacent neighbors.
7. 256³ grid tier added to capability detection: discrete GPU with ≥ 512 MB VRAM budget at ≤ 10% expected occupancy.

### Acceptance Criteria

| # | Criterion | Verification |
|---|-----------|-------------|
| 1 | 256³ grid at 5% occupancy sustains ≥ 10 ticks/sec on discrete GPU | Automated: benchmark with controlled 5% population |
| 2 | Simulation behavior is identical to dense grid for equivalent configurations | Automated: run same scenario at 64³ dense and 64³-within-256³ sparse, compare final state checksums |
| 3 | Memory usage scales with occupancy, not grid volume (256³ at 5% uses < 200 MB) | Automated: measure allocated buffer sizes |
| 4 | Bricks are deallocated when emptied — memory does not grow monotonically | Automated: seed and kill a population, verify brick count returns near initial level |

---

## Milestone Timing Estimates

These are rough estimates for an AI coding agent (Claude Code) with complete spec documents, not for a human developer. They assume the agent produces correct code on first or second attempt given precise specifications.

| Milestone | Estimated Duration | Cumulative |
|-----------|--------------------|------------|
| M1 | 1–2 sessions | 1–2 sessions |
| M2 | 1 session | 2–3 sessions |
| M3 | 1–2 sessions | 3–5 sessions |
| M4 | 1–2 sessions | 4–7 sessions |
| M5 | 1 session | 5–8 sessions |
| M6 | 1 session | 6–9 sessions |
| M7 | 1–2 sessions | 7–11 sessions |
| M8 | 1–2 sessions | 8–13 sessions |
| M9 | 2–3 sessions | 10–16 sessions |

A "session" is a single extended Claude Code interaction (up to context window capacity). The wide ranges reflect uncertainty in debugging GPU shader issues, which are the primary risk to velocity.

---

## Regression Policy

Each milestone inherits all prior acceptance criteria. When working on milestone N, the agent MUST verify that milestones 1 through N-1 still pass. If a regression is detected, fixing it takes priority over new milestone work.

The determinism tests (M2 AC#7, M3 AC#8) are particularly important as regression indicators — if determinism breaks, something fundamental is wrong in the shader pipeline.
