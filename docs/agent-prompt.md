# Agent Prompt: Voxel Ecosystem Simulator

**Version:** 0.1.0-draft
**Status:** Draft — pending review before CLAUDE.md phase
**Prerequisites:** All prior spec documents. Read them in order.

---

## 0. Before You Write Any Code

Read these documents in this order. Do not skip any.

1. `requirements.md` — What the system must do.
2. `architecture.md` — How the system is structured. Pay special attention to §4 (simulation pipeline) and §5 (conflict resolution).
3. `milestones.md` — What you build and when. Each milestone has acceptance criteria. You are done with a milestone when ALL criteria pass.
4. `project-structure.md` — Where every file goes. Which files to create per milestone. Which files to NOT create yet.
5. `technical-constraints.md` — What will break if you ignore it. Read §7 (prohibited patterns) as a checklist before every commit.
6. `test-strategy.md` — What to test and how. The `TestHarness` API in §7.1 is your primary debugging tool.
7. This document — How to execute each milestone.

**The single most important rule:** Check the project-structure.md "Skip" tables before creating any file. If a file is listed as "Skip — do NOT create in MX," do not create it, even if it seems useful. Premature files create import errors, confuse the module graph, and waste context window. Create files exactly when the milestone says to.

---

## 1. General Execution Rules

### 1.1 Build and Test Cycle

After every significant change (new file, new shader, new test):

```bash
# Build
wasm-pack build crates/host --target web --release

# Rust unit tests (types crate — no GPU needed)
cargo test -p types

# GPU integration tests (requires headless Chrome)
wasm-pack test --chrome --headless crates/sim-core
```

If the build fails, fix the build before doing anything else. If a test fails, fix the test before adding new features. Never proceed to the next milestone with failing tests.

### 1.2 Commit Discipline

Commit after each of these events:
- A new file compiles successfully.
- A new test passes.
- A milestone's acceptance criteria are all met.

Commit messages: `M{N}: {what changed}`. Examples: `M1: scaffold workspace and types crate`, `M2: metabolism shader passes energy drain test`, `M3: intent/resolve system with replication`.

### 1.3 Shader Development Workflow

For every shader file:

1. Write the bind group layout as a comment header (what bindings, what types, what access modes).
2. Write the entry point signature with `@workgroup_size` and `@compute`/`@vertex`/`@fragment` annotation.
3. Write the logic.
4. Create the corresponding Rust pipeline in `pipelines.rs`, matching the bind group layout exactly.
5. If the shader reads voxel data, verify the field offsets match `common.wgsl` accessor functions AND the `types` crate pack/unpack.

When modifying an existing shader, re-read the bind group comment header first. If your change requires a new binding, update both the shader header comment and the Rust-side pipeline creation.

### 1.4 When You're Stuck

If a shader produces wrong results and you can't find the bug:

1. Add the 32³ determinism test. Does it pass? If not, suspect a workgroup boundary bug.
2. Reduce to a 4³ grid with 1 protocell. Readback the entire grid. Inspect every voxel.
3. Check the double-buffer swap: are you reading from the correct buffer (tick parity)?
4. Check the intent buffer: is it cleared? Log the intent values for the protocell in question.
5. Check the PRNG: is tick_count being passed as a uniform? Is it incrementing?

If a Rust/WASM build fails with cryptic linker errors:
1. Check that `wgpu` features are correct (see technical-constraints RS-6).
2. Check that only the `host` crate enables the `webgpu` feature.
3. Run `cargo clean` and rebuild.

---

## 2. Milestone Execution Guides

### M1: GPU Bootstrap and Static Rendering

**Duration estimate:** 1–2 sessions.

**Step-by-step:**

**Step 1 — Workspace scaffolding.**
Create the workspace `Cargo.toml` with four members: `types`, `sim-core`, `renderer`, `host`. Create each crate's `Cargo.toml` with dependencies per project-structure.md. Create `shaders/` directory. Verify `cargo check --workspace` passes (all crates compile with empty `lib.rs` files).

**Step 2 — types crate.**
Implement in this order:
1. `grid.rs` — `grid_index`, `grid_coords`, `neighbor_offsets`. Write unit tests. Run them.
2. `voxel.rs` — `VoxelType` enum, `Voxel` struct, `pack()`, `unpack()`. Write the `word_layout_matches_spec` test that checks exact bit positions against the architecture §2.1 table. Write all roundtrip tests. Run them.
3. `genome.rs` — `Genome` struct, byte accessors, `species_id()` hash. Write accessor tests. Write hash stability test with hardcoded expected values (compute them once, then hardcode). Run them.
4. `params.rs` — `SimParams` struct with all fields (use default values). `to_bytes()`. Write length and determinism tests. Run them.
5. `lib.rs` — Re-export all modules.

At this point `cargo test -p types` should pass with ~20 tests. Do not proceed until it does.

**Step 3 — Shaders: common.wgsl.**
Define:
- Voxel type constants (`const VOXEL_EMPTY: u32 = 0u;` ... `const VOXEL_COLD_SOURCE: u32 = 7u;`)
- Grid coordinate functions (`fn grid_index(pos: vec3<u32>, grid_size: u32) -> u32`)
- Voxel accessor functions (`fn voxel_get_type(base: u32, buf: ptr<storage, array<u32>>) -> u32`). Base offset = voxel_index × 8.
- PRNG: PCG-RXS-M-XS-32 (`fn pcg_hash(input: u32) -> u32`, `fn pcg_next(state: ptr<function, u32>) -> u32`). Seed formula: `pcg_hash(voxel_index ^ (tick_count * 0x9E3779B9u) ^ (grid_size * 0x85EBCA6Bu) ^ dispatch_salt)`. Each shader pass uses a different salt constant (intent=0x1, resolve=0x2, etc.) to produce independent streams.
- Neighbor offset array (`const NEIGHBORS: array<vec3<i32>, 6>`)

Critical: the accessor function offsets (which word, which bits) must exactly match the `types::Voxel::pack` layout. Cross-reference the architecture §2.1 table while writing both.

**Step 4 — Shaders: update_render_texture.wgsl.**
Compute shader: reads voxel buffer (`array<u32>`, read-only storage), writes 3D texture (`texture_storage_3d<rgba8unorm, write>`). Color mapping per architecture §7.1. Concatenate `common.wgsl` + this shader at pipeline creation.

**Step 5 — Shaders: ray_march.wgsl.**
Vertex shader: full-screen triangle (3 vertices, no vertex buffer). Fragment shader: compute ray from inverse VP matrix and frag coords, intersect AABB, march through 3D texture, accumulate color front-to-back. Accept clip plane uniform. This is a render pipeline (vertex + fragment), not a compute pipeline.

**Step 6 — Shaders: wireframe.wgsl.**
12 lines. Vertex buffer with 24 vertices (or use line list with 12 × 2 = 24 vertices). Simple pass-through vertex shader, solid color fragment shader.

**Step 7 — renderer crate.**
1. `camera.rs` — Orbit/zoom/pan camera. Store: distance, yaw, pitch, target position, clip axis, clip position. Method `view_projection_inverse()` returns `Mat4` using `glam`. 
2. `render_texture.rs` — Create compute pipeline for `update_render_texture`. Bind group: voxel buffer (binding 0, read), 3D texture (binding 1, write), uniform with overlay mode (binding 2).
3. `ray_march.rs` — Create render pipeline. Bind group: 3D texture as sampled texture (binding 0), sampler (binding 1), camera uniform (binding 2, contains inverse VP matrix + clip plane).
4. `wireframe.rs` — Create render pipeline. Vertex buffer with box corners. Uniform: VP matrix.
5. `lib.rs` — `Renderer::new()`, `update_render_texture()`, `render_frame()`.

**Step 8 — sim-core crate (minimal).**
1. `buffers.rs` — Allocate `voxel_buf_a` only (64 MB at 128³). Helper function to write raw bytes at offset.
2. `uniform.rs` — Create uniform buffer, upload `SimParams::to_bytes()`.
3. `lib.rs` — `SimEngine::new()`, `initialize_grid()`. The init function writes ~100 test voxels at known positions using `types::Voxel::pack()`: a cluster of WALLs, some NUTRIENTs, a few ENERGY_SOURCEs, some PROTOCELLs with dummy genomes, some WASTE, a HEAT_SOURCE, a COLD_SOURCE.

**Step 9 — host crate.**
1. `gpu.rs` — Request adapter, request device with `webgpu` backend, configure surface. Log adapter info and limits.
2. `timing.rs` — Frame dt tracking only (no tick accumulator yet).
3. `bridge.rs` — `wasm_bindgen` exports: `init()`, `frame(dt)`, mouse/keyboard handlers. Forward mouse events to `Camera`.
4. `lib.rs` — `init()`: create device, create `SimEngine` and `Renderer`. `frame()`: call `renderer.update_render_texture()` and `renderer.render_frame()`. No simulation tick.

**Step 10 — web/ files.**
1. `index.html` — Canvas element, loads WASM via `<script type="module">`.
2. `style.css` — Canvas fullscreen.
3. `input.js` — Capture mouse/keyboard events, call WASM bridge functions. Forward `requestAnimationFrame` dt to `frame()`.

**Step 11 — verify.**
`wasm-pack build crates/host --target web --release`. Serve with `python3 -m http.server`. Open Chrome. Verify: voxels visible, camera works, clip plane works, all types have distinct colors. Check WASM size < 2 MB. Check GPU memory logged < 80 MB.

---

### M2: Simulation Loop, Metabolism, and Death

**Duration estimate:** 1 session.

**Step-by-step:**

**Step 1 — Double buffer.**
In `sim-core/buffers.rs`: allocate `voxel_buf_b` (64 MB). Create two bind groups:
- Bind group A: `voxel_buf_a` as read, `voxel_buf_b` as read_write.
- Bind group B: `voxel_buf_b` as read, `voxel_buf_a` as read_write.
Track tick parity. `current_read_buffer()` returns whichever was last written.

**Step 2 — resolve_execute.wgsl (v1: metabolism only).**
This is the initial CA update shader. No intents, no conflict resolution yet.

Bind group: read buffer (binding 0), write buffer (binding 1), sim_params uniform (binding 2), tick_count uniform (binding 3).

For each voxel at `global_invocation_id`:
- Initialize per-voxel PRNG from `pcg_hash(voxel_index ^ (tick_count * 0x9E3779B9u) ^ (grid_size * 0x85EBCA6Bu) ^ 0x2u)` (salt `0x2` for the resolve/execute dispatch; intent_declaration uses `0x1`).
- Read own voxel from read buffer.
- Switch on voxel type:
  - EMPTY: check nutrient spawning (PRNG roll vs nutrient_spawn_rate). If spawn, write NUTRIENT. Else write EMPTY.
  - PROTOCELL: (a) Gain energy from adjacent ENERGY_SOURCE (scan 6 neighbors, sum photosynthetic gains). (b) Gain energy from adjacent NUTRIENT (scan 6 neighbors, consume proportionally). (c) Subtract metabolic cost. Use saturating subtraction (constraint SIM-4). (d) If energy == 0, write WASTE with decay countdown. Else write updated protocell.
  - NUTRIENT: Decrement concentration if any adjacent PROTOCELL consumed this tick. If concentration == 0, write EMPTY. Else write NUTRIENT.
  - WASTE: Decrement decay countdown. If zero, write EMPTY or NUTRIENT per config.
  - All others: copy unchanged.

Structure the PROTOCELL case as helper functions: `fn apply_photosynthesis(...)`, `fn apply_nutrient_consumption(...)`, `fn apply_metabolism(...)`. These will be reused in M3.

**Step 3 — pipelines.rs.**
Create the compute pipeline for `resolve_execute`. Concatenate `common.wgsl` + `resolve_execute.wgsl`. Create pipeline layout with the bind group layout matching the shader header.

**Step 4 — tick.rs.**
`SimEngine::tick()`: encode one compute dispatch with the correct bind group for the current tick parity. Dispatch `(grid_size/4, grid_size/4, grid_size/4)`. Flip tick parity.

**Step 5 — Timing and host integration.**
In `host/timing.rs`: add tick accumulator. In `host/lib.rs`: `frame()` now checks tick accumulator, calls `sim_engine.tick()` when due (capped at 3 ticks per frame), then calls renderer. Renderer reads `sim_engine.current_read_buffer()`.

In `host/bridge.rs`: add `set_paused()`, `single_step()`, `set_tick_rate()`.

**Step 6 — Update seeding.**
Revise `initialize_grid()`: create a cluster of ~50 protocells with random genomes and energy = 500, surrounded by a dense field of nutrients (500+), with 2–3 ENERGY_SOURCEs nearby.

**Step 7 — Test harness.**
Create the `TestHarness` in `sim-core/tests/`. Implement `new()`, `seed_voxels()`, `tick()`, `read_voxel()`, `checksum()` per test-strategy §7.1. Use `device.poll(Maintain::Wait)` for blocking readback (permitted in test code only per test-strategy §7.2).

**Step 8 — Write and run tests.**
Implement all M2 Tier 2 tests from test-strategy §3.1. Run them. Fix failures. Pay special attention to the determinism test — if it fails, suspect double-buffer swap or PRNG seeding.

**Step 9 — Visual verification.**
Build and serve. Observe: protocells near energy sources sustain, distant protocells die, nutrients deplete, waste decays. Pause and resume work. Camera remains responsive during simulation.

---

### M3: Replication, Mutation, and the Intent System

**Duration estimate:** 1–2 sessions. This is the hardest milestone.

**Step-by-step:**

**Step 1 — types::Intent.**
Create `types/src/intent.rs`: `ActionType` enum, `Direction` enum, `Intent::encode()`, `Intent::decode()`. Write roundtrip tests. Run them. Do not proceed to shader work until intent encoding tests pass.

**Step 2 — Intent buffer.**
In `sim-core/buffers.rs`: allocate `intent_buf` (8 MB). Add `encoder.clear_buffer(&intent_buf, 0, None)` call at the start of each tick in `tick.rs`.

**Step 3 — CRITICAL: Write the resolve_execute case enumeration.**
Before writing ANY executable WGSL for the intent-aware resolve_execute shader, write a complete case enumeration as a block comment at the top of `resolve_execute.wgsl`. This is required by technical-constraint SH-1. The enumeration must cover:

```
// ═══════════════════════════════════════════════════════════════
// CASE ENUMERATION — resolve_execute.wgsl
// This section enumerates every case this shader handles.
// It was written BEFORE the implementation and serves as the
// authoritative specification. If the code disagrees with these
// cases, the code is wrong.
// ═══════════════════════════════════════════════════════════════
//
// INPUT: EMPTY VOXEL
//   Case E1: No neighbor intents target this voxel → output EMPTY
//   Case E2: Exactly one neighbor declares REPLICATE targeting here
//            → write offspring (mutated genome, split energy, age=0)
//   Case E3: Exactly one neighbor declares MOVE targeting here
//            → copy mover's state here, deduct movement cost
//   Case E4: Multiple neighbors target here (any mix of REPLICATE/MOVE)
//            → compare bids, highest wins. Apply E2 or E3 for winner.
//            → losers: their source threads handle the loss (see P cases)
//   [M4 addition] Case E5: MOVE and REPLICATE compete for same empty
//            → same bid comparison, winner type determines action
//
// INPUT: PROTOCELL VOXEL
//   Case P1: Own intent = DIE → output WASTE
//   Case P2: Own intent = REPLICATE, targeting neighbor N
//     P2a: This cell won the bid at N → deduct split energy, apply metabolism
//     P2b: This cell lost the bid at N → keep full energy, apply metabolism
//     To determine win/loss: read neighbor N's intent. Read all intents
//     targeting N (this cell + any others). Compare bids. Deterministic.
//   Case P3: Own intent = IDLE → apply metabolism only
//   [M4] Case P4: Own intent = MOVE, targeting neighbor N
//     P4a: Won bid at N → output EMPTY (this cell moved away)
//     P4b: Lost bid at N → stay here, apply metabolism
//   [M6] Case P5: Own intent = PREDATE, targeting neighbor N
//     P5a: Won predation bid → gain prey energy fraction, apply metabolism
//     P5b: Lost predation bid → apply metabolism only (idle fallback)
//
// INPUT: WASTE VOXEL
//   Case W1: Decrement decay countdown
//   Case W2: Countdown reached zero → NUTRIENT or EMPTY per config
//
// INPUT: NUTRIENT VOXEL
//   Case N1: Adjacent protocell exists → decrement concentration
//   Case N2: Concentration zero → EMPTY
//   Case N3: No adjacent protocells → unchanged
//
// INPUT: WALL / ENERGY_SOURCE / HEAT_SOURCE / COLD_SOURCE
//   Case X1: Copy unchanged (player-managed, modified only by commands)
//
// CONFLICT RESOLUTION INVARIANT:
//   Each thread determines its own output by reading:
//   - Its own input voxel (from read buffer)
//   - Its own intent (from intent_buf at its own index)
//   - Up to 6 neighbors' intents (from intent_buf at neighbor indices)
//   No thread reads beyond the immediate neighborhood.
//   Bid comparison is computed redundantly by both the source thread
//   and the target thread — they always agree because the comparison
//   is deterministic on identical inputs.
```

Review this case enumeration for completeness. Then implement.

**Step 4 — intent_declaration.wgsl (v1: REPLICATE and IDLE only).**
Bind group: read buffer (binding 0), intent buffer (binding 1, read_write), sim_params (binding 2), tick_count (binding 3).

For each protocell:
1. Initialize PRNG with a dispatch-specific salt to avoid sequence overlap with resolve_execute: `seed = pcg_hash(voxel_index ^ (tick_count * 0x9E3779B9u) ^ (grid_size * 0x85EBCA6Bu) ^ 0x1u)`. The resolve_execute shader uses `^ 0x2u` instead. Different salts produce independent PRNG streams for the same voxel in the same tick.
2. Consume all 21 PRNG advances regardless of branch (architecture §9.3).
3. If energy > replication_threshold AND at least one adjacent EMPTY neighbor: pick a random EMPTY neighbor direction, compute bid = `pcg_next(&state) % (energy + 1)`, encode intent as REPLICATE.
4. Else: encode IDLE.
5. Non-protocell voxels: write 0 (NO_ACTION).

**Step 5 — resolve_execute.wgsl (v2: intent-aware).**
Refactor from M2's single-pass to the intent-reading version. Implement cases E1–E4, P1–P3, W1–W2, N1–N3, X1 from the case enumeration. P4 and P5 are stubs (commented out, marked for M4 and M6).

Mutation logic is inline in the E2 case: for each of 16 genome bytes, roll PRNG, compare to `mutation_rate`, replace byte if mutation fires. Compute new `species_id` from mutated genome.

**Step 6 — Update tick.rs.**
Change from single dispatch to two dispatches: `intent_declaration` then `resolve_execute`. Clear intent buffer before intent_declaration. Update bind groups to include intent buffer.

**Step 7 — Update update_render_texture.wgsl.**
Species-to-color mapping: `hue = fract(f32(species_id) * 0.618033988749)`. Convert HSV to RGB with `S = 0.7`, `V = energy / max_energy`.

**Step 8 — Tests.**
Write and run all M3 Tier 2 tests from test-strategy §3.2. The `conflict_resolution` test is the most important — it validates that the redundant bid comparison produces consistent results between source and target threads.

Run M2 tests for regression. Run determinism tests at both 8³ and 32³.

---

### M4: Movement, Chemotaxis, and Player Commands

**Duration estimate:** 1–2 sessions.

**Step-by-step:**

**Step 1 — Extend intent_declaration.wgsl.**
Add MOVE intent logic after replication check. Priority: REPLICATE > MOVE > IDLE.

Movement direction selection:
1. Scan 6 neighbors for EMPTY voxels.
2. If chemotaxis_strength > 0, also note which neighbors are NUTRIENT or ENERGY_SOURCE.
3. If chemotactic neighbors exist: bias direction selection toward them. Probability of choosing the chemotactic direction = `chemotaxis_strength / 255`. Otherwise random EMPTY neighbor.
4. Roll PRNG against `movement_bias / 255` to decide if movement happens at all.
5. If moving: encode MOVE intent with direction and bid.

**Step 2 — Extend resolve_execute.wgsl.**
Implement cases E5 (MOVE + REPLICATE compete), P4a (successful move → EMPTY at source), P4b (failed move → stay). Movement energy cost deducted in P4a.

The critical subtlety: when a protocell's MOVE intent wins at the target, this thread (for the source position) must output EMPTY. But this thread also needs to know if it won. It determines this by reading its own intent (MOVE targeting direction D) and all other intents targeting the same destination (neighbors of position D that also declared MOVE or REPLICATE toward D). It computes the bid comparison and determines if it won. If yes → EMPTY. If no → stay with metabolism.

This is the redundant computation pattern from architecture §5. Both the source thread and the target thread compute the same comparison. They agree because the inputs are identical.

**Step 3 — Player command system.**
1. `sim-core/commands.rs`: define `Command` struct matching architecture §4.2 (64 bytes). Write serialization to raw bytes.
2. `sim-core/buffers.rs`: allocate `command_buf` (4 KB, max 64 commands).
3. `shaders/apply_commands.wgsl`: iterate over command buffer, apply each command. This runs before intent_declaration (first dispatch in the tick pipeline).
4. `sim-core/pipelines.rs`: create `apply_commands` pipeline.
5. `sim-core/tick.rs`: insert `apply_commands` dispatch at the start. Accept `commands: &[Command]` parameter.

**Step 4 — Host-side command pipeline.**
1. `host/bridge.rs`: add `set_tool()`, `set_brush_radius()`, `on_mouse_down` (for world interaction). Maintain tool state machine.
2. `host/lib.rs`: on world click, ray cast to find target voxel (temporary: use CPU-side ray march against a simplified occupancy check. The GPU picker comes in M7). Construct command. Write to `command_buf` via `queue.writeBuffer()`.

**Step 5 — web/ui.js.**
Create tool palette: buttons for Wall, Energy Source, Nutrient, Seed Protocells, Toxin. Brush radius slider. Active tool highlight. Call WASM bridge on click.

**Step 6 — web/input.js update.**
Modify mouse handling: left-click with a tool selected = world interaction (send to bridge). Left-click with no tool = camera orbit. Or: always right-drag for camera, left-click for tools. Choose a scheme and be consistent.

**Step 7 — Tests.**
Write and run M4 Tier 2 tests. The chemotaxis test is statistical (see test-strategy §3.3 and §7.4 for failure logging). Run full regression suite.

---

### M5: Temperature System

**Duration estimate:** 1 session.

**Step-by-step:**

**Step 1 — Temperature buffers.**
In `sim-core/buffers.rs`: allocate `temp_buf_a` and `temp_buf_b` (8 MB each, f32 per voxel). Initialize all values to 0.5 (ambient). Double-buffer swap synchronized with voxel buffers (same tick parity).

**Step 2 — temperature_diffusion.wgsl.**
Bind group: temp read buffer (binding 0), temp write buffer (binding 1), voxel read buffer (binding 2, for wall check), sim_params (binding 3).

For each voxel:
1. Read own temperature from temp_read.
2. Read own voxel type from voxel_read.
3. If WALL: write own temperature unchanged (walls are inert).
4. If HEAT_SOURCE or COLD_SOURCE: write target temperature (Dirichlet boundary).
5. Else: compute average of non-wall, non-out-of-bounds face neighbors' temperatures. Apply diffusion: `t_new = t + diffusion_rate * (t_avg - t)`. Clamp to [0.0, 1.0] (constraint SIM-6). Write.

**Step 3 — Integrate temperature into simulation pipeline.**
`tick.rs` dispatch order becomes: apply_commands → temperature_diffusion → intent_declaration → resolve_execute → (stats stub).

Update bind groups for intent_declaration and resolve_execute to include the temperature read buffer (newly written by diffusion, so use temp_write as the read source for subsequent passes within the same tick).

Wait — clarification on buffer usage within a tick: temperature_diffusion reads `temp_read` and writes `temp_write`. Then intent_declaration and resolve_execute need to read the NEW temperature (the one just written). They should bind `temp_write` as read. This is safe because temperature_diffusion is complete before intent_declaration begins (sequential dispatches in the same command encoder).

**Step 4 — Temperature modulation in intent_declaration.wgsl.**
Read local temperature. Compute `temp_modifier = 1.0 + temp_sensitivity * (local_temp - 0.5)`. Apply to replication_threshold: `effective_threshold = replication_threshold * temp_modifier` (higher temperature → higher threshold → harder to replicate? Or: temperature increases mutation rate but doesn't change replication threshold?).

Design decision: temperature affects MUTATION RATE and METABOLIC COST only, not replication threshold directly. This matches requirements SG-6. The replication threshold is genome-encoded and temperature-independent. Temperature makes replication more expensive (higher metabolic cost drains energy faster, making it harder to reach the threshold) and more mutagenic (offspring are more diverse in hot zones).

**Step 5 — Temperature modulation in resolve_execute.wgsl.**
In the metabolism calculation: `effective_cost = base_cost * temp_modifier`.
In the mutation calculation (E2 case): `effective_mutation_rate = mutation_rate * temp_modifier`. Clamp to [0, 255].

**Step 6 — Player tools for heat/cold sources.**
Add HEAT_SOURCE and COLD_SOURCE to the tool palette in `ui.js`. Add corresponding command types in `commands.rs` and `apply_commands.wgsl`.

**Step 7 — Temperature overlay.**
Add a uniform to `update_render_texture.wgsl` for overlay mode. Mode 0: normal colors. Mode 1: temperature → blue-red gradient. Bind the temperature buffer to the render texture shader.

Add overlay toggle button to `ui.js`. Wire to `host/bridge.rs` → `set_overlay_mode()`.

**Step 8 — Update types::SimParams.**
Add `diffusion_rate` (default 0.1, clamped to [0.0, 0.25]) and `temp_sensitivity` (default 1.0) to `SimParams`. Update `to_bytes()`. Run types tests to verify.

**Step 9 — Tests.**
Write and run M5 Tier 2 tests from test-strategy §3.4. The `diffusion_stability` test (1000 ticks, no NaN, convergence) is essential — run it early to catch clamping issues. Run full regression suite.

---

### M6: Predation

**Duration estimate:** 1 session.

**Step-by-step:**

**Step 1 — Update the case enumeration in resolve_execute.wgsl.**
Uncomment and finalize cases P5a and P5b from the M3 enumeration. Add:

```
// Case P5: Own intent = PREDATE, targeting neighbor N
//   P5a: This cell's predation bid is highest among all predators targeting N
//        → Prey at N converts to WASTE. This cell gains predation_energy_fraction * prey_energy.
//        → Apply metabolism.
//   P5b: Another predator had a higher bid → this cell's predation fails.
//        → Fallback to IDLE behavior. Apply metabolism only.
//
// PREDATION AT TARGET (from prey's perspective):
//   Case PP1: This protocell is targeted by one or more PREDATE intents.
//     PP1a: Highest-bid predator wins. This cell → WASTE.
//            Prey's own intent (move, replicate, etc.) is cancelled.
//     PP1b: No predator wins (none targeting this cell) → process normally.
//
// Note: Predation uses simplified model (architecture §5):
//   - Predation always succeeds if bid wins. Prey movement is cancelled.
//   - Prey does NOT escape even if it declared MOVE.
//   - This preserves the invariant: each thread's output depends only
//     on its own cell and immediate neighbors' intents.
```

**Step 2 — Extend intent_declaration.wgsl.**
Insert PREDATE evaluation between DIE and REPLICATE in the priority chain: DIE > PREDATE > REPLICATE > MOVE > IDLE.

Predation logic:
1. If `predation_capability == 0`: skip (not a predator).
2. Scan 6 neighbors. For each neighbor that is a PROTOCELL with energy < `predation_aggression` threshold (scaled): record as potential prey.
3. If at least one prey found: pick one (lowest energy, or random among tied). Encode PREDATE with direction and bid = `pcg_next(&state) % (energy + 1)`.

**Step 3 — Extend resolve_execute.wgsl.**
For PROTOCELL input voxels: add P5 handling. Check if any neighbor declared PREDATE targeting this voxel's position (by checking if any neighbor's intent has direction pointing here and action = PREDATE). If so, compare bids. If this cell is the prey and the predator won: output WASTE.

For PROTOCELL input voxels that are predators: check own PREDATE intent. Determine if own bid won at the target (same redundant comparison pattern as MOVE/REPLICATE). If won: add `predation_energy_fraction * prey_energy` to own energy. Apply metabolism.

**Step 4 — SimParams update.**
Add `predation_energy_fraction` (default 0.5) to `SimParams`. Update `to_bytes()`.

**Step 5 — Visual distinction.**
In `update_render_texture.wgsl`: for protocells with `predation_capability > 128`, boost saturation to 1.0 (from 0.7). Subtle but visible distinction between predators and non-predators.

**Step 6 — Tests.**
Write and run all M6 Tier 2 tests from test-strategy §3.5. The `predation_cancels_prey_movement` test validates the simplified predation model. The `predation_conflict_two_predators` test validates bid resolution for predation.

Run FULL regression suite (M2–M5). Predation adds the most complex new code path — regressions are likely. Pay special attention to the determinism tests.

---

### M7: Statistics, Inspector, and UI

**Execution guide:** Follow milestones.md §M7 deliverables and project-structure.md §M7. Key implementation note: the stats reduction shader uses a two-stage parallel reduction per architecture §4.6. Stage 1 uses workgroup shared memory; stage 2 reduces across workgroups. The species histogram uses a small (64-entry) open-addressing hash table in shared memory — accept collisions and approximate counts.

The voxel picker (`pick_voxel.wgsl`) dispatches a single workgroup that performs a ray march and writes the hit position + voxel data to a 64-byte output buffer. This is triggered on click, not every frame.

The `ui.js` population graph uses a 2D canvas with manual line drawing. No charting library. Store the last 500 data points in a JavaScript array. Draw on each stats update (every 10 ticks).

---

### M8: Polish, Performance, and Fallback Tiers

**Execution guide:** Follow milestones.md §M8 deliverables. The critical task is parameterizing grid size throughout the codebase — search for any hardcoded `128`, `128*128`, `128*128*128`, or `2097152` and replace with expressions derived from the `grid_size` constant. The grid_size is set once during `init()` based on GPU tier detection and never changes during the session.

The benchmark harness seeds protocells to 30% occupancy and measures: ticks per second (averaged over 100 ticks), frame time (averaged over 300 frames), and memory usage. Results are logged to console for the developer. Pass/fail thresholds are from requirements.md §3.

---

## 3. Common Failure Modes and Fixes

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| Black screen, no voxels | Render texture not being updated, or 3D texture not bound to ray march shader | Check that `update_render_texture` dispatch runs. Check bind group bindings. |
| All voxels same color | Species ID is always 0, or hue mapping is wrong | Check `species_id` hash function in common.wgsl. Check that `species_id != 0` guard exists (constraint SIM-5). |
| Protocells never die | Energy underflow → u16 wraps to 65535 | Add saturating subtraction (constraint SIM-4). |
| Population explodes infinitely | Nutrient spawning rate too high, or metabolism cost too low | Check SimParams defaults. Nutrient spawn rate should be ~0.001 (1 in 1000 EMPTY voxels per tick). |
| Simulation freezes after first tick | Double-buffer swap not happening | Check tick parity flip in `tick.rs`. Verify bind group selection. |
| Checkerboard artifacts | Reading from write buffer (double-buffer violation) | Verify bind groups: even ticks read A/write B, odd ticks read B/write A. |
| Replication never happens | Replication threshold too high relative to energy gains | Lower threshold or increase photosynthetic/nutrient gain rates in SimParams. |
| Stale intents cause phantom actions | Intent buffer not cleared between ticks | Verify `encoder.clear_buffer(&intent_buf, ...)` runs before intent_declaration dispatch. (Constraint SIM-2.) |
| Determinism test fails | PRNG seed doesn't include tick_count, or floating-point in critical path | Check seed formula includes tick_count. Check that all bid/energy/genome operations are integer. (Constraints SIM-3, SH-6.) |
| Movement always goes +X | Chemotaxis or direction selection only checks first neighbor | Verify direction selection iterates all 6 neighbors and uses PRNG for randomness. |
| Predator never predates | Aggression threshold comparison is inverted | Check: prey is valid if prey.energy < predator.predation_aggression (scaled). Not the reverse. |
| Simulation correct for N ticks, then diverges | PRNG seed collision. Two voxels with coordinates that hash to the same seed make identical decisions for one tick, causing a cascade. More likely at small grid sizes (8³) where the coordinate space is small. | Verify seed includes `grid_size` and `dispatch_salt`: `seed = pcg_hash(voxel_index ^ (tick_count * 0x9E3779B9u) ^ (grid_size * 0x85EBCA6Bu) ^ dispatch_salt)`. If divergence only occurs at 8³ but not 32³, this is almost certainly the cause. |
