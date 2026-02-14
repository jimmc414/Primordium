# Requirements: Voxel Ecosystem Simulator

**Version:** 0.1.0-draft
**Status:** Draft — pending review before architecture phase
**RFC 2119:** Key words MUST, MUST NOT, SHOULD, SHOULD NOT, MAY are used per RFC 2119.

---

## 1. Project Scope

A browser-based 3D cellular automata simulator where single-voxel protocells with small genetic tapes compete for energy and resources in a voxel world. Players manipulate environmental conditions; organisms emerge, adapt, and die through CA rule dynamics. No organisms are player-designed.

### 1.1 Non-Goals

These are explicitly out of scope for all milestones:

- Multi-voxel organisms (organisms spanning more than one voxel)
- Networking, multiplayer, or shared simulation state
- Procedural terrain generation beyond simple geometric primitives
- Save/load of simulation state to persistent storage (MAY be added as extension)
- Mobile platform support
- Audio

---

## 2. Platform Requirements

| ID | Requirement |
|----|-------------|
| P-1 | The application MUST run in browsers supporting the WebGPU API (Chrome 113+, Edge 113+). |
| P-2 | The host-side runtime MUST be compiled from Rust to WebAssembly via `wasm-pack` or equivalent toolchain. |
| P-3 | GPU compute shaders MUST be authored in WGSL. |
| P-4 | The application MUST NOT require any browser plugins, extensions, or flags beyond enabling WebGPU where necessary. |
| P-5 | The application SHOULD degrade gracefully with an informative error message on browsers lacking WebGPU support. |
| P-6 | The application MUST NOT assume a discrete GPU. It MUST run on integrated GPUs (Intel UHD 630 class or newer) at reduced grid resolution. |
| P-7 | The total WASM binary size MUST NOT exceed 5 MB gzipped. |

---

## 3. Performance Targets

### 3.1 Primary Target: 128³ Grid

| ID | Requirement |
|----|-------------|
| FP-1 | The simulation MUST sustain ≥ 10 simulation ticks per second on a discrete GPU (RTX 3060 class). |
| FP-2 | The simulation SHOULD sustain ≥ 20 simulation ticks per second on a discrete GPU under typical load (< 30% voxel occupancy). |
| FP-3 | The render loop MUST maintain ≥ 60 FPS independent of simulation tick rate. |
| FP-4 | Simulation tick rate and render frame rate MUST be fully decoupled. Rendering MUST interpolate or display the most recent completed simulation state. |
| FP-5 | Total GPU memory usage for the 128³ grid MUST NOT exceed 160 MB. Budget: double-buffered voxel state (~128 MB), double-buffered temperature field (~16 MB), rendering resources (~16 MB). |
| FP-6 | Per-frame CPU-side work (WASM host) MUST NOT exceed 4 ms. |

### 3.2 Stretch Target: 256³ Grid (Sparse)

| ID | Requirement |
|----|-------------|
| FS-1 | A sparse representation SHOULD support a 256³ logical grid while allocating storage only for occupied voxels and their immediate neighborhoods. |
| FS-2 | The sparse grid SHOULD sustain ≥ 10 simulation ticks per second at ≤ 10% occupancy on discrete GPU. |
| FS-3 | The sparse grid MUST NOT be attempted until the dense 128³ implementation passes all acceptance criteria. |

### 3.3 Intermediate Fallback: 96³ Grid

| ID | Requirement |
|----|-------------|
| FM-1 | On discrete GPUs with insufficient VRAM for 128³ (buffer allocation failure or maxStorageBufferBindingSize < 64 MB), the application SHOULD fall back to a 96³ grid. |
| FM-2 | At 96³, the simulation SHOULD sustain ≥ 10 ticks/second and ≥ 60 FPS render on discrete GPU. |

### 3.4 Integrated GPU Fallback

| ID | Requirement |
|----|-------------|
| FI-1 | On integrated GPUs, the application MUST automatically reduce grid resolution to 64³. |
| FI-2 | At 64³, the simulation MUST sustain ≥ 10 ticks/second and ≥ 30 FPS render. |

---

## 4. Voxel State

| ID | Requirement |
|----|-------------|
| V-1 | Each voxel MUST occupy exactly 32 bytes of GPU buffer storage. No padding variance between platforms. |
| V-2 | Each voxel MUST encode: voxel type, energy level, genome data, species identifier, and age. |
| V-3 | Voxel type MUST distinguish at minimum: EMPTY, WALL, NUTRIENT, ENERGY_SOURCE, PROTOCELL, WASTE, HEAT_SOURCE, COLD_SOURCE. |
| V-4 | Protocell voxels MUST carry a genome of 8–16 bytes interpreted as an instruction/parameter tape. |
| V-5 | The genome MUST encode at minimum: metabolic efficiency, replication threshold, replication mutation rate, movement bias, chemotaxis strength, toxin resistance, predation capability, and predation aggression threshold. These MAY be encoded as raw bytes interpreted by the CA rules rather than named fields. |
| V-6 | The species identifier MUST be derived deterministically from genome content (hash) so that identical genomes share a species ID. |
| V-7 | Non-protocell voxels MUST use the genome bytes for type-specific state (e.g., nutrient concentration, energy output level). |

---

## 5. Simulation Rules

### 5.1 Core CA Update

| ID | Requirement |
|----|-------------|
| S-1 | The CA MUST use a 3D von Neumann neighborhood (6 face-adjacent neighbors). |
| S-2 | Each simulation tick MUST process all voxels exactly once via a double-buffered read/write pattern. No voxel reads its own tick's writes. |
| S-3 | Update rules MUST be deterministic given identical state and identical PRNG seed. |
| S-4 | The simulation MUST use a GPU-side PRNG seeded per-voxel per-tick to provide stochastic behavior (mutation, movement, consumption probability). |

### 5.2 Metabolism

| ID | Requirement |
|----|-------------|
| M-1 | Protocells MUST consume energy each tick proportional to their genome-encoded metabolic rate. |
| M-2 | A protocell whose energy reaches zero MUST die: its voxel converts to WASTE. |
| M-3 | Protocells adjacent to NUTRIENT voxels MUST attempt to consume them. Consumption probability MUST be a function of the protocell's genome-encoded metabolic efficiency. |
| M-4 | Protocells adjacent to ENERGY_SOURCE voxels MUST gain energy proportional to genome-encoded photosynthetic capability (or equivalent parameter). |
| M-5 | WASTE voxels SHOULD decay to EMPTY over a configurable number of ticks. |
| M-6 | WASTE voxels MAY convert to NUTRIENT at a configurable rate (nutrient recycling). |

### 5.3 Replication

| ID | Requirement |
|----|-------------|
| R-1 | A protocell MUST attempt replication when its energy exceeds its genome-encoded replication threshold. |
| R-2 | Replication MUST target a random adjacent EMPTY voxel. If no adjacent EMPTY voxel exists, replication MUST fail (no action, energy retained). |
| R-3 | On successful replication, the parent's energy MUST be split between parent and offspring. The split ratio SHOULD be 50/50 but MAY be genome-encoded. |
| R-4 | The offspring genome MUST be a copy of the parent genome with per-byte mutation applied at the genome-encoded mutation rate. |
| R-5 | Mutation MUST operate as: for each byte, with probability = mutation_rate, replace the byte with a uniformly random value. |

### 5.4 Movement

| ID | Requirement |
|----|-------------|
| MV-1 | Each tick, a protocell SHOULD attempt to move to an adjacent EMPTY voxel with probability proportional to its genome-encoded movement bias. |
| MV-2 | Movement MUST cost energy. The cost MUST be configurable as a global simulation parameter. |
| MV-3 | Movement direction SHOULD be biased toward adjacent NUTRIENT or ENERGY_SOURCE voxels when present (chemotaxis). The strength of this bias MUST be genome-encoded. |

### 5.5 Competition

| ID | Requirement |
|----|-------------|
| C-1 | When two protocells simultaneously target the same EMPTY voxel (for replication or movement), the conflict MUST be resolved probabilistically: each contender's win probability MUST be proportional to its energy relative to the sum of all contenders' energy. Ties (equal energy) MUST be broken by PRNG. |
| C-2 | Protocells MUST implement a predation mechanism: a protocell adjacent to another protocell MAY consume it, converting the prey to WASTE and gaining a configurable fraction of its energy. Predation capability and predation target selection MUST be genome-encoded. Predation success probability MUST be energy-weighted (attacker energy vs. defender energy). |

### 5.6 Resource Dynamics

| ID | Requirement |
|----|-------------|
| RD-1 | NUTRIENT voxels MUST spawn at a configurable global rate in EMPTY voxels, with spatial distribution controlled by a configurable noise function or uniform random placement. |
| RD-2 | ENERGY_SOURCE voxels MUST be persistent unless explicitly removed by player action. |
| RD-3 | The total energy in the system (sum of all protocell energy + nutrient energy + energy source output) SHOULD trend toward a configurable equilibrium to prevent unbounded population growth or extinction spirals. |

### 5.7 Spatial Environment Gradients

| ID | Requirement |
|----|-------------|
| SG-1 | Temperature MUST be a per-voxel scalar field, not a global uniform. |
| SG-2 | The temperature field MUST diffuse each tick: each voxel's temperature MUST trend toward the average of its neighbors at a configurable diffusion rate. |
| SG-3 | ENERGY_SOURCE voxels MUST emit heat, raising the temperature of surrounding voxels. |
| SG-4 | WALL voxels MUST act as thermal insulators (no diffusion across walls). |
| SG-5 | The player MUST be able to place persistent hot and cold sources that anchor temperature gradients. |
| SG-6 | Local temperature MUST modulate protocell behavior: higher temperature MUST increase mutation rate and metabolic cost; lower temperature MUST decrease both. The modulation function MUST be configurable. |
| SG-7 | The temperature diffusion pass MUST run as a separate compute shader dispatch, not interleaved with the CA update pass. |
| SG-8 | The temperature field MUST be stored in a separate GPU buffer from the voxel state buffer to allow independent update scheduling. |

---

## 6. Player Interaction

### 6.1 Environmental Tools

| ID | Requirement |
|----|-------------|
| IT-1 | The player MUST be able to place and remove ENERGY_SOURCE voxels. |
| IT-2 | The player MUST be able to place and remove WALL voxels. |
| IT-3 | The player MUST be able to introduce NUTRIENT voxels in a configurable brush radius. |
| IT-4 | The player MUST be able to introduce a toxin effect: all protocells within a radius whose genome-encoded toxin resistance is below a threshold MUST die (convert to WASTE). |
| IT-5 | The player MUST be able to place and remove persistent heat sources and cold sources that create spatial temperature gradients per SG-5. |
| IT-6 | The player SHOULD be able to seed initial protocells with random genomes at a specified location and radius. |

### 6.2 Observation Tools

| ID | Requirement |
|----|-------------|
| IO-1 | The player MUST be able to pause, resume, and single-step the simulation. |
| IO-2 | The player MUST be able to adjust simulation tick rate (1–60 ticks/second). |
| IO-3 | The player MUST be able to inspect any voxel to see its full state (type, energy, genome, species ID, age). |
| IO-4 | The application MUST display real-time population count, species count, and average energy. |
| IO-5 | The application SHOULD display a population-over-time graph for the top N species by count. |
| IO-6 | The application SHOULD provide a heatmap overlay mode showing energy density, population density, temperature, or species diversity per region. |

### 6.3 Camera and Navigation

| ID | Requirement |
|----|-------------|
| IC-1 | The player MUST be able to orbit, zoom, and pan a 3D camera around the simulation grid. |
| IC-2 | The application MUST support a cross-section/slice view that reveals the interior of the grid along any axis. |
| IC-3 | Camera controls MUST remain responsive (≥ 60 FPS) regardless of simulation load. |

---

## 7. Rendering

| ID | Requirement |
|----|-------------|
| RN-1 | Rendering MUST operate on the most recently completed simulation state. It MUST NOT block or delay simulation ticks. |
| RN-2 | The renderer MUST display occupied voxels only. EMPTY voxels MUST NOT consume draw calls or fragment shader time. |
| RN-3 | Protocell voxels MUST be visually distinguishable by species (color derived from species ID hash). |
| RN-4 | Protocell voxels SHOULD encode energy level as brightness or opacity variation. |
| RN-5 | WALL, NUTRIENT, ENERGY_SOURCE, WASTE, HEAT_SOURCE, and COLD_SOURCE voxels MUST each have a distinct visual appearance. |
| RN-6 | The renderer SHOULD support a ray-marching approach against a 3D texture for the dense 128³ grid. Alternative rendering strategies MAY be used if they meet FP-3 performance requirements. |
| RN-7 | The application MUST render a bounding wireframe around the simulation volume. |

---

## 8. Configuration and Extensibility

| ID | Requirement |
|----|-------------|
| E-1 | All simulation parameters referenced as "configurable" in this document MUST be adjustable at runtime via the UI without restarting the simulation. |
| E-2 | Simulation parameters MUST be exposed as a flat key-value structure suitable for serialization to JSON. |
| E-3 | The CA rule system SHOULD be structured such that adding a new voxel type requires changes to at most: the voxel type enum, the compute shader update function, and the renderer color map. |
| E-4 | The genome interpretation logic MUST be isolated in a single compute shader function so that genome encoding changes do not require modifications to replication, mutation, or movement logic. |
| E-5 | The application MAY support importing/exporting simulation parameters as JSON files. |

---

## 9. Acceptance Criteria Summary

The requirements are satisfied when:

1. A 128³ simulation runs in Chrome with protocells that metabolize, replicate with mutation, move, compete, predate, and die — with no scripted behavior.
2. Distinct species (identifiable by color) emerge and persist across hundreds of ticks without player intervention after initial seeding.
3. Spatial temperature gradients produce visible niche differentiation: protocells near heat sources exhibit measurably different genome distributions than those in cold regions.
4. Player environmental tools visibly alter ecosystem dynamics (e.g., adding energy sources causes local population growth; toxins cause selective die-off; temperature changes shift species distributions).
5. Predation interactions are observable: predator species visibly consume prey species, and predator/prey population dynamics show coupled oscillation over time.
6. Simulation ticks at ≥ 10/sec and render at ≥ 60 FPS on target hardware.
7. A voxel inspector shows correct genome and state data for any clicked voxel.
