# Voxel Ecosystem Simulator

**You don't design life. You design the world. Life designs itself.**

Seed a barren voxel world with raw energy and primitive molecules. Set the conditions — heat, cold, light, barriers — and step back. Watch as protocells emerge, compete, adapt, and evolve through nothing but physics. No scripts. No AI directors. Every organism is a genuine product of selection pressure acting on random mutation in real time.

Build a sun and watch photosynthesizers colonize the light. Wall off a population and watch it diverge. Drop a toxin and watch the resistant survive. Introduce a heat source and watch cold-adapted species retreat, replaced by fast-burning, fast-mutating newcomers that didn't exist ten minutes ago.

Every ecosystem is unique. Every extinction is permanent. Every recovery is earned.

*You set the conditions. Chemistry does the rest.*

---

## How It Works

The simulation runs entirely on the GPU as a 3D cellular automaton. Each voxel is a 32-byte cell that can hold a protocell — a single-celled organism with a 16-byte genetic tape governing metabolism, replication, movement, predation, and mutation. There are no hand-authored behaviors. Protocells that are better adapted to local conditions replicate more, and their offspring inherit slightly mutated genomes. Over hundreds of ticks, populations diverge into distinct species competing for energy, space, and each other.

Your tools are environmental. Place energy sources, walls, nutrients, heat and cold sources. Apply toxins. Adjust temperature gradients. The organisms respond — or don't.

## Tech Stack

- **Simulation:** WGSL compute shaders on WebGPU
- **Host:** Rust compiled to WebAssembly via wasm-pack
- **Rendering:** GPU ray marching against a 3D voxel texture
- **UI:** Vanilla HTML/CSS/JS overlay
- **Target:** 128³ grid (2M voxels), 10+ simulation ticks/sec, 60 FPS render

## Build

```bash
wasm-pack build crates/host --target web --release
python3 -m http.server 8080
# Open http://localhost:8080/web/index.html in Chrome
```

Requires Chrome 113+ or Edge 113+ (WebGPU support).

## Documentation

The `docs/` directory contains the full specification package:

| Document | Purpose |
|----------|---------|
| `requirements.md` | RFC 2119 requirements — what the system must do |
| `architecture.md` | Technical decisions, data structures, GPU pipeline design |
| `milestones.md` | Vertical slices with acceptance criteria and dependency graph |
| `project-structure.md` | Crate/module/file layout annotated by milestone |
| `technical-constraints.md` | Platform limitations, prohibited patterns, gotchas |
| `test-strategy.md` | Test types, per-milestone test plans, pass/fail criteria |
| `agent-prompt.md` | Per-milestone implementation guides |

`CLAUDE.md` at the project root contains build commands, critical rules, and architecture quick reference.

## License

TBD
