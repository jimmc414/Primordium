# CLAUDE.md — Voxel Ecosystem Simulator

## Build

```bash
# Build WASM (run after every change)
wasm-pack build crates/host --target web --release

# Unit tests (types crate, no GPU)
cargo test -p types

# GPU integration tests (needs headless Chrome)
wasm-pack test --chrome --headless crates/sim-core

# Serve for manual testing
python3 -m http.server 8080
# Open http://localhost:8080/web/index.html in Chrome
```

## Project Layout

```
crates/types/       Pure Rust data types. No GPU deps. Voxel, Genome, Intent, Command, SimParams, grid math.
crates/sim-core/    GPU simulation engine. Depends on types + wgpu. Includes sparse.rs for brick-based 256³.
crates/renderer/    GPU rendering. Depends on types + wgpu.
crates/host/        WASM entry point. Depends on all above + wasm-bindgen.
shaders/            WGSL shader files. common.wgsl is prepended to all others; brick_common.wgsl for sparse mode.
web/                HTML/CSS/JS. Thin UI layer.
docs/               Spec documents. Read before coding.
```

## Read Before Each Milestone

Before starting milestone N, read IN ORDER:
1. `docs/milestones.md` — §MN deliverables and acceptance criteria
2. `docs/project-structure.md` — §MN file list and SKIP list
3. `docs/technical-constraints.md` — all constraints tagged ≤ MN
4. `docs/agent-prompt.md` — §MN execution guide
5. `docs/test-strategy.md` — §MN tests

## Critical Rules

### Never Do These

- **`device.poll(Maintain::Wait)` in `src/`** — blocks WASM main thread, freezes page. Allowed ONLY in `tests/`.
- **`unwrap()` / `expect()` in hot path** — panic kills WASM permanently. Use `match`/`if let`. OK in `init()` for unrecoverable errors.
- **Raw u32 bit manipulation for voxel data** — all voxel construction/reading goes through `types::Voxel::pack()`/`unpack()`. No inline bit shifts outside the types crate.
- **WGSL structs for the voxel buffer** — use `array<u32>` with accessor functions in `common.wgsl`. WGSL struct padding is unpredictable and causes silent corruption. Do NOT refactor accessors into a struct.
- **Modify voxel field offsets in `common.wgsl` without updating `types` crate** — accessor offsets and pack/unpack must match. Change both simultaneously, run roundtrip tests.
- **Read from the write buffer in CA update** — violates double-buffer isolation. Read buffer A, write buffer B (or vice versa based on tick parity). Causes checkerboard artifacts.
- **Skip `intent_buf` clear between ticks** — ghost intents from prior tick cause phantom replications. Always `encoder.clear_buffer()` before `intent_declaration`.
- **`energy - cost` without underflow guard** — u16 wraps to 65535. Use saturating subtraction: `select(0u, energy - cost, energy >= cost)`.
- **Add new compute dispatches without justification** — each dispatch costs 5-50µs overhead. Max 5 dispatches per tick. Merge into existing pass if possible.
- **`std::time::Instant`** — not available in WASM. Frame dt comes from JS.
- **Signed integer arithmetic in shaders** — use unsigned throughout. Signed overflow semantics differ from expectation.
- **Write `resolve_execute.wgsl` branching logic without case enumeration** — see agent-prompt.md §M3 Step 3. Cases first, code second. This is mandatory, not a suggestion.
- **Double-validate in resolve pass** — do NOT check "is the target still empty?" in resolve_execute. Preconditions were validated in intent_declaration. Double-checking causes both contenders to back off and the target stays empty forever.
- **Species ID of zero for protocells** — zero is reserved for non-protocells. After hashing genome, if result is 0, set to 1.

### Always Do These

- **Run `cargo test -p types` after changing any data layout** — roundtrip tests catch Rust/WGSL drift.
- **Run determinism tests after any shader change** — checksum at 8³ (100 ticks) AND 32³ (100 ticks). If they fail, revert and debug.
- **Include `grid_size` and `dispatch_salt` in PRNG seed** — `seed = pcg_hash(voxel_index ^ (tick_count * 0x9E3779B9u) ^ (grid_size * 0x85EBCA6Bu) ^ dispatch_salt)`. Salt differs per shader pass (0x1 for intent, 0x2 for resolve) so same voxel gets independent PRNG streams in different dispatches.
- **Consume exactly 21 PRNG advances per protocell per tick** — regardless of which branch is taken. Determinism requires fixed advance count.
- **Clamp temperature to [0.0, 1.0] after diffusion** — prevents NaN propagation.
- **Clamp diffusion_rate to [0.0, 0.25]** — higher values cause oscillation.
- **Check project-structure.md SKIP list before creating any file** — if a file is marked SKIP for the current milestone, do not create it.
- **Commit after each passing test and each completed milestone.**

## Architecture Quick Reference

### Voxel Layout: 32 bytes = 8 × u32

```
Word 0: [0:7] type  [8:15] flags  [16:31] energy (u16)
Word 1: [0:15] age (u16)  [16:31] species_id (u16)
Words 2-5: genome (16 bytes, 4 × u32)
Words 6-7: extra (type-specific state)
```

### SimParams Fields (20 × f32 = 80 bytes)

```
grid_size  tick_count  dt  nutrient_spawn_rate
waste_decay_ticks  nutrient_recycle_rate  movement_energy_cost  base_ambient_temp
metabolic_cost_base  replication_energy_min  energy_from_nutrient  energy_from_source
diffusion_rate  temp_sensitivity  predation_energy_fraction  max_energy
overlay_mode  sparse_mode  brick_grid_dim  max_bricks
```

### Voxel Types

```
0=EMPTY  1=WALL  2=NUTRIENT  3=ENERGY_SOURCE
4=PROTOCELL  5=WASTE  6=HEAT_SOURCE  7=COLD_SOURCE
```

### Genome Byte Map

```
0: metabolic_efficiency    1: metabolic_rate
2: replication_threshold   3: mutation_rate
4: movement_bias           5: chemotaxis_strength
6: toxin_resistance        7: predation_capability
8: predation_aggression    9: photosynthetic_rate
10: energy_split_ratio     11-15: reserved (mutate freely, interpret later)
```

### Intent Encoding (u32)

```
[0:2]  target_direction (0-5 = ±X/Y/Z, 6 = self)
[3:5]  action_type (0=NO_ACTION, 1=DIE, 2=PREDATE, 3=REPLICATE, 4=MOVE, 5=IDLE)
[6:31] bid (26-bit, energy-weighted PRNG value)
```

### Intent Priority

```
DIE > PREDATE > REPLICATE > MOVE > IDLE
```

### Tick Pipeline (5 dispatches)

```
1. apply_player_commands  — modifies read buffer in-place
2. temperature_diffusion  — reads temp_read, writes temp_write
3. intent_declaration     — reads voxel_read + temp_write, writes intent_buf
4. resolve_and_execute    — reads voxel_read + intent_buf + temp_write, writes voxel_write
5. stats_reduction        — reads voxel_write, writes stats_buf
```

In sparse mode, all dispatches use brick_table indirection via `brick_common.wgsl`.
Dispatches iterate over allocated bricks only, not the full 256³ grid.

### Buffer Inventory (128³ Dense)

```
voxel_buf_a:   64 MB    voxel_buf_b:   64 MB
temp_buf_a:     8 MB    temp_buf_b:     8 MB
intent_buf:     8 MB    render_tex:     8 MB
sim_params:   256 B     stats_buf:    128 B
command_buf:    4 KB    TOTAL:       ~152 MB (budget: 160 MB)
```

### Buffer Inventory (256³ Sparse)

```
brick_table:  128 KB    pool_a:     variable (max_bricks × 512 × 32 B)
pool_b:      variable   temp_pool_a: variable (max_bricks × 512 × 4 B)
temp_pool_b: variable   intent_pool: variable (max_bricks × 512 × 4 B)
render_tex:    64 MB    sim_params:  256 B
stats_buf:    128 B     command_buf:   4 KB
```

### Double Buffer Swap

```
Even ticks: read A, write B
Odd ticks:  read B, write A
No copies. Two bind groups, select by tick_count % 2.
```

### Temperature in Mid-Tick

```
temperature_diffusion writes temp_write.
intent_declaration and resolve_execute read temp_write (NOT temp_read).
This is correct — sequential dispatches in same command encoder are ordered.
```

## Grid Tiers

```
Discrete GPU ≥ 256MB VRAM (sparse): 256³  (brick-based, 8³ bricks)
Discrete GPU ≥ 256MB VRAM (dense):  128³
Discrete GPU < 256MB:                96³
Integrated GPU:                      64³
```

## Shader Conventions

- Entry point: `fn {filename}_main(@builtin(global_invocation_id) gid: vec3<u32>)`
- Workgroup size: `@workgroup_size(4, 4, 4)` for all compute shaders
- Bind group layout documented in comment header of each shader — this is authoritative
- `common.wgsl` is concatenated before every shader at pipeline creation via `include_str!()`
- `common.wgsl` has NO entry points — only types, constants, and helper functions
- `brick_common.wgsl` is additionally concatenated for sparse-mode pipeline variants

## Dependency Versions

```
wgpu:          latest stable, features = ["webgpu", "wgsl"] on host only
wasm-bindgen:  latest stable
web-sys:       latest stable
js-sys:        latest stable
glam:          latest stable
```

No game engines. No ECS. No nalgebra. No async runtimes. No bundlers.
