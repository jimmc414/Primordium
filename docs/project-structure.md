# Project Structure: Voxel Ecosystem Simulator

**Version:** 0.1.0-draft
**Status:** Draft — pending review before technical-constraints phase
**Prerequisites:** requirements.md, architecture.md, milestones.md

---

## 1. Repository Layout

```
voxel-ecosystem/
├── Cargo.toml                          # Workspace root
├── CLAUDE.md                           # Agent rules (created separately)
├── README.md                           # Project overview, build instructions
├── docs/
│   ├── requirements.md
│   ├── architecture.md
│   ├── milestones.md
│   ├── project-structure.md            # This file
│   ├── technical-constraints.md
│   ├── test-strategy.md
│   └── agent-prompt.md
├── crates/
│   ├── types/                          # Shared types between sim-core and renderer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Re-exports
│   │       ├── voxel.rs               # Voxel struct, pack/unpack, type constants
│   │       ├── intent.rs              # Intent encoding/decoding
│   │       ├── genome.rs              # Genome field accessors
│   │       ├── params.rs              # SimParams struct (shared between crates)
│   │       ├── grid.rs               # Grid coordinate math, neighbor offsets
│   │       └── commands.rs           # Player command struct, encode/decode
│   ├── sim-core/                       # GPU simulation engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Public API: SimEngine
│   │       ├── buffers.rs              # Buffer allocation and management
│   │       ├── pipelines.rs            # Compute pipeline creation
│   │       ├── tick.rs                 # Tick dispatch orchestration
│   │       ├── stats.rs               # Stats readback pipeline
│   │       ├── uniform.rs             # SimParams → uniform buffer upload
│   │       └── sparse.rs             # Brick pool allocator, spatial hash map (M9)
│   ├── renderer/                       # GPU rendering
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # Public API: Renderer
│   │       ├── camera.rs              # Camera state and matrix computation
│   │       ├── ray_march.rs           # Render pipeline setup
│   │       ├── render_texture.rs      # Render texture compute pass
│   │       ├── wireframe.rs           # Bounding box wireframe
│   │       └── picker.rs             # Voxel picking ray cast
│   └── host/                           # WASM entry point and orchestration
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                  # wasm_bindgen entry point, main loop
│           ├── gpu.rs                  # WebGPU device init, adapter detection
│           ├── timing.rs              # Tick scheduling, frame timing
│           └── bridge.rs             # JS ↔ Rust API boundary
├── shaders/
│   ├── common.wgsl                    # Shared types, constants, PRNG, genome decode
│   ├── update_render_texture.wgsl     # Voxel → RGBA8 3D texture
│   ├── ray_march.wgsl                 # Vertex + fragment: ray march renderer
│   ├── wireframe.wgsl                 # Vertex + fragment: bounding box lines
│   ├── apply_commands.wgsl            # Player command processing
│   ├── temperature_diffusion.wgsl     # Temperature field diffusion pass
│   ├── intent_declaration.wgsl        # Protocell intent evaluation
│   ├── resolve_execute.wgsl           # Conflict resolution + state update
│   ├── stats_reduction.wgsl           # Parallel reduction for statistics
│   ├── pick_voxel.wgsl               # Ray cast for voxel inspector
│   └── brick_common.wgsl             # Brick coordinate math, hash map lookup (M9 sparse)
└── web/
    ├── index.html                      # Entry page, canvas, WASM loader
    ├── style.css                       # UI styling
    ├── ui.js                           # Stats display, parameter controls, tool palette
    └── input.js                        # Mouse/keyboard event handling, tool state machine
```

---

## 2. Crate Dependency Graph

```
host
 ├── sim-core
 │    ├── types
 │    └── wgpu, glam
 ├── renderer
 │    ├── types
 │    └── wgpu, glam
 ├── types
 ├── wasm-bindgen
 ├── web-sys
 └── js-sys
```

`sim-core` and `renderer` do NOT depend on each other. They both depend on `types` for shared data definitions: voxel layout, intent encoding, genome field accessors, grid math, and `SimParams`. They share the `wgpu::Device` and buffer handles passed in by `host`.

`types` has no dependencies on `wgpu`, `wasm-bindgen`, or any GPU/browser crate. It is pure Rust data types and pack/unpack logic. It MAY depend on `glam` for vector types if grid math needs it.

`host` is the only crate compiled to WASM. `sim-core` and `renderer` are pure Rust libraries that operate on `wgpu` types. `types` is a pure data crate.

---

## 3. API Boundaries

### 3.1 types Public API

```
// Voxel layout: pack/unpack between Rust struct and 8 × u32 GPU representation
Voxel {
    voxel_type: VoxelType,
    flags: u8,
    energy: u16,
    age: u16,
    species_id: u16,
    genome: Genome,
    extra: [u32; 2],
}

VoxelType — enum with u8 repr: Empty, Wall, Nutrient, EnergySource, Protocell,
            Waste, HeatSource, ColdSource

Voxel::pack(&self) → [u32; 8]
    Packs the struct into 8 u32 words matching the WGSL layout exactly.

Voxel::unpack(words: &[u32; 8]) → Voxel
    Unpacks 8 u32 words into a Voxel struct.

Genome {
    bytes: [u8; 16],
}

Genome::metabolic_efficiency(&self) → u8      // byte 0
Genome::metabolic_rate(&self) → u8            // byte 1
Genome::replication_threshold(&self) → u8     // byte 2
Genome::mutation_rate(&self) → u8             // byte 3
Genome::movement_bias(&self) → u8             // byte 4
Genome::chemotaxis_strength(&self) → u8       // byte 5
Genome::toxin_resistance(&self) → u8          // byte 6
Genome::predation_capability(&self) → u8      // byte 7
Genome::predation_aggression(&self) → u8      // byte 8
Genome::photosynthetic_rate(&self) → u8       // byte 9
Genome::energy_split_ratio(&self) → u8        // byte 10
Genome::species_id(&self) → u16              // computed hash

// Intent encoding — mirrors WGSL intent word format
Intent::encode(action: ActionType, direction: Direction, bid: u32) → u32
Intent::decode(word: u32) → (ActionType, Direction, u32)

ActionType — enum: NoAction, Die, Predate, Replicate, Move, Idle
Direction — enum: PosX, NegX, PosY, NegY, PosZ, NegZ, Self_

// Grid math
grid_index(x, y, z, grid_size) → usize
grid_coords(index, grid_size) → (u32, u32, u32)
neighbor_offsets() → [(i32, i32, i32); 6]

// Simulation parameters — flat struct, all f32 for uniform buffer compatibility
SimParams { ... }
SimParams::to_bytes(&self) → Vec<u8>
```

**Roundtrip correctness test:** The `types` crate MUST include a unit test that constructs known `Voxel` values, packs them to `[u32; 8]`, unpacks them, and asserts field equality. This test is the single source of truth for verifying that the Rust pack/unpack agrees with the WGSL struct layout. If this test passes, Rust and WGSL agree. If it fails, the Rust side is wrong (WGSL is authoritative per §7).

The same pattern applies to `Intent::encode/decode` and `SimParams::to_bytes`.

### 3.2 sim-core Public API

```
SimEngine::new(device, queue, grid_size) → SimEngine
    Allocates all simulation buffers. Returns error if allocation fails.

SimEngine::tick(encoder, sim_params: &SimParams, commands: &[Command]) → ()
    Encodes all simulation compute dispatches into the provided command encoder.
    Does NOT submit — host controls submission.
    sim_params and Command types are from the types crate.

SimEngine::current_read_buffer() → &Buffer
    Returns whichever voxel buffer was most recently written (for renderer to read).

SimEngine::current_temp_buffer() → &Buffer
    Returns current temperature buffer.

SimEngine::stats_readback(device) → Option<SimStats>
    Non-blocking. Returns Some(stats) if the most recent async map has completed.
    Returns None if still pending. Caller must poll.

SimEngine::initialize_grid(queue, seed_config) → ()
    Writes initial voxel state using types::Voxel::pack() for safe construction.
```

**What sim-core does NOT expose:**
- Individual buffer handles (except current read buffers for renderer binding)
- Internal pipeline objects or bind groups
- Any shader source or compilation
- Tick count (internal bookkeeping — host tracks its own tick count for timing)

### 3.3 renderer Public API

```
Renderer::new(device, surface_config, grid_size) → Renderer
    Creates render pipelines, render texture, wireframe pipeline.

Renderer::update_render_texture(encoder, voxel_buf, temp_buf, overlay_mode) → ()
    Encodes the render texture compute pass.

Renderer::render_frame(encoder, surface_view, camera) → ()
    Encodes the ray march render pass and wireframe pass.

Renderer::pick_voxel(encoder, camera, screen_x, screen_y) → ()
    Encodes the pick compute dispatch. Result available via pick_result().

Renderer::pick_result(device) → Option<PickResult>
    Non-blocking readback of pick result.

Camera::new() → Camera
Camera::orbit(dx, dy) → ()
Camera::zoom(delta) → ()
Camera::pan(dx, dy) → ()
Camera::clip_plane() → ClipPlane
Camera::set_clip_axis(axis: Option<Axis>) → ()
Camera::set_clip_position(t: f32) → ()
Camera::view_projection_inverse() → Mat4
```

**What renderer does NOT expose:**
- The 3D render texture handle (internal detail)
- Ray march shader internals
- Surface configuration after creation

### 3.4 host ↔ JavaScript Bridge

Exposed via `wasm_bindgen`:

```
#[wasm_bindgen]
init() → Promise<()>
    Initializes WebGPU, creates SimEngine and Renderer. Rejects if WebGPU unavailable.

#[wasm_bindgen]
frame(dt: f32) → ()
    Called from requestAnimationFrame. Handles tick scheduling and rendering.

#[wasm_bindgen]
set_tool(tool_id: u32) → ()
set_brush_radius(r: u32) → ()
on_mouse_down(x: f32, y: f32, button: u32) → ()
on_mouse_move(x: f32, y: f32, dx: f32, dy: f32, buttons: u32) → ()
on_mouse_up(x: f32, y: f32, button: u32) → ()
on_scroll(delta: f32) → ()
on_key_down(key: String) → ()
on_key_up(key: String) → ()

#[wasm_bindgen]
set_param(key: String, value: f32) → ()
    Updates a simulation parameter by name.

#[wasm_bindgen]
get_stats() → JsValue
    Returns current stats as a JS object. Returns null if no stats available yet.

#[wasm_bindgen]
get_pick_result() → JsValue
    Returns voxel inspector data for most recent pick. Null if pending or no pick.

#[wasm_bindgen]
set_paused(paused: bool) → ()
single_step() → ()
set_tick_rate(ticks_per_sec: f32) → ()
set_overlay_mode(mode: u32) → ()
load_preset(preset_id: u32) → ()
```

**What the bridge does NOT expose:**
- Raw GPU buffer handles or bind groups
- Direct encoder access
- Any async/await from Rust side (all async GPU operations are managed internally; JS only polls results)

---

## 4. Shader Dependency Map

`common.wgsl` is included (via WGSL `#import` or string concatenation at pipeline creation time) by every other shader. It defines:

- Voxel struct layout (8 × u32)
- Voxel type constants
- Intent encoding/decoding functions
- PRNG (PCG hash) function
- Genome byte extraction helpers
- Grid coordinate ↔ linear index conversion
- Neighbor offset table (von Neumann 6-neighborhood)

```
common.wgsl
 ├── update_render_texture.wgsl
 ├── apply_commands.wgsl
 ├── temperature_diffusion.wgsl
 ├── intent_declaration.wgsl
 ├── resolve_execute.wgsl
 ├── stats_reduction.wgsl
 └── pick_voxel.wgsl

common.wgsl + brick_common.wgsl  (sparse-mode pipeline variants)

ray_march.wgsl       (standalone — reads 3D texture, no voxel struct)
wireframe.wgsl        (standalone — just geometry)
```

---

## 5. File-by-Milestone Mapping

### Legend

- **[CREATE]** — File is created in this milestone.
- **[MODIFY]** — File exists from a prior milestone and is modified.
- **[SKIP]** — File is in the final layout but MUST NOT be created yet. Reasons given.

---

### M1: GPU Bootstrap and Static Rendering

**Create:**

| File | Notes |
|------|-------|
| `Cargo.toml` (workspace) | Workspace with four members |
| `crates/types/Cargo.toml` | Dependencies: glam (optional) |
| `crates/types/src/lib.rs` | Re-exports voxel, genome, grid, params |
| `crates/types/src/voxel.rs` | Voxel struct, VoxelType enum, pack/unpack. Roundtrip unit test. |
| `crates/types/src/genome.rs` | Genome struct, named field accessors, species_id hash. |
| `crates/types/src/grid.rs` | grid_index, grid_coords, neighbor_offsets. |
| `crates/types/src/params.rs` | SimParams struct, to_bytes(). All fields defined even if unused in M1. |
| `crates/host/Cargo.toml` | Dependencies: wgpu, wasm-bindgen, web-sys, js-sys, glam, types |
| `crates/host/src/lib.rs` | `init()`, `frame()` with render-only loop (no simulation tick) |
| `crates/host/src/gpu.rs` | Adapter request, device creation, surface config, capability detection |
| `crates/host/src/timing.rs` | Frame timing only. No tick accumulator yet. |
| `crates/host/src/bridge.rs` | Camera input handlers: `on_mouse_down/move/up`, `on_scroll`, `on_key_down/up` |
| `crates/renderer/Cargo.toml` | Dependencies: wgpu, glam, types |
| `crates/renderer/src/lib.rs` | `Renderer::new()`, `update_render_texture()`, `render_frame()` |
| `crates/renderer/src/camera.rs` | Full camera implementation: orbit, zoom, pan, clip plane |
| `crates/renderer/src/ray_march.rs` | Render pipeline: full-screen triangle, ray march fragment shader |
| `crates/renderer/src/render_texture.rs` | Compute pass: voxel buf → 3D RGBA8 texture |
| `crates/renderer/src/wireframe.rs` | 12-line bounding box |
| `crates/sim-core/Cargo.toml` | Dependencies: wgpu, glam, types |
| `crates/sim-core/src/lib.rs` | `SimEngine::new()` — allocates `voxel_buf_a` only. `initialize_grid()` for test voxels using `types::Voxel::pack()`. |
| `crates/sim-core/src/buffers.rs` | Buffer allocation helper. Single-buffer mode (no double buffer). |
| `crates/sim-core/src/uniform.rs` | Uploads `SimParams::to_bytes()` to GPU uniform buffer. |
| `shaders/common.wgsl` | Voxel struct, type constants, coordinate helpers, PRNG. |
| `shaders/update_render_texture.wgsl` | Color mapping for all voxel types. |
| `shaders/ray_march.wgsl` | Vertex + fragment shader. |
| `shaders/wireframe.wgsl` | Line vertex + fragment shader. |
| `web/index.html` | Canvas, WASM loader script. |
| `web/style.css` | Canvas fullscreen, minimal styling. |
| `web/input.js` | Mouse/keyboard event capture, forwards to WASM bridge. |

**Skip — do NOT create in M1:**

| File | Reason |
|------|--------|
| `crates/sim-core/src/tick.rs` | No simulation loop yet. |
| `crates/sim-core/src/commands.rs` | No player commands yet. |
| `crates/sim-core/src/stats.rs` | No stats yet. |
| `crates/sim-core/src/pipelines.rs` | No compute pipelines beyond render texture. That one lives in renderer. |
| `crates/types/src/intent.rs` | No intent system until M3. Created then. |
| `crates/renderer/src/picker.rs` | No picking yet. |
| `crates/host/src/timing.rs` → tick logic | File exists but tick accumulator logic is absent. Only frame dt tracking. |
| `shaders/apply_commands.wgsl` | No commands. |
| `shaders/temperature_diffusion.wgsl` | No temperature. |
| `shaders/intent_declaration.wgsl` | No intents. |
| `shaders/resolve_execute.wgsl` | No conflict resolution. |
| `shaders/stats_reduction.wgsl` | No stats. |
| `shaders/pick_voxel.wgsl` | No picking. |
| `web/ui.js` | No UI controls beyond canvas. Camera input is in `input.js`. |

---

### M2: Simulation Loop, Metabolism, and Death

**Create:**

| File | Notes |
|------|-------|
| `crates/sim-core/src/tick.rs` | `SimEngine::tick()` — encodes CA update dispatch. Single-pass (no intents yet). |
| `crates/sim-core/src/pipelines.rs` | Creates `ca_update` compute pipeline. |

**Modify:**

| File | Change |
|------|--------|
| `crates/sim-core/src/lib.rs` | Add `tick()`, `current_read_buffer()`. |
| `crates/sim-core/src/buffers.rs` | Allocate `voxel_buf_b`. Implement double-buffer swap. Two bind groups. |
| `crates/host/src/lib.rs` | Add tick accumulator to `frame()`. Call `sim_engine.tick()` when due. |
| `crates/host/src/timing.rs` | Add tick accumulator, pause/resume state, configurable tick rate. |
| `crates/host/src/bridge.rs` | Add `set_paused()`, `single_step()`, `set_tick_rate()`. |
| `shaders/common.wgsl` | Add PRNG implementation (PCG hash). Genome byte extraction. |

**New shader:**

| File | Notes |
|------|-------|
| `shaders/resolve_execute.wgsl` | Initial version: metabolism only. No intents, no conflict resolution. Named for its final role to avoid a rename later. Handles: protocell energy drain, nutrient consumption, waste decay, nutrient spawning. |

**Skip:**

| File | Reason |
|------|--------|
| `shaders/intent_declaration.wgsl` | No intents until M3. Metabolism is a single-pass update. |
| `shaders/apply_commands.wgsl` | No commands until M4. |
| `shaders/temperature_diffusion.wgsl` | No temperature until M5. |

**Important:** The M2 `resolve_execute.wgsl` is a simplified single-pass shader where each voxel reads its own state and neighbors, computes its new state, and writes it. This is NOT the intent/resolve split yet. The shader is intentionally structured so that the metabolism logic (energy consumption, nutrient depletion, waste decay) can be extracted into helper functions and reused when the intent system is introduced in M3.

---

### M3: Replication, Mutation, and the Intent System

**Create:**

| File | Notes |
|------|-------|
| `shaders/intent_declaration.wgsl` | Protocell intent evaluation. M3 version handles: REPLICATE and IDLE only. |
| `crates/types/src/intent.rs` | Intent encode/decode, ActionType enum, Direction enum. Roundtrip unit test. |

**Modify:**

| File | Change |
|------|--------|
| `crates/types/src/lib.rs` | Re-export intent module. |
| `crates/sim-core/src/buffers.rs` | Allocate `intent_buf`. Add clear-at-tick-start logic. |
| `crates/sim-core/src/pipelines.rs` | Create `intent_declaration` pipeline. |
| `crates/sim-core/src/tick.rs` | Change dispatch from single-pass to two-pass: intent → resolve. Clear intent buf. |
| `shaders/resolve_execute.wgsl` | Major rewrite. Now reads `intent_buf`. Implements: EMPTY voxel checks neighbor intents for REPLICATE, resolves bids. Protocell checks own intent outcome. Mutation logic inline. Species ID computation. |
| `shaders/common.wgsl` | Add intent encoding/decoding functions. Species ID hash function. |
| `shaders/update_render_texture.wgsl` | Species → color mapping (golden ratio hue). Energy → brightness. |

**Skip:**

| File | Reason |
|------|--------|
| Movement logic in `intent_declaration.wgsl` | Movement is M4. M3 intent shader handles REPLICATE and IDLE only. |
| `shaders/apply_commands.wgsl` | Commands are M4. |

---

### M4: Movement, Chemotaxis, and Player Commands

**Create:**

| File | Notes |
|------|-------|
| `crates/types/src/commands.rs` | Command struct definition, encode/decode. (Data type, lives in types crate.) |
| `shaders/apply_commands.wgsl` | Processes command buffer, modifies voxel read buffer in-place. |
| `web/ui.js` | Tool palette, brush radius slider. Minimal — no stats, no parameter controls yet. |

**Modify:**

| File | Change |
|------|--------|
| `crates/sim-core/src/buffers.rs` | Allocate `command_buf`. |
| `crates/sim-core/src/pipelines.rs` | Create `apply_commands` pipeline. |
| `crates/sim-core/src/tick.rs` | Insert `apply_commands` dispatch before intent declaration. Accept commands parameter. |
| `crates/sim-core/src/lib.rs` | Add commands parameter to `tick()`. |
| `crates/host/src/lib.rs` | Command construction from input events. Pass to `tick()`. |
| `crates/host/src/bridge.rs` | Add `set_tool()`, `set_brush_radius()`, tool state machine. |
| `shaders/intent_declaration.wgsl` | Add MOVE intent logic: movement bias, chemotaxis, direction selection. Priority: REPLICATE > MOVE > IDLE. |
| `shaders/resolve_execute.wgsl` | Handle MOVE intents at EMPTY voxels. Source voxel → EMPTY on successful move. Movement energy cost. |
| `web/input.js` | Tool-aware click handling: camera controls vs. world interaction based on active tool. |

**Skip:**

| File | Reason |
|------|--------|
| `web/ui.js` → parameter sliders | Parameters are M7. M4 ui.js has tool palette only. |
| `web/ui.js` → stats display | Stats are M7. |
| `crates/sim-core/src/stats.rs` | Stats are M7. |

---

### M5: Temperature System

**Create:**

| File | Notes |
|------|-------|
| `shaders/temperature_diffusion.wgsl` | Diffusion pass. Wall insulation. Heat/cold source anchoring. |

**Modify:**

| File | Change |
|------|--------|
| `crates/sim-core/src/buffers.rs` | Allocate `temp_buf_a`, `temp_buf_b`. Double-buffer sync with voxel buffers. Initialize to 0.5. |
| `crates/sim-core/src/pipelines.rs` | Create `temperature_diffusion` pipeline. |
| `crates/sim-core/src/tick.rs` | Insert diffusion dispatch after commands, before intent declaration. Pass temp buffer to intent and execute dispatches. |
| `crates/sim-core/src/lib.rs` | Add `current_temp_buffer()`. |
| `crates/types/src/params.rs` | Add `temp_sensitivity`, `diffusion_rate` to SimParams. |
| `shaders/intent_declaration.wgsl` | Read temperature. Modify replication threshold by temperature. |
| `shaders/resolve_execute.wgsl` | Read temperature. Apply metabolic cost multiplier. Apply mutation rate multiplier. |
| `shaders/common.wgsl` | Temperature modulation helper functions. |
| `shaders/update_render_texture.wgsl` | Add temperature overlay mode. |
| `web/ui.js` | Add heat/cold source tools. Add overlay mode toggle button. |
| `crates/host/src/bridge.rs` | Add `set_overlay_mode()`. |

**Skip:**

| File | Reason |
|------|--------|
| Predation logic in any shader | Predation is M6. Temperature modulation of predation energy cost is added in M6. |

---

### M6: Predation

**Modify only — no new files.**

| File | Change |
|------|--------|
| `shaders/intent_declaration.wgsl` | Add PREDATE intent. Priority: DIE > PREDATE > REPLICATE > MOVE > IDLE. Scan neighbors for prey (protocells below aggression threshold). |
| `shaders/resolve_execute.wgsl` | Handle PREDATE: prey converts to WASTE, predator gains energy. Simplified model — predation cancels prey movement. |
| `shaders/common.wgsl` | Add intent type constant for PREDATE. |
| `shaders/update_render_texture.wgsl` | Predator visual distinction (high saturation for predation_capability > 128). |

**Skip:**

| File | Reason |
|------|--------|
| Any stats or UI work | M7. Predation dynamics are observable visually at this stage. |

---

### M7: Statistics, Inspector, and UI

**Create:**

| File | Notes |
|------|-------|
| `crates/sim-core/src/stats.rs` | Stats buffer allocation, async readback pipeline, SimStats struct. |
| `crates/renderer/src/picker.rs` | Pick compute pipeline, readback, PickResult struct. |
| `shaders/stats_reduction.wgsl` | Two-stage parallel reduction. |
| `shaders/pick_voxel.wgsl` | Single-workgroup ray cast, writes hit position + voxel data. |

**Modify:**

| File | Change |
|------|--------|
| `crates/sim-core/src/buffers.rs` | Allocate `stats_buf`, `stats_staging`. |
| `crates/sim-core/src/pipelines.rs` | Create `stats_reduction` pipeline. |
| `crates/sim-core/src/tick.rs` | Add stats reduction dispatch as final step. |
| `crates/sim-core/src/lib.rs` | Add `stats_readback()`. |
| `crates/renderer/src/lib.rs` | Add `pick_voxel()`, `pick_result()`. |
| `crates/host/src/lib.rs` | Poll stats and pick results each frame. Forward to JS. |
| `crates/host/src/bridge.rs` | Add `get_stats()`, `get_pick_result()`, `set_param()`. |
| `web/ui.js` | Full UI: stats display, population graph (canvas-based line chart), voxel inspector tooltip, parameter sliders for all SimParams, tick rate slider, single-step button, overlay mode buttons, keyboard shortcut overlay. |

---

### M8: Polish, Performance, and Fallback Tiers

**Create:**

| File | Notes |
|------|-------|
| (none) | M8 modifies existing files only. |

**Modify:**

| File | Change |
|------|--------|
| `crates/host/src/gpu.rs` | GPU tier detection. Grid size selection based on adapter limits and device type. |
| `crates/sim-core/src/buffers.rs` | Parameterize all buffer sizes by `grid_size`. Retry at lower tier on allocation failure. |
| `crates/sim-core/src/lib.rs` | Accept `grid_size` parameter. Propagate to all buffer and dispatch calculations. |
| `crates/renderer/src/lib.rs` | Accept `grid_size`. Render texture size parameterized. |
| `crates/renderer/src/ray_march.rs` | Step count derived from grid size. |
| `crates/host/src/lib.rs` | Loading screen during init. Seeding presets ("Petri Dish", "Gradient", "Arena"). |
| `crates/host/src/bridge.rs` | Add `load_preset()`. |
| `web/ui.js` | Preset selector dropdown. Keyboard shortcut overlay. |
| `web/index.html` | Loading screen HTML/CSS. |
| All shaders | `GRID_SIZE` as override constant or uniform where hardcoded to 128. |

---

### M9: Sparse Grid (Stretch)

**Create:**

| File | Notes |
|------|-------|
| `crates/sim-core/src/sparse.rs` | Brick pool allocator, spatial hash map management, brick lifecycle. |
| `shaders/brick_common.wgsl` | Brick coordinate math, hash map lookup, cross-brick neighbor access. |

**Modify:**

| File | Change |
|------|--------|
| `crates/sim-core/src/buffers.rs` | Conditional: allocate brick pool + hash map instead of flat buffers when sparse mode active. |
| `crates/sim-core/src/pipelines.rs` | Create sparse variants of all compute pipelines (or parameterize existing ones). |
| `shaders/intent_declaration.wgsl` | Replace flat index with brick lookup. |
| `shaders/resolve_execute.wgsl` | Replace flat index with brick lookup. Cross-brick neighbor handling. |
| `shaders/temperature_diffusion.wgsl` | Process allocated bricks only. |
| `shaders/update_render_texture.wgsl` | Process allocated bricks only. |
| `shaders/stats_reduction.wgsl` | Iterate over allocated bricks instead of flat grid. |

---

## 6. File Count by Milestone

| Milestone | New Files | Modified Files | Total Files at End |
|-----------|-----------|----------------|-------------------|
| M1 | 30 | 0 | 30 |
| M2 | 3 | 7 | 33 |
| M3 | 2 | 6 | 35 |
| M4 | 3 | 9 | 38 |
| M5 | 1 | 10 | 39 |
| M6 | 0 | 4 | 39 |
| M7 | 4 | 8 | 43 |
| M8 | 0 | 10+ | 43 |
| M9 | 2 | 7 | 45 |

---

## 7. Naming Conventions

**Crate names:** lowercase with hyphens (`sim-core`, not `sim_core`). Cargo normalizes to underscores for module paths (`sim_core`).

**Shader files:** snake_case matching their compute entry point name. `intent_declaration.wgsl` contains entry point `fn intent_declaration_main(...)`.

**Rust files:** snake_case. One primary type per file. `camera.rs` contains `pub struct Camera`.

**WGSL entry points:** `fn {filename}_main(@builtin(global_invocation_id) gid: vec3<u32>)` for compute shaders. This convention lets the agent find the entry point from the filename without searching.

**Bind group layout:** Each shader file documents its expected bind group layout in a comment header. The host-side pipeline creation code mirrors this layout. If a shader and its Rust pipeline disagree on binding indices, the shader comment is authoritative and the Rust code is wrong.

---

## 8. What MUST NOT Exist

These constraints apply across all milestones:

| Prohibition | Reason |
|-------------|--------|
| No `ECS` crate or entity-component system | Unnecessary abstraction. Voxels are a flat buffer, not entities. |
| No `utils` or `helpers` crate | Dump-drawer modules. Shared code goes in `common.wgsl` (GPU-side) or `types` crate (CPU-side). |
| No separate CSS/JS files per UI component | Single `ui.js` and `style.css`. The UI is simple enough to stay in two files. |
| No build-time shader preprocessing | WGSL `#import` is not standard. Use string concatenation of `common.wgsl` + shader source at pipeline creation time. No build step for shaders. |
| No runtime shader compilation from Rust | All shader source is embedded at build time via `include_str!()`. No file I/O at runtime. |
| No abstraction layer over wgpu | Call wgpu directly. No custom "RenderContext" or "GpuManager" wrapper. The wgpu API is already the abstraction layer. |
| No raw u32 bit manipulation for voxel data outside `types` crate | All voxel pack/unpack goes through `types::Voxel`. Any code constructing or reading voxel data via manual bit shifts is a bug. The `types` crate is the single Rust-side authority for the GPU data layout. |
| No async runtime (tokio, async-std) | All async is `wgpu` futures + browser `requestAnimationFrame`. No runtime needed. |
| No test framework beyond `wasm-pack test` | Browser-based tests use `wasm-pack test --chrome`. No custom harness. |
