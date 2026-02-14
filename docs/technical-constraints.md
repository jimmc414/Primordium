# Technical Constraints: Voxel Ecosystem Simulator

**Version:** 0.1.0-draft
**Status:** Draft — pending review before test-strategy phase
**Prerequisites:** requirements.md, architecture.md, milestones.md, project-structure.md

---

## How to Use This Document

Each constraint has a **Milestone** tag indicating when it first becomes relevant. Constraints accumulate — a constraint tagged M1 applies to all subsequent milestones. When implementing milestone N, read all constraints tagged ≤ N.

---

## 1. WebGPU Platform Constraints

### WG-1: Buffer Size Limits — M1

WebGPU `maxBufferSize` varies by device. The spec guarantees at least 256 MB, but some integrated GPUs report lower limits. The `maxStorageBufferBindingSize` (the maximum a single binding can reference) is often lower than `maxBufferSize` — some devices cap it at 128 MB.

**Impact:** `voxel_buf_a` and `voxel_buf_b` are each 64 MB at 128³. This is within limits for most discrete GPUs. At 256³ dense (1 GB per buffer), allocation will fail on most consumer hardware — this is why the stretch goal uses sparse representation.

**Rule:** Query `device.limits().max_buffer_size` and `device.limits().max_storage_buffer_binding_size` at startup. If either is less than 64 MB, fall back to 96³ or 64³. Log the actual limits.

### WG-2: Uniform Buffer Size — M1

WebGPU `maxUniformBufferBindingSize` minimum is 64 KB. `SimParams` is ~256 bytes. No risk now, but uniforms cannot grow unboundedly.

**Rule:** Keep `SimParams` under 1 KB. If it grows beyond that, move to a storage buffer binding.

### WG-3: Workgroup Size Limits — M1

WebGPU guarantees `maxComputeInvocationsPerWorkgroup` ≥ 256 and `maxComputeWorkgroupSizeX/Y/Z` ≥ 256. The architecture uses `(4, 4, 4)` = 64 invocations, well within limits.

**Rule:** Do not exceed workgroup size of 64 without querying device limits. An `(8, 8, 8)` = 512 workgroup will fail on some devices.

### WG-4: No Timestamp Queries on All Devices — M8

`timestamp-query` is an optional WebGPU feature. Many devices (especially integrated GPUs and some mobile-class adapters) do not support it. Required only for the M8 performance profiling pass.

**Rule:** Feature-gate timestamp queries behind `adapter.features().contains(Features::TIMESTAMP_QUERY)`. Never require it. M8 profiling must degrade to wall-clock timing when unavailable.

### WG-5: Storage Texture Format Restrictions — M1

WebGPU compute shaders can write to storage textures, but the set of writable formats is limited. `rgba8unorm` is writable in the `"bgra8unorm-storage"` extension or via `rgba8unorm` storage directly. Some implementations only support `rgba8uint` or `rgba8sint` for storage texture writes.

**Rule:** Use `rgba8unorm` for the render texture. If the device does not support `rgba8unorm` as a storage texture format, fall back to `rgba8uint` in the compute pass and normalize in the fragment shader. Test on Chrome's software rasterizer (SwiftShader) to catch this early.

### WG-6: No Shared Memory Across Dispatches — M3

WGSL workgroup shared memory (`var<workgroup>`) is scoped to a single dispatch. There is no persistent on-chip memory between dispatches and no global shared memory across workgroups within a dispatch.

**Impact:** The intent/resolve two-pass architecture exists because of this constraint. Intents must be written to a global storage buffer (`intent_buf`) so the next dispatch can read them.

**Rule:** Never attempt to communicate between dispatches except via storage buffers. Never assume workgroup execution order within a dispatch.

### WG-7: No Recursion or Function Pointers in WGSL — All Milestones

WGSL does not support recursion, function pointers, or dynamic dispatch. All control flow must be statically resolvable.

**Rule:** All shader logic uses if/else chains and loops with bounded iteration counts. The genome interpreter must not be implemented as a virtual machine with a program counter — it must be a fixed sequence of parameter reads.

### WG-8: WGSL Integer Overflow is Defined — All Milestones

WGSL wraps unsigned integer overflow (modular arithmetic) and defines signed integer overflow as wrapping. This differs from C/C++ (undefined behavior for signed) and Rust (panic in debug, wrap in release).

**Impact:** This is actually helpful — the PRNG relies on wrapping multiplication. No special handling needed.

**Rule:** Rely on wrapping behavior for PRNG and hash functions. Do not use signed integers in arithmetic that may overflow — use unsigned throughout the shader.

---

## 2. WGSL Shader Constraints

### SH-1: resolve_execute.wgsl is the Highest-Risk Shader — M3

This shader handles every combination of: input voxel type (8 types) × own intent outcome (6 action types) × up to 6 neighbor intents that may target this voxel. The total case space is large and the interactions are subtle (e.g., a protocell that declared MOVE but lost the bid at its target must not become EMPTY at its source position).

**Rule:** Before writing ANY branching logic in `resolve_execute.wgsl`, the agent MUST write a complete case enumeration as comments at the top of the shader. The enumeration must cover:

1. **For EMPTY input voxels:** What happens when 0, 1, or 2+ neighbors target this voxel with MOVE or REPLICATE intents. How bids are compared. What data is written (moved cell vs. replicated offspring vs. stays empty).

2. **For PROTOCELL input voxels:** What happens for each own intent (DIE, PREDATE, REPLICATE, MOVE, IDLE) × whether the intent succeeded or failed. Special attention to: MOVE declared but lost → cell stays put, energy still consumed? REPLICATE declared but no empty neighbor existed at intent time → what does the intent look like? PREDATE declared but prey was simultaneously predated by a higher-bid predator → fallback behavior.

3. **For WASTE, NUTRIENT, and other non-protocell types:** Confirm they ignore intents entirely and follow simple state transitions.

The case enumeration is part of the deliverable, not scaffolding to delete later. It remains as documentation for future modifications.

### SH-2: WGSL Alignment and Padding — M1

WGSL struct members follow specific alignment rules that can silently insert padding bytes. The key rules: `u32` aligns to 4 bytes, `vec2<u32>` aligns to 8 bytes, `vec3<u32>` aligns to 16 bytes (not 12), and `vec4<u32>` aligns to 16 bytes. A struct's total size is rounded up to a multiple of its largest member's alignment.

**The specific failure mode:** If the voxel buffer were declared as a WGSL struct:
```
struct Voxel {
    type_flags_energy: u32,  // offset 0
    age_species: u32,        // offset 4
    genome: array<u32, 4>,   // offset 8 — but arrays have element stride rules
    extra: vec2<u32>,        // offset 24? or 32? depends on array stride
}
```
The array stride and the `vec2` alignment interact in ways that depend on the WGSL implementation. If WGSL inserts padding after the array to align `vec2` to 8 bytes, the struct is 32 bytes. If it doesn't, it's still 32 bytes in this case — but change `extra` to `vec3<u32>` and the struct jumps to 48 bytes due to 16-byte alignment, with 12 bytes of invisible padding. The Rust `types::Voxel::pack()` would still produce 32 bytes, the GPU would read 48-byte strides, and every voxel after index 0 would read corrupt data from the wrong offset. No error, no warning — just a garbled simulation.

**Rule:** Declare the voxel buffer as `array<u32>` in WGSL, not as a struct. Access fields by computing the base offset (`index * 8`) and reading specific words. The `common.wgsl` helper functions (`voxel_get_type(idx)`, `voxel_get_energy(idx)`, etc.) encapsulate the offset math and provide named access without struct padding risk.

**Why not refactor to a struct later:** An agent may see the accessor functions and think "this would be cleaner as a struct." It would not. The accessor pattern exists specifically because WGSL struct layout is not guaranteed to match a packed `[u32; 8]` under all implementations. Do not replace the accessor functions with a struct. If this constraint is unclear, re-read the failure mode above.

### SH-3: No Dynamic Indexing of Workgroup Arrays — M7

WGSL allows `var<workgroup>` arrays, but dynamic indexing into them can be slow on some architectures (serialized access). The stats reduction shader (M7) uses shared memory for per-workgroup reduction.

**Rule:** In the stats reduction shader, use tree-structured reduction with power-of-two stride access patterns. Avoid scatter/gather patterns with data-dependent indices in shared memory. The species histogram (which requires hash table logic in shared memory) is the most sensitive — keep the hash table small (64 entries) and accept collisions.

### SH-4: WGSL Does Not Have `#include` — All Milestones

There is no standard mechanism for including one WGSL file in another.

**Rule:** The host (Rust side) performs string concatenation: `common_wgsl + "\n" + shader_wgsl` when creating each compute pipeline. `common.wgsl` MUST NOT contain any entry points (`@compute`, `@vertex`, `@fragment`). It contains only type definitions, constants, and helper functions. Every shader file after `common.wgsl` must be self-contained given the common prefix.

### SH-5: PRNG Quality Matters for Spatial Uniformity — M2

A weak PRNG (e.g., linear congruential) produces visible spatial patterns in nutrient spawning and mutation because adjacent voxels have adjacent seeds. PCG's output function is designed to destroy seed correlation, but only if the full hash is applied.

**Rule:** Use the full PCG-RXS-M-XS-32 hash, not a simplified version. The implementation is ~6 lines of WGSL. Do not substitute a cheaper hash to save cycles — the visual artifacts from a weak PRNG are immediately noticeable and will be mistaken for simulation bugs.

### SH-6: Floating-Point Determinism in WGSL — M2

WGSL does not guarantee strict IEEE 754 behavior for `fma`, `sqrt`, or transcendental functions. Different GPUs may produce different results for the same input. However, the simulation uses integer arithmetic for all mechanistic operations (energy, genome, intent encoding). Floating-point is used only for: temperature diffusion, temperature-to-modifier conversion, and rendering.

**Impact:** Temperature values may differ by ULP across GPUs. This is acceptable — temperature modulates behavior probabilistically (it adjusts a PRNG threshold), so ULP-level differences don't cause divergent simulation paths.

**Rule:** Never use floating-point for simulation state that feeds into determinism-critical comparisons. All bid values, energy values, genome values, and intent encodings must be integer. Temperature is the only float in the simulation path and its influence is filtered through integer PRNG thresholds.

---

## 3. Rust / WASM Constraints

### RS-1: WASM Binary Size — M1

The 5 MB gzipped budget (requirement P-7) is tight if heavy dependencies are pulled in. `wgpu` alone compiles to ~2 MB gzipped for the WebGPU backend. Adding a game engine or large math library will exceed the budget.

**Rule:** Use `glam` (tiny) for math, not `nalgebra` (large). Do not add `image`, `png`, or other media crates. Profile binary size with `twiggy` or `wasm-opt --print-size` after M1 to establish a baseline.

### RS-2: No Blocking in WASM Main Thread — All Milestones

WASM runs on the browser's main thread (unless using Web Workers). Any blocking call — including synchronous GPU readback — freezes the entire page.

**Rule:** All GPU buffer readback uses `mapAsync` with a callback or polled via `device.poll(Maintain::Poll)`. Never use `device.poll(Maintain::Wait)` in the WASM build. The host's `frame()` function must complete in < 4 ms CPU time (requirement FP-6), with all GPU work submitted asynchronously.

### RS-3: wasm-bindgen Supported Types — M1

`wasm_bindgen` only supports a subset of Rust types at the JS boundary: primitives, `String`, `JsValue`, and `Vec<u8>` / typed arrays. Complex structs must be serialized.

**Rule:** The `bridge.rs` API uses only `f32`, `u32`, `bool`, `String`, and `JsValue`. `SimStats` is converted to a `JsValue` (via `serde_json` → `JsValue::from_str` or manual object construction with `js_sys::Object`). Do not attempt to pass `types::Voxel` directly across the boundary — serialize to `JsValue` on the Rust side.

### RS-4: No std::time in WASM — M2

`std::time::Instant` is not available in WASM. Time comes from the browser.

**Rule:** Frame delta time (`dt`) is passed from JavaScript's `requestAnimationFrame` callback into the Rust `frame(dt)` function. The Rust side never calls any clock function. All timing is driven by the JS event loop.

### RS-5: Panic Behavior in WASM — All Milestones

Rust panics in WASM abort the module. There is no stack unwinding, no recovery. A panic during `frame()` kills the simulation permanently.

**Rule:** No `unwrap()` or `expect()` on fallible operations in the hot path (`frame()`, `tick()`, `render()`). Use `match` or `if let` with graceful error handling. Panics are acceptable only in `init()` for unrecoverable setup errors (e.g., WebGPU not available). The `types` crate's `pack`/`unpack` functions may use `debug_assert!` for invariant checks but must not panic in release builds.

### RS-6: Cargo Feature Flags for wgpu — M1

`wgpu` requires specific feature flags for WebGPU (as opposed to WebGL or native backends). The WASM target needs `webgpu` feature enabled and `webgl` disabled.

**Rule:** In `sim-core/Cargo.toml` and `renderer/Cargo.toml`:
```
[dependencies.wgpu]
version = "..."
default-features = false
features = ["wgsl"]
```
The `host` crate enables the backend feature:
```
[dependencies.wgpu]
version = "..."
default-features = false
features = ["webgpu", "wgsl"]
```
Do not enable `vulkan`, `metal`, `dx12`, or `webgl` features — they increase binary size and are unused.

---

## 4. GPU Memory and Performance Constraints

### GP-1: Memory Bandwidth is the Bottleneck — M2

At 128³ (2M voxels × 32 bytes = 64 MB per buffer), a single CA tick reads the input buffer (~64 MB) and writes the output buffer (~64 MB), plus reads neighbor data (~6× per voxel, though L2 cache handles most of this). Effective bandwidth demand: ~200–400 MB per tick.

An RTX 3060 has ~360 GB/s memory bandwidth. At 10 ticks/sec, bandwidth demand is ~2–4 GB/s — well under capacity. At 60 ticks/sec, demand rises to ~12–24 GB/s — still under capacity. The bottleneck shifts to dispatch overhead and pipeline stalls at very high tick rates.

Integrated GPUs (Intel UHD 630: ~25 GB/s bandwidth) can handle 64³ (~8 MB per buffer, ~50–100 MB per tick at 10 ticks/sec = ~0.5–1 GB/s). 128³ would require ~2–4 GB/s, which is feasible but tight.

**Rule:** Optimize for spatial locality. Access voxel data in linear index order (which maps to Z-order in the 3D grid). Avoid random-access patterns. The intent buffer's access pattern is inherently neighbor-indexed (reading 6 offsets per voxel), which has poor locality — this is unavoidable but bounded.

### GP-2: Dispatch Overhead — M3

Each `dispatchWorkgroups` call has fixed overhead on the CPU side (~5–50 µs depending on driver) and a pipeline drain/restart cost on the GPU. The 5-dispatch pipeline (commands, diffusion, intent, execute, stats) costs ~25–250 µs in dispatch overhead per tick.

**Rule:** Do not add dispatches unnecessarily. The 5-dispatch pipeline is the maximum. If a new feature requires GPU computation, merge it into an existing dispatch as an additional code path, not a new dispatch. Exception: M7's stats reduction is a new dispatch, but it runs at lower frequency (every Nth tick, not every tick) to amortize overhead.

### GP-3: Render Texture Update Frequency — M1

The `update_render_texture` compute pass converts voxel state to RGBA8 for ray marching. At 128³, this writes 2M × 4 bytes = 8 MB. Running it every frame at 60 FPS = 480 MB/s write bandwidth — acceptable, but wasteful when the simulation is paused.

**Rule:** Skip `update_render_texture` when: (1) the simulation is paused AND (2) no overlay mode change has occurred since the last update. Track a "render texture dirty" flag set by simulation ticks and overlay mode changes.

### GP-4: Atomic Operations Are Unavailable for Conflict Resolution — M3

WebGPU supports `atomicAdd`, `atomicMax`, etc. on `atomic<u32>` and `atomic<i32>` types in storage buffers. However, there is no 32-byte atomic compare-and-swap, which would be needed to atomically write a voxel.

**Impact:** This is why the architecture uses the intent/resolve pattern instead of atomic writes. Confirmed in architecture §5. Do not revisit.

**Rule:** No atomic operations in the simulation pipeline. If atomics are used anywhere (e.g., stats reduction counters), they must be `atomic<u32>` only, and the buffer must be declared with `atomic` type in WGSL.

### GP-5: 3D Texture Size Limits — M1

WebGPU `maxTextureDimension3D` minimum is 256. A 128³ texture is within limits. A 256³ texture is also within limits on most discrete GPUs. No risk for the primary target.

**Rule:** Query `device.limits().max_texture_dimension_3d` at startup. If < grid_size, fall back.

---

## 5. Simulation Logic Constraints

### SIM-1: Double-Buffer Write Isolation — M2

The cardinal rule of the CA: a voxel's tick-N output must depend only on tick-(N-1) state. A thread must never read from the buffer it is writing to.

**Rule:** Read from buffer A, write to buffer B (or vice versa). Bind groups enforce this — the read buffer is bound as `read` access, the write buffer as `read_write`. Verify bind group construction matches the tick parity. A violation produces "smearing" artifacts where one half of the grid sees partially updated state. This is a silent bug — no error, just wrong behavior.

**Test:** The M2 determinism test (run 100 ticks twice, compare checksums) catches this. If checksums diverge on the second run, suspect a double-buffer binding error.

### SIM-2: Intent Buffer Must Be Cleared Each Tick — M3

If the intent buffer is not zeroed before `intent_declaration`, stale intents from the previous tick will be read by `resolve_and_execute`. An EMPTY voxel might resolve a ghost intent from a protocell that no longer exists.

**Rule:** Clear `intent_buf` to zero at the start of each tick. Use `encoder.clear_buffer()` (WebGPU native clear, very fast — ~0.1 ms for 8 MB). Do not rely on the intent_declaration shader to overwrite all entries — EMPTY voxels write NO_ACTION (0), but if the buffer already contains 0, skipping the clear is only safe if EVERY voxel writes to its intent slot. Safer to always clear.

### SIM-3: PRNG Seed Must Differ Per Tick — M2

If the PRNG seed only depends on voxel index (not tick count), every tick produces the same random numbers for the same voxel. Nutrient spawning happens in the same locations every tick. Movement decisions repeat. The simulation becomes periodic.

**Rule:** Seed = `pcg_hash(voxel_index ^ (tick_count * 0x9E3779B9) ^ (grid_size * 0x85EBCA6B) ^ dispatch_salt)`. The tick count is passed as a uniform. It increments monotonically even across pause/resume. It does not reset to zero. The grid_size term prevents seed collisions when the same simulation logic runs at different grid resolutions (8³ test grids vs 128³ production) — without it, small grids have a higher probability of two voxels producing identical PRNG sequences for a tick. The `dispatch_salt` differs per shader pass (`0x1` for intent_declaration, `0x2` for resolve_execute, etc.) to ensure that two dispatches processing the same voxel in the same tick generate independent random sequences.

### SIM-4: Energy Underflow Protection — M2

Energy is a `u16` (0–65535). Subtracting metabolic cost from a low-energy protocell can underflow to a large value, making the protocell appear energy-rich instead of dead.

**Rule:** Use saturating subtraction: `new_energy = select(0u, energy - cost, energy >= cost)` in WGSL. Or equivalently: `if (cost >= energy) { die; } else { energy -= cost; }`. Never subtract without the guard.

### SIM-5: Species ID Hash Must Not Be Zero — M3

Species ID = 0 is reserved for non-protocell voxels. If the genome hash produces 0, a protocell would be visually indistinguishable from a non-protocell in the species tracking system.

**Rule:** After computing `species_id = hash16(genome)`, if the result is 0, set it to 1. One line. Do not skip this — a zero species ID will cause the stats system to undercount protocells.

### SIM-6: Temperature Clamping Prevents NaN — M5

Temperature is `f32`. Diffusion with extreme source values (0.0 or 1.0) and high diffusion rate can push intermediate values slightly outside [0.0, 1.0] due to floating-point accumulation. If temperature goes negative, `sqrt` or `log` operations downstream (if any are added) will produce NaN, which propagates.

**Rule:** Clamp temperature to [0.0, 1.0] after every diffusion step. Use `clamp(t_new, 0.0, 1.0)` in WGSL. Even though the current system uses only linear operations on temperature, the clamp prevents future NaN propagation.

### SIM-7: Mutation Must Not Write Reserved Genome Bytes — M3 (Softened in M6+)

Genome bytes 11–15 are reserved. Mutation can write random values to any byte including reserved ones (by design — requirement R-5 says "for each byte"). This is intentional: reserved bytes accumulate neutral mutations, which creates latent genetic diversity. When a reserved byte is later assigned a function (e.g., in a future milestone), the population already has variation in that byte.

**Rule:** Do NOT mask or skip reserved bytes during mutation. The genome interpretation functions simply ignore bytes 11–15 until they are assigned a role. This is correct behavior, not a bug.

### SIM-8: Movement and Replication Must Not Target Occupied Voxels — M3/M4

A protocell declares MOVE or REPLICATE targeting an adjacent EMPTY voxel. But between intent declaration and execution, there is no intermediate state change — intents are evaluated against tick-(N-1) state and executed atomically in the resolve pass. The "target was EMPTY at intent time" check is inherently correct because the intent declaration reads from the read buffer, which does not change during the tick.

The concern: two protocells both declare REPLICATE targeting the same EMPTY neighbor. Both intents reference a voxel that IS empty. The resolve pass handles this via bid comparison. This is not a bug — it's the intended conflict resolution mechanism.

**Rule:** Intent declaration checks for EMPTY in the read buffer and that is sufficient. Do not add a second check in the resolve pass — the resolve pass resolves conflicts, it does not revalidate preconditions. Adding a precondition check in resolve would cause both contenders to fail (both see "target is being claimed"), which is wrong.

---

## 6. Build and Deployment Constraints

### BD-1: wasm-pack Build Command — M1

The canonical build command is:
```
wasm-pack build crates/host --target web --release
```

This produces `pkg/` containing the `.wasm` binary and JS glue. The `--target web` flag generates ES module output suitable for `<script type="module">` loading.

Do NOT use `--target bundler` (requires webpack/rollup), `--target nodejs` (wrong platform), or `--target no-modules` (deprecated pattern).

### BD-2: Shader Source Embedding — M1

Shaders are embedded in the Rust binary via `include_str!()`:
```
const COMMON_WGSL: &str = include_str!("../../shaders/common.wgsl");
const RESOLVE_EXECUTE_WGSL: &str = include_str!("../../shaders/resolve_execute.wgsl");
```

The path is relative to the source file containing the `include_str!`. Since pipelines are created in `sim-core` and `renderer`, the path is `../../shaders/` (up from `crates/X/src/` to repo root, then into `shaders/`).

**Rule:** If the relative path is wrong, the build fails at compile time with a clear error. This is a feature — it guarantees no missing shader files at runtime.

### BD-3: No Dev Server Required — M1

The web page can be served by any static file server. `python3 -m http.server` in the repo root is sufficient for development. No bundler, no hot-reload, no dev server.

**Rule:** The `web/index.html` file loads the WASM module directly:
```html
<script type="module">
  import init from './pkg/host.js';
  await init();
</script>
```
Adjust the path to wherever `wasm-pack` outputs the `pkg/` directory. No additional build steps between `wasm-pack build` and opening the page.

### BD-4: Git LFS Not Required — All Milestones

There are no large binary assets. Shaders are text files. The WASM binary is a build artifact not committed to the repo.

**Rule:** Add `pkg/`, `target/`, and `*.wasm` to `.gitignore`.

---

## 7. Prohibited Patterns (Quick Reference)

These are patterns that agents tend to reach for that are specifically wrong for this project. Each references the constraint it violates.

| Pattern | Why It's Wrong | Ref |
|---------|---------------|-----|
| `device.poll(Maintain::Wait)` | Blocks WASM main thread, freezes page | RS-2 |
| `unwrap()` in `frame()` or `tick()` | Panic kills WASM module permanently | RS-5 |
| Struct with mixed types in WGSL voxel buffer | Alignment padding makes layout unpredictable | SH-2 |
| Atomic CAS for voxel writes | No 32-byte atomic exists; use intent/resolve | GP-4 |
| Reading from write buffer in CA update | Violates double-buffer isolation | SIM-1 |
| Writing a new dispatch for a small feature | Adds 5–50 µs overhead per tick | GP-2 |
| `nalgebra` for vector math | Binary size too large for WASM budget | RS-1 |
| `tokio` or `async-std` runtime | No async runtime needed, increases binary size | RS-1 |
| Raw u32 bit manipulation for voxel data | Must use `types::Voxel::pack/unpack` | proj-struct §8 |
| Skipping intent_buf clear between ticks | Ghost intents cause phantom replications | SIM-2 |
| Signed integer arithmetic in shaders | Overflow semantics differ from expectation | WG-8 |
| `std::time::Instant` in WASM | Not available; dt comes from JS | RS-4 |
| `energy - cost` without underflow guard | u16 wraps to 65535, protocell becomes immortal | SIM-4 |
| Species ID of zero for protocells | Zero reserved for non-protocells; stats undercount | SIM-5 |
| Writing resolve_execute.wgsl without case enumeration first | Highest-risk shader; cases must be enumerated before code | SH-1 |
| Modifying voxel field offsets in `common.wgsl` without updating `types` crate | Accessor functions in WGSL and pack/unpack in Rust must agree on which word/bit contains which field. Changing one without the other produces silent data corruption — every voxel reads the wrong fields. The roundtrip test catches this, but the rule is: never modify voxel field offsets in `common.wgsl` without simultaneously updating `types::Voxel::pack/unpack` and running the roundtrip test. | SH-2 |
| Refactoring `common.wgsl` accessor functions into a WGSL struct | The accessors exist because WGSL struct padding is unpredictable. A struct that "looks" like 32 bytes may have invisible padding that breaks the Rust-side layout. Do not replace accessors with a struct regardless of how "clean" it looks. | SH-2 |
