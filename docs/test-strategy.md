# Test Strategy: Voxel Ecosystem Simulator

**Version:** 0.1.0-draft
**Status:** Draft — pending review before agent-prompt phase
**Prerequisites:** requirements.md, architecture.md, milestones.md, project-structure.md, technical-constraints.md

---

## 1. Testing Philosophy

GPU simulation code is hard to test because: (1) the simulation runs on a GPU and results require async readback, (2) visual correctness is often easier to assess than numerical correctness, and (3) non-determinism from floating-point imprecision or driver differences can mask real bugs.

The strategy addresses this with three test tiers:

**Tier 1 — Rust unit tests (`cargo test`):** Test the `types` crate exhaustively. Pack/unpack roundtrips, genome accessors, intent encoding, grid math. These run on CPU with no GPU. They are fast, deterministic, and catch data layout drift between Rust and WGSL. This tier is the foundation — if types are wrong, everything downstream is wrong.

**Tier 2 — GPU integration tests (`wasm-pack test --chrome --headless`):** Test simulation behavior by seeding a known initial state, running N ticks, reading back GPU buffer contents, and asserting properties. These require a browser with WebGPU support. They are slower (~1–5 seconds per test) and require headless Chrome. They test the actual shader code against real GPU hardware.

**Tier 3 — Visual smoke tests (manual, with optional screenshot regression):** Launch the application, exercise player tools, and visually verify emergent behavior. These cannot be fully automated but can be partially automated by capturing screenshots and comparing against baselines. Described per-milestone in the acceptance criteria.

---

## 2. Tier 1: Rust Unit Tests

### 2.1 types::Voxel — M1

| Test | Description | Pass Condition |
|------|-------------|----------------|
| `roundtrip_empty` | Construct EMPTY voxel, pack to `[u32; 8]`, unpack, compare all fields | Field equality |
| `roundtrip_protocell` | Construct PROTOCELL with known genome, energy, age, species_id. Pack, unpack, compare. | Field equality |
| `roundtrip_all_types` | For each VoxelType variant: construct, pack, unpack, verify type field survives | Type field matches for all 8 variants |
| `pack_energy_boundaries` | Pack voxels with energy = 0, 1, 65534, 65535. Unpack and verify. | u16 boundary values preserved |
| `pack_genome_all_bytes` | Set each genome byte (0–15) to a distinct value (byte N = N * 17). Pack, unpack, verify all 16 bytes. | All 16 bytes individually addressable |
| `species_id_deterministic` | Compute species_id for a known genome. Verify it matches a hardcoded expected value. | Hash is deterministic and non-zero for non-trivial genomes |
| `species_id_nonzero` | Construct genome that would hash to zero. Verify species_id is corrected to 1. | species_id ≠ 0 |
| `word_layout_matches_spec` | Pack a known voxel, examine raw `[u32; 8]` words directly. Verify: word 0 bits [0:7] = type, word 0 bits [16:31] = energy, word 1 bits [0:15] = age, etc. | Bit positions match architecture §2.1 |

### 2.2 types::Genome — M1

| Test | Description | Pass Condition |
|------|-------------|----------------|
| `accessor_byte_positions` | Set genome = `[0, 1, 2, ..., 15]`. Verify `metabolic_efficiency()` = 0, `metabolic_rate()` = 1, ..., `energy_split_ratio()` = 10. | Each accessor reads the correct byte index |
| `species_hash_stability` | Compute species hash for 10 known genomes. Compare against hardcoded expected values. | Hash output unchanged across code modifications. This test is a regression canary — if it fails, something changed in the hash function. |
| `species_hash_sensitivity` | Flip one bit in a genome. Verify species_id changes. | Single-bit change produces different hash (statistical — test 100 random genomes, require ≥ 90% produce different hashes) |

### 2.3 types::Intent — M3

| Test | Description | Pass Condition |
|------|-------------|----------------|
| `roundtrip_all_actions` | For each ActionType: encode with known direction and bid, decode, verify fields match. | All fields survive encode/decode |
| `bid_range` | Encode bid = 0, bid = (1 << 26) - 1 (max 26-bit). Decode. Verify values. | Boundary values preserved |
| `direction_all_values` | Encode each Direction variant (PosX..Self_), decode, verify. | All 7 direction values roundtrip |

### 2.4 types::SimParams — M1

| Test | Description | Pass Condition |
|------|-------------|----------------|
| `to_bytes_length` | Call `to_bytes()`. Verify length = expected byte count. | Length is a multiple of 4 (u32-aligned) and matches the uniform buffer size declared in architecture. |
| `to_bytes_deterministic` | Call `to_bytes()` twice on the same params. Verify identical output. | Byte-exact equality |

### 2.5 types::Grid — M1

| Test | Description | Pass Condition |
|------|-------------|----------------|
| `index_roundtrip` | For all (x, y, z) in a 4³ grid: verify `grid_coords(grid_index(x, y, z, 4), 4) == (x, y, z)` | All 64 coordinates roundtrip |
| `index_bounds` | `grid_index(127, 127, 127, 128)` = 128³ - 1 = 2097151 | Max index correct |
| `neighbor_offsets_count` | `neighbor_offsets().len()` = 6 | Von Neumann neighborhood |
| `neighbor_offsets_symmetry` | For each offset in `neighbor_offsets()`, verify the negation is also in the list. | ±X, ±Y, ±Z all present |

---

## 3. Tier 2: GPU Integration Tests

These tests run via `wasm-pack test --chrome --headless`. Each test:
1. Initializes a WebGPU device.
2. Allocates simulation buffers at a small grid size (8³ or 16³) to keep tests fast.
3. Seeds a known initial state using `types::Voxel::pack()`.
4. Runs N simulation ticks.
5. Reads back the output buffer via `mapAsync`.
6. Unpacks voxels using `types::Voxel::unpack()` and asserts properties.

**Grid size for tests:** 8³ (512 voxels, 16 KB per buffer). This is small enough that tests complete in < 1 second and the entire grid can be examined. Simulation behavior at 8³ is identical to 128³ — the CA rules are scale-independent.

**PRNG seed for tests:** Use tick_count = 0 with a fixed seed constant. Document the expected outcomes for this seed so tests are reproducible.

### 3.1 Metabolism and Death — M2

| Test | Description | Pass Condition |
|------|-------------|----------------|
| `protocell_energy_drain` | Seed one protocell with energy = 100, metabolic_rate = 10, no adjacent nutrients or energy sources. Run 1 tick. Readback. | Energy = 90 (100 - cost derived from metabolic_rate) |
| `protocell_death` | Seed one protocell with energy = 5, metabolic_rate high enough that cost > 5. Run 1 tick. | Voxel type = WASTE |
| `nutrient_consumption` | Seed one protocell adjacent to one nutrient. Run 1 tick. | Protocell energy increased. Nutrient concentration decreased. |
| `energy_source_gain` | Seed one protocell adjacent to one energy source. Run 1 tick. | Protocell energy increased by photosynthetic_rate-derived amount. |
| `waste_decay` | Seed one WASTE voxel with decay_countdown = 2. Run 2 ticks. | Tick 1: WASTE with countdown 1. Tick 2: EMPTY or NUTRIENT depending on config. |
| `determinism_100_ticks` | Seed a 16³ grid with 50 random protocells and nutrients. Run 100 ticks. Compute checksum of output buffer. Repeat with same seed. | Checksums match exactly. |
| `determinism_32_cubed_workgroup_boundary` | Same as above but at 32³ grid. This forces multiple workgroups (dispatch size 8×8×8), exercising halo loading, edge-of-workgroup neighbor reads, and barrier synchronization across workgroup boundaries. | Checksums match across two identical runs. |

### 3.2 Replication and Mutation — M3

| Test | Description | Pass Condition |
|------|-------------|----------------|
| `replication_basic` | Seed one protocell with energy >> replication_threshold, one adjacent EMPTY voxel. Run 1 tick. | Two protocells exist. Combined energy ≈ original (minus metabolism). |
| `replication_energy_split` | Same as above. Verify parent and offspring energy values. | Parent energy = original × split_ratio − metabolic_cost. Offspring energy = original × (1 − split_ratio). |
| `replication_blocked` | Seed one protocell with energy >> threshold, ALL 6 neighbors are WALLs. Run 1 tick. | Protocell remains, no replication. Energy retained (minus metabolism). |
| `mutation_occurs` | Seed one protocell with mutation_rate = 255 (100% mutation per byte). Run 1 tick (must replicate). Read offspring genome. | Offspring genome differs from parent in at least 1 byte (probabilistically: nearly all bytes differ). |
| `mutation_rate_zero` | Seed one protocell with mutation_rate = 0. Run 1 tick (must replicate). | Offspring genome identical to parent. |
| `conflict_resolution` | Seed two protocells (A with energy 200, B with energy 100) both adjacent to the same single EMPTY voxel. Run 1 tick. | Exactly one new protocell at the target. (Which one wins is PRNG-dependent but deterministic for the fixed seed — document the expected winner.) |
| `species_id_on_offspring` | After replication with mutation, verify offspring's species_id = hash of offspring's genome (not parent's). | species_id matches recomputed hash. |
| `intent_buffer_clear` | Run 2 ticks. On tick 2, place a new EMPTY region where protocells existed on tick 1. Verify no phantom replications from stale intents. | EMPTY voxels remain EMPTY if no protocell targets them on tick 2. |

### 3.3 Movement and Commands — M4

| Test | Description | Pass Condition |
|------|-------------|----------------|
| `movement_basic` | Seed one protocell with high movement_bias, surrounded by EMPTY on all sides. Run 1 tick. | Protocell has moved to an adjacent voxel. Original position is EMPTY. |
| `movement_energy_cost` | Seed protocell with known energy. Run 1 tick with movement. | Energy decreased by movement_energy_cost (global param) + metabolic cost. |
| `chemotaxis` | Seed one protocell with high chemotaxis_strength. Place nutrient in +X direction, EMPTY in all other directions. Run 10 ticks (PRNG-dependent). | Protocell has moved toward nutrient more often than away. (Statistical: over 10 independent runs with different seeds, net displacement in +X > 0 at least 7 times.) **On failure, log all 10 seeds and the tick number at which each failing seed first diverged from expected behavior.** See §7.4 for statistical test logging requirements. |
| `replication_priority` | Seed one protocell with energy >> replication threshold and high movement_bias. One adjacent EMPTY voxel. Run 1 tick. | Protocell replicates, does not move. Two cells exist. |
| `command_place_wall` | Send PLACE_WALL command at position (3, 3, 3). Run 1 tick. Readback position (3, 3, 3). | Voxel type = WALL. |
| `command_toxin` | Seed 10 protocells: 5 with toxin_resistance = 0, 5 with toxin_resistance = 255. Send toxin command covering all 10. Run 1 tick. | 5 low-resistance → WASTE. 5 high-resistance → PROTOCELL (alive). |

### 3.4 Temperature — M5

| Test | Description | Pass Condition |
|------|-------------|----------------|
| `diffusion_basic` | Set one voxel to temp 1.0, all others to 0.0. Run 10 diffusion ticks. Readback. | Source temperature decreased. Adjacent temperatures increased. Gradient radiates outward. |
| `wall_insulation` | Place WALL between heat source and observation point. Run 20 diffusion ticks. | Temperature on observation side is lower than it would be without the wall. (Compare against a control run without wall.) |
| `heat_source_anchoring` | Place HEAT_SOURCE with target_temp = 0.9. Run 100 diffusion ticks. | Temperature at heat source position = 0.9 (not diffused away). |
| `diffusion_stability` | Set diffusion_rate = 0.25 (maximum allowed). All voxels start at random temperatures. Run 1000 ticks. | No temperature value is outside [0.0, 1.0]. No NaN. Temperatures converge toward mean. |
| `temp_modulates_metabolism` | Seed identical protocells in hot (0.9) and cold (0.1) zones, equal nutrients. Run 100 ticks. | Hot-zone protocells have lower average energy (higher metabolic cost). |
| `temp_modulates_mutation` | Seed identical protocells in hot and cold zones. Run 500 ticks with replication. | Hot-zone population has higher species_id variance (more mutation). |

### 3.5 Predation — M6

| Test | Description | Pass Condition |
|------|-------------|----------------|
| `predation_basic` | Seed predator (predation_capability = 200, energy = 500) adjacent to prey (predation_capability = 0, energy = 100, which is below predator's aggression threshold). Run 1 tick. | Prey → WASTE. Predator energy increased by fraction of prey energy. |
| `predation_non_predator` | Seed protocell with predation_capability = 0 adjacent to low-energy protocell. Run 100 ticks. | No predation occurs. Both survive (energy permitting). |
| `predation_cancels_prey_movement` | Seed predator adjacent to prey. Prey has high movement_bias and an adjacent EMPTY voxel to flee to. Run 1 tick. | Prey is consumed at its current position. Prey does not escape. (Simplified predation model per architecture §5.) |
| `predation_priority_over_replication` | Seed predator with energy >> replication_threshold adjacent to both prey and an EMPTY voxel. Run 1 tick. | Predation occurs (prey → WASTE). Replication does not occur this tick (predation has higher priority). |
| `predation_conflict_two_predators` | Two predators (energy 300 and 200) adjacent to same prey. Run 1 tick. | One predator gains energy, the other does not. Prey → WASTE exactly once. (Which predator wins is PRNG-dependent; document expected outcome for test seed.) |
| `predation_prey_too_strong` | Seed predator with predation_aggression = 50 (low threshold) adjacent to protocell with energy = 200. Run 1 tick. | No predation — prey energy exceeds aggression threshold. |

### 3.6 Statistics — M7

| Test | Description | Pass Condition |
|------|-------------|----------------|
| `stats_population_count` | Seed 25 protocells in 8³ grid. Run 0 ticks (just reduce). Readback stats. | Population count = 25. |
| `stats_total_energy` | Seed 10 protocells each with energy = 100. Readback stats. | Total energy = 1000. |
| `stats_species_count` | Seed 5 protocells with genome A, 5 with genome B (different species_id). Readback stats. | Species count = 2. |

---

## 4. Tier 3: Visual Smoke Tests

These are manual tests performed by the developer after each milestone. They cannot be fully automated but serve as the final validation that the simulation looks and feels correct.

### M1 Smoke Test
- Page loads, colored voxels visible.
- Orbit, zoom, pan all work smoothly.
- Cross-section reveals interior voxels.
- All voxel types have distinct colors.

### M2 Smoke Test
- Protocells near energy sources glow steadily.
- Protocells far from energy sources dim over ~10 seconds and disappear (→ WASTE → decay).
- Nutrient field visibly shrinks around protocell clusters.
- Pause/resume works. Camera works during pause.

### M3 Smoke Test
- Population expands outward from seed cluster.
- After ~200 ticks, multiple colors (species) visible.
- Population growth slows as nutrients deplete.
- No visually obvious grid artifacts (striping, checkerboard patterns from double-buffer errors).

### M4 Smoke Test
- Protocells visibly migrate toward placed nutrient clusters.
- Walls block movement and replication.
- Newly placed energy sources attract colonization.
- Toxin creates visible die-off zones.

### M5 Smoke Test
- Temperature overlay shows gradient radiating from heat/cold sources.
- Walls create sharp temperature boundaries.
- Protocells near heat sources appear more diverse (more colors) than cold-zone populations.

### M6 Smoke Test
- Predator clusters visibly expand into prey clusters.
- After prey is depleted in a region, predator population crashes.
- Over 1000+ ticks, population graph (if available) shows oscillation.

### M7 Smoke Test
- Stats panel shows live population and species count.
- Population graph shows smooth curves, not noise.
- Voxel inspector shows correct data for clicked voxel.
- Parameter sliders change simulation behavior in real time.
- Overlay modes are visually distinct.

### M8 Smoke Test
- Application loads on integrated GPU at 64³ without errors.
- Loading screen displays during init.
- All three presets produce distinct ecosystem dynamics.
- 10-minute continuous run with no console errors.

---

## 5. Determinism Testing Protocol

Determinism is the foundation of test reliability. If the simulation is not deterministic, Tier 2 tests become flaky and the test suite is worthless.

**How determinism is tested:** Run the simulation for N ticks with a fixed initial state and fixed `tick_count` starting value. Compute a checksum (CRC32 or SHA-256) of the entire output voxel buffer. Repeat. Checksums must be identical.

**When determinism tests run:** After every milestone. The M2 determinism test (100 ticks) and M3 determinism test (200 ticks) are the two primary regression tests. If either fails after any code change, the change must be reverted and debugged before proceeding.

**What breaks determinism:**
- Branch-dependent PRNG advance count (constraint SIM-3 / architecture §9.3 prevents this)
- Floating-point operations in the simulation critical path (constraint SH-6 prevents this)
- Reading from the wrong buffer (double-buffer swap error — constraint SIM-1)
- Workgroup execution order dependence (constraint WG-6)
- Uninitialized memory reads (intent buffer not cleared — constraint SIM-2)

**If determinism breaks:** Binary search ticks. Run 1 tick — deterministic? Run 10 ticks? Run 50? Find the first tick where checksums diverge. Then examine what happens at that tick: which voxels differ, and what operation produces the difference. The `types::Voxel::unpack()` function is essential for this debugging — it converts the raw buffer into inspectable field values.

---

## 6. Regression Test Suite

The regression suite is the set of Tier 1 + Tier 2 tests that must pass before any milestone is considered complete. It grows monotonically — tests are never removed.

| Milestone | New Tests Added | Cumulative Test Count (approx) |
|-----------|-----------------|-------------------------------|
| M1 | types roundtrips, grid math, params | ~20 |
| M2 | metabolism, death, waste decay, determinism | ~30 |
| M3 | replication, mutation, conflict, intent roundtrip, determinism 200-tick | ~42 |
| M4 | movement, chemotaxis, commands, toxin | ~50 |
| M5 | diffusion, insulation, temp modulation | ~58 |
| M6 | predation (6 tests), full regression | ~65 |
| M7 | stats accuracy, picking | ~70 |
| M8 | benchmark harness (pass/fail on perf targets), tier detection | ~75 |

**Runtime estimate:** Tier 1 (CPU): < 1 second total. Tier 2 (GPU): ~30–60 seconds total (each test ~0.5–2 seconds with WebGPU init overhead).

---

## 7. Test Infrastructure Requirements

### 7.1 GPU Test Harness

A shared test utility (in `sim-core/tests/` or a dedicated `test-harness/` module) that provides:

```
TestHarness::new(grid_size: u32) → TestHarness
    Initializes WebGPU device and creates a SimEngine at the given grid size.

TestHarness::seed_voxels(voxels: &[(u32, u32, u32, Voxel)]) → ()
    Writes specific voxels at specific coordinates. All other voxels are EMPTY.

TestHarness::set_params(params: SimParams) → ()
    Sets simulation parameters.

TestHarness::tick(n: u32) → ()
    Runs n simulation ticks.

TestHarness::read_voxel(x: u32, y: u32, z: u32) → Voxel
    Reads back a single voxel (blocking readback — acceptable in test code).

TestHarness::read_all() → Vec<u8>
    Reads back the entire voxel buffer as raw bytes.

TestHarness::checksum() → u32
    CRC32 of the full output buffer.
```

This harness is created in M2 (when the simulation loop exists) and used by all subsequent milestones. It lives in test code only — not compiled into the release binary.

### 7.2 Blocking Readback in Tests

Production code must never use blocking GPU readback (constraint RS-2). Test code MAY use `device.poll(Maintain::Wait)` to synchronously read buffer contents, because tests are not running in a browser event loop — they run in headless Chrome's WASM environment where blocking is acceptable.

**Rule:** `Maintain::Wait` is permitted ONLY in files under `tests/` directories. A `Maintain::Wait` call in `src/` is always a bug.

### 7.3 Test Seed Documentation

Every Tier 2 test that depends on PRNG outcomes must document the expected outcome for the test seed. For example:

```
// Test: conflict_resolution
// Seed: tick_count = 0, grid_size = 8
// Protocell A at (3,3,3) energy=200, Protocell B at (3,3,5) energy=100
// Both target EMPTY at (3,3,4)
// Expected: A wins (bid A = 147 > bid B = 62 for this PRNG seed)
// If PRNG implementation changes, update this expected outcome.
```

This documentation is part of the test code, not a separate document. When the PRNG implementation changes (it shouldn't, but if it does), these comments tell the developer exactly what to update.

### 7.4 Statistical Test Failure Logging

Any test that uses multiple PRNG seeds and a statistical pass/fail threshold (e.g., "7 out of 10 runs must show positive displacement") MUST log the following on failure:

1. Every seed used (not just the count).
2. For each failing seed: the tick number at which behavior first diverged from expectation.
3. The aggregate result (e.g., "4/10 passed: seeds 3, 5, 7, 9 failed at ticks 45, 12, 89, 34").

This enables targeted debugging: the developer can rerun seed 5 at tick 12 deterministically and inspect the exact voxel state that produced the unexpected outcome. Without per-seed logging, the developer must rerun all seeds and guess which ones failed — an unacceptable debugging cost for a test that will occasionally fail during rule parameter tuning.

Statistical tests SHOULD also log the actual measured value for each seed (e.g., net displacement = -2 for seed 5) so that near-misses (displacement = -1) can be distinguished from total failures (displacement = -8).

---

## 8. What NOT to Test

| Don't Test | Why |
|------------|-----|
| Exact pixel colors in the renderer | Rendering is visual; colors are derived from species hashes and energy which vary by run. Test the render texture compute pass by verifying non-zero alpha for occupied voxels, not exact RGB values. This prohibition is permanent across all milestones. Do not add "protocells of species X should be blue-ish" tests even if it seems useful — color derives from genome hash (tested in types), genome changes with mutation (stochastic by design), and any color assertion is either redundant with the hash unit test or testing a non-deterministic evolutionary outcome. The occupancy alpha check is the ceiling for render validation. |
| Performance in Tier 2 tests | 8³ grid tests don't reflect 128³ performance. Performance testing belongs in M8's benchmark harness at full grid size. |
| Browser UI behavior | `ui.js` is thin DOM manipulation. Testing it requires a DOM testing framework (Cypress, Playwright) that adds complexity disproportionate to the UI's simplicity. Visual smoke tests cover this. |
| WGSL shader syntax | If the shader has a syntax error, `device.createComputePipeline()` fails and every GPU test fails immediately. No separate shader lint step needed. |
| Floating-point exact equality for temperature | Temperature uses f32 and may have ULP-level variance. Test temperature with epsilon tolerance (1e-5), not exact equality. |
