# Architecture: Voxel Ecosystem Simulator

**Version:** 0.1.0-draft
**Status:** Draft — pending review before milestones phase
**Prerequisite:** requirements.md v0.1.0

---

## 1. System Overview

The system has four subsystems with strict boundaries:

**Simulation Engine** — GPU compute shaders that advance the CA state each tick. Owns the voxel buffers and temperature field. Stateless between ticks (all state lives in GPU buffers).

**Renderer** — GPU render pipeline that ray-marches a 3D texture derived from voxel state. Reads simulation state but never writes it. Runs at display refresh rate, decoupled from simulation tick rate.

**Host Orchestrator** — Rust/WASM module running on the CPU. Owns the WebGPU device, manages buffer allocation, dispatches compute and render passes, handles timing, and bridges JavaScript UI events to GPU commands. Does not execute simulation logic.

**UI Layer** — HTML/JavaScript overlay. Displays stats, parameter controls, and tool selectors. Communicates with the Host Orchestrator via a thin WASM-bindgen API. Handles mouse/keyboard input and ray casting for voxel selection.

Data flows in one direction per frame: UI → Host → Simulation → Renderer → Screen. The only reverse flow is async stats readback (Simulation → Host → UI), which is non-blocking and may lag by 1–2 frames.

---

## 2. Voxel Data Model

### 2.1 Memory Layout

Each voxel is exactly 32 bytes, laid out as 8 × `u32` words for WGSL alignment. All field packing uses little-endian bit ordering within each `u32`.

| Word | Bits | Field | Description |
|------|------|-------|-------------|
| 0 | [0:7] | `voxel_type` | Enum: 0=EMPTY, 1=WALL, 2=NUTRIENT, 3=ENERGY_SOURCE, 4=PROTOCELL, 5=WASTE, 6=HEAT_SOURCE, 7=COLD_SOURCE |
| 0 | [8:15] | `flags` | Bit flags. Bit 0: player-placed. Bits 1–7: reserved. |
| 0 | [16:31] | `energy` | u16. Energy level (0–65535). Interpretation varies by type. |
| 1 | [0:15] | `age` | u16. Ticks since creation. Wraps at 65535. |
| 1 | [16:31] | `species_id` | u16. Hash of genome for protocells. 0 for non-protocells. |
| 2 | [0:31] | `genome_0` | Genome bytes 0–3 packed as u32. |
| 3 | [0:31] | `genome_1` | Genome bytes 4–7 packed as u32. |
| 4 | [0:31] | `genome_2` | Genome bytes 8–11 packed as u32. |
| 5 | [0:31] | `genome_3` | Genome bytes 12–15 packed as u32. |
| 6 | [0:31] | `extra_0` | Type-specific state. Protocells: last action result. Nutrients: concentration. |
| 7 | [0:31] | `extra_1` | Reserved. Must be zero-initialized. |

**Rationale for u16 energy:** 65535 is sufficient dynamic range for the ecosystem. Energy operations (consume, split, gain) use integer arithmetic — no floating-point precision issues. If finer granularity is needed later, energy can be reinterpreted as a fixed-point value without changing the layout.

### 2.2 Genome Encoding

The 16-byte genome is interpreted as 16 independent `u8` parameters, extracted by byte-shifting the 4 genome `u32` words. Each parameter ranges 0–255 and is normalized to [0.0, 1.0] by the CA rules as needed (divide by 255.0).

| Byte | Parameter | Interpretation |
|------|-----------|---------------|
| 0 | `metabolic_efficiency` | Higher = better nutrient-to-energy conversion |
| 1 | `metabolic_rate` | Higher = more energy consumed per tick (base cost) |
| 2 | `replication_threshold` | Energy level required to attempt replication (scaled to energy range) |
| 3 | `mutation_rate` | Per-byte probability of mutation during replication |
| 4 | `movement_bias` | Probability of attempting movement each tick |
| 5 | `chemotaxis_strength` | Movement bias toward nutrients/energy sources |
| 6 | `toxin_resistance` | Threshold for surviving toxin events |
| 7 | `predation_capability` | 0 = non-predator. >0 = can attempt predation. Higher = stronger. |
| 8 | `predation_aggression` | Energy threshold below which a neighbor becomes prey |
| 9 | `photosynthetic_rate` | Energy gain rate from adjacent ENERGY_SOURCE |
| 10 | `energy_split_ratio` | Parent's share of energy on replication (0=0%, 255=100%) |
| 11–15 | Reserved | Must be zero in initial genomes. Mutation may write non-zero values. Future: dormancy, signaling, etc. |

**Species ID computation:** `species_id = hash16(genome_0 XOR genome_1 XOR genome_2 XOR genome_3)` where `hash16` is a 16-bit mixing function (e.g., `x = ((x >> 8) ^ x) * 0x6979; x = ((x >> 8) ^ x) * 0x0235; return x >> 16`). This runs in the replication shader when offspring are created, not every tick.

### 2.3 Non-Protocell State Reuse

For non-protocell voxel types, the genome words and extra words carry type-specific data:

| Voxel Type | genome_0 | genome_1–3 | extra_0 | extra_1 |
|------------|----------|------------|---------|---------|
| NUTRIENT | concentration (u32) | unused (0) | decay_timer | unused |
| ENERGY_SOURCE | output_rate (u32) | unused (0) | heat_emission | unused |
| WASTE | decay_countdown (u32) | original species_id | nutrient_conversion_timer | unused |
| WALL | unused (0) | unused (0) | unused | unused |
| HEAT_SOURCE | target_temp (u32, f32 reinterp) | unused (0) | unused | unused |
| COLD_SOURCE | target_temp (u32, f32 reinterp) | unused (0) | unused | unused |

---

## 3. Buffer Inventory and Memory Budget

All sizes computed for 128³ grid (2,097,152 voxels).

| Buffer | Size | Usage | Binding |
|--------|------|-------|---------|
| `voxel_buf_a` | 64 MB | Voxel state read buffer (ping) | storage, read |
| `voxel_buf_b` | 64 MB | Voxel state write buffer (pong) | storage, read_write |
| `temp_buf_a` | 8 MB | Temperature field read (f32 per voxel) | storage, read |
| `temp_buf_b` | 8 MB | Temperature field write | storage, read_write |
| `intent_buf` | 8 MB | Intent declarations (u32 per voxel) | storage, read_write |
| `render_tex` | 8 MB | 3D RGBA8 texture for ray marching | texture, write then sample |
| `sim_params` | 256 B | Uniform buffer: all configurable simulation parameters | uniform |
| `stats_buf` | 128 B | Reduction output: population, species counts, energy totals | storage, map_read |
| `stats_staging` | 128 B | Staging buffer for async CPU readback | map_read |
| `command_buf` | 4 KB | Player action ring buffer (max 64 commands per tick) | storage, read |

**Total: ~152 MB.** Under the 160 MB budget with 8 MB headroom for WebGPU internal allocations, pipeline state, and bind groups.

### 3.1 Double-Buffering Protocol

The simulation alternates reads and writes between A and B buffers each tick:

- Tick N (even): read `voxel_buf_a` + `temp_buf_a`, write `voxel_buf_b` + `temp_buf_b`
- Tick N+1 (odd): read `voxel_buf_b` + `temp_buf_b`, write `voxel_buf_a` + `temp_buf_a`

This is implemented by maintaining two bind groups (one per direction) and selecting based on `tick_count % 2`. No buffer copies. No GPU-side pointer swaps. The host tracks which buffer is "current" via a boolean.

The `intent_buf` is not double-buffered — it is cleared at the start of each tick and fully written before being read.

---

## 4. Simulation Pipeline

Each simulation tick consists of five compute shader dispatches executed in sequence within a single command encoder. Workgroup size for all dispatches: `(4, 4, 4)` = 64 threads. Grid dispatch size: `(128/4, 128/4, 128/4)` = `(32, 32, 32)`.

### 4.1 Dispatch Order

```
┌─────────────────────────────────────────────────┐
│ TICK N                                          │
│                                                 │
│  1. apply_player_commands                       │
│     Reads: command_buf, voxel_read              │
│     Writes: voxel_read (in-place modification)  │
│     Note: Modifies the READ buffer before       │
│     simulation reads it. Safe because player    │
│     commands target specific voxels, not the    │
│     full grid.                                  │
│                                                 │
│  2. temperature_diffusion                       │
│     Reads: temp_read, voxel_read (wall check)   │
│     Writes: temp_write                          │
│                                                 │
│  3. intent_declaration                          │
│     Reads: voxel_read, temp_read                │
│     Writes: intent_buf                          │
│     Each protocell declares its intended action │
│     and bid value.                              │
│                                                 │
│  4. resolve_and_execute                         │
│     Reads: voxel_read, intent_buf, temp_write   │
│     Writes: voxel_write                         │
│     Each output voxel resolves conflicts and    │
│     determines its new state.                   │
│                                                 │
│  5. stats_reduction                             │
│     Reads: voxel_write                          │
│     Writes: stats_buf                           │
│     Parallel reduction: population, species     │
│     histogram, total energy.                    │
│                                                 │
│  [swap read/write buffer binding]               │
└─────────────────────────────────────────────────┘
```

Pipeline barriers between dispatches are implicit — each `dispatchWorkgroups` call in the same command encoder is sequenced by WebGPU's execution model. No manual barriers needed.

### 4.2 apply_player_commands

The host writes player actions into `command_buf` before encoding the tick. Each command is a 64-byte struct:

| Field | Size | Description |
|-------|------|-------------|
| `command_type` | u32 | Enum: PLACE_VOXEL, REMOVE_VOXEL, SEED_PROTOCELLS, APPLY_TOXIN, PLACE_HEAT, PLACE_COLD |
| `position` | 3 × u32 | Target voxel coordinates |
| `radius` | u32 | Effect radius (0 for single voxel) |
| `param_0` | u32 | Command-specific parameter |
| `param_1` | u32 | Command-specific parameter |
| `_padding` | 40 B | Pad to 64 bytes |

The compute shader iterates over the command buffer (small — max 64 commands) and applies each command. Commands with radius > 0 iterate over a sphere of voxels. This pass has low occupancy but runs rarely and is not performance-critical.

### 4.3 temperature_diffusion

Standard discrete diffusion: for each voxel, new temperature = current temperature + `diffusion_rate` × (average of non-wall neighbors' temperatures − current temperature).

Heat sources and cold sources override their own temperature to their target value after diffusion (Dirichlet boundary condition). Wall voxels do not participate in diffusion (their temperature is irrelevant; they block heat transfer between adjacent non-wall voxels).

The diffusion rate is a global simulation parameter in `sim_params`. One diffusion step per tick. Multiple diffusion steps per tick are unnecessary — the per-tick rate can be increased instead.

### 4.4 intent_declaration

Each voxel evaluates independently. Non-protocell voxels write a NO_ACTION intent (0).

For protocells, the priority order is:

1. **Die check:** If energy == 0, intent = DIE (no target, no bid). Converts to WASTE unconditionally in the execute pass.
2. **Predation:** If `predation_capability` > 0, scan neighbors for protocells with energy below `predation_aggression` threshold. If a target is found, intent = PREDATE, target = direction of prey, bid = PRNG-weighted by attacker energy.
3. **Replication:** If energy > `replication_threshold` (scaled by temperature modifier), intent = REPLICATE, target = random empty neighbor direction. If no empty neighbor, fall through.
4. **Movement:** With probability = `movement_bias` (modified by temperature), intent = MOVE, target = preferred direction (chemotaxis-biased toward nutrients/energy sources if present, else random empty neighbor). If no empty neighbor, fall through.
5. **Idle:** intent = IDLE. No action. Metabolism still applies.

Only one intent per protocell per tick. This simplifies conflict resolution.

**Intent word encoding (u32):**

| Bits | Field |
|------|-------|
| [0:2] | `target_direction`: 0–5 = ±X, ±Y, ±Z. 6 = self (die/idle). 7 = unused. |
| [3:5] | `action_type`: 0=NO_ACTION, 1=DIE, 2=PREDATE, 3=REPLICATE, 4=MOVE, 5=IDLE |
| [6:31] | `bid`: 26-bit value = `prng() % (energy + 1)`. Higher energy → higher expected bid. Stochastic. |

### 4.5 resolve_and_execute

This is the core pass. Each output voxel determines its own new state by reading its input state and the intents of its neighbors.

**For each voxel position P:**

**Case: Input is EMPTY.**
Check all 6 neighbors' intents. Collect any neighbor whose `target_direction` points toward P and whose `action_type` is REPLICATE or MOVE. If multiple contenders exist, the one with the highest `bid` wins (deterministic given PRNG seed). If no contenders, output remains EMPTY.

If a winner exists:
- MOVE: copy the winner's full voxel state to P. Apply metabolism (subtract energy cost). Increment age.
- REPLICATE: write a new protocell at P with the winner's genome (mutated — see below), energy = winner's energy × (1 - split_ratio), age = 0. Compute new species_id.

**Case: Input is PROTOCELL.**
Check P's own intent:
- DIE: output WASTE with P's species_id in extra data and a decay countdown.
- PREDATE (with target direction D): read the intent resolution at the target. If P won the predation contest at neighbor D's position (P's bid > defender's bid), output P with energy += prey_fraction × prey_energy. Increment age. Apply metabolism.
- REPLICATE or MOVE with win: P's intent was outgoing. If P won at the target, apply the consequences. For MOVE: output EMPTY at P (P moved away). For REPLICATE: output P with energy × `split_ratio`, incremented age, metabolism applied.
- REPLICATE or MOVE with loss, or IDLE: output P with metabolism applied (energy −= metabolic_rate × temperature_modifier). Increment age.

**Case: Input is WASTE.**
Decrement decay countdown. If zero, convert to NUTRIENT (if nutrient recycling enabled) or EMPTY.

**Case: Input is NUTRIENT.**
If any adjacent protocell declared PREDATE/MOVE targeting this voxel, ignore (nutrients don't move, protocells can only consume nutrients, not move into them). Instead, consumption is handled by protocells that declared no action: protocells adjacent to nutrients gain energy during the metabolism phase proportional to `metabolic_efficiency`.

Wait — this creates an asymmetry. Let me clarify: nutrient consumption is passive (adjacency-based), not intent-based. A protocell gains energy from each adjacent NUTRIENT simply by existing next to it. The nutrient's concentration decreases accordingly. When concentration reaches zero, the nutrient becomes EMPTY.

This is cleaner because it avoids protocells needing to "target" nutrients with intents. Intents are only for movement, replication, and predation.

**Case: All other types (WALL, ENERGY_SOURCE, HEAT_SOURCE, COLD_SOURCE).**
Copy unchanged. These are player-placed and only modified by player commands.

**Mutation during replication:**
Performed inline in the execute pass. For each of the 16 genome bytes of the offspring: generate a PRNG value. If `(prng_value % 256) < parent.mutation_rate`, replace the byte with `prng_next() % 256`. This uses the temperature-modified mutation rate: `effective_rate = mutation_rate × temperature_multiplier(local_temp)`.

### 4.6 stats_reduction

A parallel reduction shader that operates in two stages:

**Stage 1 (per-workgroup):** Each workgroup of 64 threads processes a contiguous chunk of voxels. Using workgroup shared memory, reduce to per-workgroup totals: total protocell count, total energy, per-species counts (up to a fixed species budget — top 64 species tracked via a small hash table in shared memory).

**Stage 2 (final):** A single workgroup reduces the per-workgroup outputs to final totals. Writes to `stats_buf`.

The host maps `stats_buf` asynchronously (`mapAsync`) and reads the results 1–2 frames later. Stats are displayed in the UI with this latency — imperceptible to the player.

---

## 5. Conflict Resolution: Detailed Semantics

The intent/resolve system eliminates GPU write conflicts entirely. No atomics are used anywhere in the simulation pipeline. Here is why:

Each output voxel is written by exactly one thread (the thread assigned to that output position). That thread reads its input voxel, its neighbors' input voxels, and its neighbors' intents. From this information it determines the output state. Multiple threads never write the same output voxel because the output grid has a 1:1 mapping to threads.

**Potential logic conflict:** Two protocells (A and B) both target the same empty voxel. The thread writing the empty voxel reads both A's intent and B's intent, compares bids, and writes the winner. The threads writing A's and B's output positions each need to know whether their protocell won. They determine this by re-reading the competitor's bid at the target position: thread-for-A reads B's intent, computes the same comparison, and determines A's outcome. This is redundant work (the comparison is computed twice) but avoids inter-thread communication.

**Correctness guarantee:** Because bid comparison is deterministic (same inputs → same result), thread-for-A and thread-for-empty-voxel will always agree on the winner. No race condition.

**Predation conflicts:** If protocell A wants to predate protocell B, and protocell B wants to move away in the same tick: A's predation targets B's original position. If B's move succeeds (B wins the bid at the target empty voxel), B is no longer at its original position. A's predation finds no prey. Resolution: the execute pass for A checks B's intent — if B declared MOVE and B's bid at its target is the winning bid, A's predation fails. A falls back to idle. This requires A's thread to read B's intent AND the intent of B's target's other potential contenders to verify B won. This fan-out is bounded (checking at most 6 additional intents) and is acceptable.

**Simplification if the fan-out is too expensive:** Predation always succeeds regardless of prey movement. The prey is consumed at its current position. If the prey also won a move, the move is cancelled (predation takes priority). This is biologically plausible (you can't outrun a predator in the same time step) and eliminates the fan-out read. Recommended for v1.

---

## 6. Temperature System

### 6.1 Buffer Format

`temp_buf_a` and `temp_buf_b` store one `f32` per voxel, indexed by linearized 3D position: `index = x + y * 128 + z * 128 * 128`.

Temperature is normalized to [0.0, 1.0] where 0.0 = minimum (cold extreme) and 1.0 = maximum (hot extreme). Default ambient temperature: 0.5.

### 6.2 Diffusion Algorithm

For each non-wall voxel at position P with current temperature T:

1. Sum temperatures of face-adjacent non-wall neighbors. Count the number of such neighbors (N, between 0 and 6).
2. If N > 0: `T_new = T + diffusion_rate × (T_avg - T)` where `T_avg = sum / N`.
3. If N == 0 (surrounded by walls): `T_new = T` (no change).
4. Clamp `T_new` to [0.0, 1.0].

After diffusion, heat/cold sources overwrite their voxel's temperature to their target value.

### 6.3 Temperature Effects on Protocells

The execute pass reads the temperature at each protocell's position (from the newly computed `temp_write` buffer) and applies two modifiers:

**Metabolic cost multiplier:** `1.0 + temp_sensitivity × (local_temp - 0.5)`. At ambient (0.5), no modifier. Hot regions increase metabolic cost. Cold regions decrease it. `temp_sensitivity` is a global simulation parameter.

**Mutation rate multiplier:** Same formula. Hot = more mutation. Cold = less mutation. This creates the niche pressure described in requirements: hot zones produce high-mutation, high-metabolism populations; cold zones produce stable, efficient populations.

---

## 7. Rendering Pipeline

### 7.1 Render Texture Generation

A compute shader pass (`update_render_texture`) runs after the simulation tick (or independently on each frame if simulation is paused). It reads the current voxel buffer and temperature buffer and writes an RGBA8 3D texture:

| Channel | Encoding |
|---------|----------|
| R | Species hue (protocells) or type-specific color (others) |
| G | Species saturation component or type color |
| B | Species brightness component or type color |
| A | Occupancy: 0 = empty/transparent, 255 = fully opaque |

The color mapping:
- PROTOCELL: HSV where H = `species_id × golden_ratio mod 1.0`, S = 0.7, V = `energy / max_energy`. Converted to RGB.
- NUTRIENT: green, alpha proportional to concentration.
- ENERGY_SOURCE: bright yellow, full alpha.
- WALL: gray, full alpha.
- WASTE: dark brown, alpha decays with age.
- HEAT_SOURCE: orange-red, full alpha.
- COLD_SOURCE: ice blue, full alpha.

Overlay modes (heatmap, temperature, etc.) replace this color mapping with alternative mappings controlled by a uniform.

### 7.2 Ray Marching Renderer

The render pass draws a single full-screen triangle. The fragment shader performs ray marching through the 3D texture volume:

1. Compute ray origin and direction from the camera's inverse view-projection matrix and fragment coordinates.
2. Intersect the ray with the simulation volume's axis-aligned bounding box.
3. Step along the ray from entry to exit at a step size of 0.5 voxels (256 steps max for 128³ diagonal).
4. At each step, sample the 3D texture with nearest-neighbor filtering. If alpha > 0, accumulate color using front-to-back alpha compositing.
5. Early-exit when accumulated alpha ≥ 0.95.

**Cross-section view:** A clip plane uniform (`clip_axis`: 0/1/2, `clip_position`: 0.0–1.0). During ray marching, skip samples that are on the clipped side of the plane.

**Performance:** 128 steps × 1080p resolution = ~265M texture samples per frame. At 128³ texture size, this fits entirely in texture cache on any discrete GPU. Expected render time: < 4 ms on RTX 3060 class.

### 7.3 Bounding Box Wireframe

Rendered as 12 line segments using a simple vertex/fragment shader pipeline. Submitted as a separate render pass (or sub-pass). Negligible cost.

### 7.4 Voxel Picking (for inspector)

On mouse click, the host performs a CPU-side ray march through the grid using the same ray computation as the renderer but reading from a CPU-side copy of voxel types only (a compact 1-byte-per-voxel buffer, updated periodically). Alternatively, a GPU compute shader writes the hit voxel position to a small buffer on click. The GPU approach is more accurate but adds latency; the CPU approach is immediate but requires maintaining a CPU-side occupancy cache.

Recommended: GPU approach — dispatch a single-workgroup compute shader that ray marches and writes the hit position. Map the result buffer asynchronously. The 1–2 frame latency is acceptable for an inspector tooltip.

---

## 8. Simulation / Render Decoupling

### 8.1 Timing Model

The host maintains two accumulators:

- `sim_accumulator`: incremented by `dt` (wall clock delta) each frame. When `sim_accumulator >= 1.0 / target_tick_rate`, run one simulation tick and subtract the tick duration. Cap at 3 ticks per frame to prevent spiral-of-death if the GPU falls behind.
- Render: runs once per `requestAnimationFrame`, always.

This means at 60 FPS display with 20 ticks/sec target: most frames run 0 simulation ticks, every 3rd frame runs 1 tick. The display always shows the latest completed state.

### 8.2 Buffer Access Pattern

The renderer always reads the "most recently written" voxel buffer. Since the host tracks which buffer was last written (A or B), the render pass's bind group is set accordingly each frame. There is no data race because WebGPU command submission is serialized — the render pass is submitted after any simulation dispatches in the same frame.

When the simulation is paused, no simulation dispatches occur. The renderer continues reading the same buffer. The `update_render_texture` pass is still dispatched if overlay mode changes, but can be skipped if nothing changed (optimization).

---

## 9. PRNG Strategy

### 9.1 Algorithm

PCG-family hash (PCG-RXS-M-XS-32): fast, statistically good, and implementable in a single WGSL function with no state beyond a `u32` seed.

**Per-voxel seed computation:** `seed = pcg_hash(voxel_index ^ (tick_count * 0x9E3779B9) ^ (grid_size * 0x85EBCA6B) ^ dispatch_salt)`. The golden ratio constant ensures different ticks produce uncorrelated seeds even for the same voxel. The grid_size term prevents seed collisions across different grid resolutions (important for test grids at 8³ vs production at 128³). The `dispatch_salt` differs per shader pass (e.g., `0x1` for intent_declaration, `0x2` for resolve_execute) so that two dispatches reading the same voxel in the same tick produce independent PRNG streams. Each voxel's shader invocation initializes its local PRNG state from this seed and advances it for each random number needed during that tick.

### 9.2 Determinism

Given the same voxel state and the same `tick_count`, the simulation produces identical output. This satisfies requirement S-3. The `tick_count` is a monotonically increasing u32 passed as a uniform. It does not reset on pause/resume.

### 9.3 Random Number Consumption

Each protocell consumes a fixed number of PRNG advances per tick to ensure determinism regardless of branch taken:

1. Movement decision (1 advance)
2. Movement direction selection (1 advance)
3. Predation target selection (1 advance)
4. Replication target selection (1 advance)
5. Bid value generation (1 advance)
6. Mutation — 16 advances (one per genome byte), regardless of whether replication occurs (advances are consumed but results discarded if no replication)

Total: 21 PRNG advances per protocell per tick. Non-protocell voxels consume 0 advances.

---

## 10. Player Interaction Pipeline

### 10.1 Input Flow

```
Browser mouse/keyboard event
  → JavaScript event handler
  → Classify: camera control vs. world interaction
  → If camera: update camera uniforms directly (no GPU dispatch)
  → If world interaction:
      → Ray cast to determine target voxel (see 7.4)
      → Construct command struct
      → Write to command_buf via queue.writeBuffer()
      → Command applied at start of next simulation tick (4.2)
```

### 10.2 Tool State Machine

The UI maintains a selected tool and tool parameters (brush radius, voxel type to place, etc.). Tool selection is pure UI state — no GPU impact until the player clicks.

### 10.3 Latency

Player action → visible result latency = time until next simulation tick + time until next render frame. At 20 ticks/sec and 60 FPS: worst case ~66 ms, typical ~33 ms. Acceptable for environmental manipulation tools (these are not twitch-precision actions).

---

## 11. CPU/GPU Responsibility Split

### 11.1 GPU-Only (compute shaders)

- All simulation logic (CA update, temperature diffusion, conflict resolution)
- PRNG generation
- Stats reduction
- Render texture generation
- Ray marching

### 11.2 CPU-Only (Rust/WASM)

- WebGPU device and pipeline creation
- Buffer allocation and bind group management
- Simulation timing and tick scheduling
- Command encoder construction and submission
- Player command serialization
- Stats readback and forwarding to JavaScript
- Camera matrix computation (inverse view-projection for ray marching)

### 11.3 JavaScript-Only

- DOM-based UI (stats display, parameter sliders, tool palette)
- Mouse/keyboard event capture
- wasm-bindgen glue for calling into Rust host

### 11.4 Prohibited Patterns

- **No CPU-side simulation logic.** The CPU must never read voxel state for simulation purposes. All simulation runs on GPU.
- **No synchronous GPU readback.** All `mapAsync` calls must be awaited without blocking the main thread.
- **No GPU-side DOM interaction.** Shaders do not influence UI directly — stats flow through the host.
- **No JavaScript-side WebGPU calls.** All WebGPU API usage goes through the Rust/WASM host via wgpu-rs.

---

## 12. Sparse Grid Extension (256³ Stretch Goal)

### 12.1 Approach

Replace the dense 3D buffer with a spatial hash map. Each entry in the hash map is a "brick" of 8³ = 512 voxels (16 KB per brick at 32 bytes/voxel). The hash map maps brick coordinates (x/8, y/8, z/8) to brick buffer offsets.

### 12.2 Allocation Strategy

Maintain a brick pool: a large pre-allocated GPU buffer sized for the expected maximum active bricks. At 10% occupancy of 256³, active voxels ≈ 1.67M, requiring ≈ 3,200 bricks × 16 KB = ~50 MB per voxel buffer (100 MB double-buffered). Temperature bricks: 8³ × 4 bytes = 2 KB × 3,200 = ~6.4 MB per buffer.

A brick is allocated when any voxel within it transitions from EMPTY to non-EMPTY. A brick is freed when all its voxels are EMPTY (checked during stats reduction, amortized).

### 12.3 Compute Shader Modifications

The CA update shader changes from indexing a flat 3D array to: (1) compute brick coordinates from voxel position, (2) look up brick offset in hash map, (3) index within the brick. Neighbor lookups that cross brick boundaries require a second hash map lookup. This adds ~10–20% overhead per voxel operation but eliminates processing of empty regions entirely.

### 12.4 Sequencing

This is a v2 optimization. The dense 128³ implementation must be fully working before this is attempted. The brick abstraction should be introduced behind an indexing trait/interface so that the CA logic remains identical.

---

## 13. Key Architectural Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| WebGPU buffer size limits vary by device | Buffers may fail to allocate on low-VRAM GPUs | Query device limits at startup. Fall back to 96³ or 64³ grid if 128³ buffers fail allocation. |
| Predation fan-out reads hurt performance | Extra neighbor reads in execute pass | Simplified predation is permanent: predation always succeeds, prey movement cancelled. This preserves the architectural invariant that each thread's output depends only on its own cell and immediate neighbors' intents — no transitive resolution. Only revisit if playtesting proves instant predation produces degenerate dynamics. |
| Passive nutrient consumption produces monoculture | Without food competition, protocells compete only for space (expansion-limited growth). This produces agar-plate dynamics rather than resource-limited boom-bust cycles and carrying capacity equilibria. | Acceptable for M2–M4. If playtesting shows monoculture dominance or flat population curves, nutrient competition is the first lever to pull. The nutrient voxel `extra_0` field already carries a decay timer; `extra_1` is reserved and can store a `claimed_by` species_id or cell index to support competitive consumption without a layout change. Adding intent-based nutrient targeting would roughly double the resolve shader's case analysis — only pursue if passive consumption demonstrably fails to produce interesting dynamics. |
| Stats reduction accuracy | Hash collisions in species histogram | Accept approximate species counts. Exact counts are not required for UI display. |
| Temperature diffusion stability | High diffusion rates cause oscillation | Clamp diffusion_rate to [0.0, 0.25] (stable regime for discrete diffusion). |
| WGSL compilation time | Complex shaders may take seconds to compile on first load | Pre-warm pipeline creation during loading screen. Cache compiled pipelines via browser's shader cache. |
| resolve_execute.wgsl is the highest-risk shader | Most complex shader in the project. Handles every combination of cell type × intent outcome × neighbor intent outcome. Incorrect case analysis produces silent simulation bugs. | M3 implementation MUST begin with a written case enumeration (as shader comments) covering every combination of input type and intent outcome before any branching logic is written. See technical-constraints TC-1. |
| Double-buffered memory footprint | 128 MB for voxels alone is aggressive | The 160 MB budget is tight. Monitor with `requestAdapterInfo()`. 96³ is the intermediate fallback (56 MB per voxel buffer, ~112 MB total). |

---

## 14. External Dependencies

| Dependency | Purpose | Version Constraint |
|------------|---------|-------------------|
| `wgpu` | WebGPU abstraction for Rust | Latest stable with `webgpu` backend |
| `wasm-bindgen` | Rust/JS interop | Latest stable |
| `web-sys` | Browser API bindings | Latest stable |
| `js-sys` | JavaScript API bindings | Latest stable |
| `glam` | Math library (vectors, matrices) | Latest stable, `no_std` compatible |
| `wasm-pack` | Build toolchain | Latest stable |

No game engine. No ECS framework. No rendering library beyond raw wgpu. These would add abstraction layers that obscure the GPU pipeline and bloat the WASM binary.
