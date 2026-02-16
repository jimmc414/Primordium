// ============================================================
// stats_reduction.wgsl — M7: Single-stage reduction with global atomics.
// Counts population, total energy, max energy, and species histogram.
// Prepended with common.wgsl at pipeline creation.
//
// Bind group 0:
//   [0] voxel_buf: storage<array<u32>, read>
//   [1] stats_buf: storage<array<atomic<u32>>, read_write>
//   [2] params: uniform<SimParams>
//
// Stats buffer layout (32 × u32):
//   [0] population
//   [1] total_energy
//   [2] species_count (unused)
//   [3] max_energy
//   [4..27] species histogram: 12 entries × 2 words (species_id, count)
//   [28..31] reserved
// ============================================================

struct SimParams {
    grid_size: f32,
    tick_count: f32,
    dt: f32,
    nutrient_spawn_rate: f32,
    waste_decay_ticks: f32,
    nutrient_recycle_rate: f32,
    movement_energy_cost: f32,
    base_ambient_temp: f32,
    metabolic_cost_base: f32,
    replication_energy_min: f32,
    energy_from_nutrient: f32,
    energy_from_source: f32,
    diffusion_rate: f32,
    temp_sensitivity: f32,
    predation_energy_fraction: f32,
    max_energy: f32,
    overlay_mode: f32,
    sparse_mode: f32,
    brick_grid_dim: f32,
    max_bricks: f32,
};

@group(0) @binding(0) var<storage, read> voxel_buf: array<u32>;
@group(0) @binding(1) var<storage, read_write> stats_buf: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> params: SimParams;

var<workgroup> wg_pop: atomic<u32>;
var<workgroup> wg_energy: atomic<u32>;
var<workgroup> wg_max_energy: atomic<u32>;
var<workgroup> wg_species_id: array<atomic<u32>, 16>;
var<workgroup> wg_species_count: array<atomic<u32>, 16>;

@compute @workgroup_size(64, 1, 1)
fn stats_reduction_main(@builtin(global_invocation_id) gid: vec3<u32>,
                         @builtin(local_invocation_id) lid: vec3<u32>) {
    let gs = u32(params.grid_size);
    var total_voxels: u32;
    if params.sparse_mode > 0.0 {
        total_voxels = u32(params.max_bricks) * 512u;
    } else {
        total_voxels = gs * gs * gs;
    }
    let num_workgroups = (total_voxels + 63u) / 64u;
    let total_threads = num_workgroups * 64u;

    // Initialize shared memory
    if lid.x == 0u {
        atomicStore(&wg_pop, 0u);
        atomicStore(&wg_energy, 0u);
        atomicStore(&wg_max_energy, 0u);
    }
    if lid.x < 16u {
        atomicStore(&wg_species_id[lid.x], 0u);
        atomicStore(&wg_species_count[lid.x], 0u);
    }
    workgroupBarrier();

    // Grid stride loop: each thread accumulates locally
    var local_pop = 0u;
    var local_energy = 0u;
    var local_max_energy = 0u;

    var vi = gid.x;
    loop {
        if vi >= total_voxels { break; }

        let base = vi * VOXEL_STRIDE;
        let word0 = voxel_buf[base];
        let vtype = word0 & 0xFFu;

        if vtype == VOXEL_PROTOCELL {
            local_pop += 1u;
            let energy = (word0 >> 16u) & 0xFFFFu;
            local_energy += energy;
            local_max_energy = max(local_max_energy, energy);

            // Species tracking via open-addressing hash in shared memory
            let word1 = voxel_buf[base + 1u];
            let species_id = (word1 >> 16u) & 0xFFFFu;
            if species_id != 0u {
                let hash_start = species_id % 16u;
                for (var probe = 0u; probe < 16u; probe += 1u) {
                    let slot = (hash_start + probe) % 16u;
                    let prev = atomicCompareExchangeWeak(&wg_species_id[slot], 0u, species_id);
                    if prev.exchanged || prev.old_value == species_id {
                        atomicAdd(&wg_species_count[slot], 1u);
                        break;
                    }
                }
            }
        }

        vi += total_threads;
    }

    // Reduce local counts into workgroup shared memory
    atomicAdd(&wg_pop, local_pop);
    atomicAdd(&wg_energy, local_energy);
    atomicMax(&wg_max_energy, local_max_energy);
    workgroupBarrier();

    // Thread 0 of each workgroup atomically adds to global stats_buf
    if lid.x == 0u {
        atomicAdd(&stats_buf[0], atomicLoad(&wg_pop));
        atomicAdd(&stats_buf[1], atomicLoad(&wg_energy));
        atomicMax(&stats_buf[3], atomicLoad(&wg_max_energy));

        // Merge workgroup species table into global 12-entry table
        for (var s = 0u; s < 16u; s += 1u) {
            let sid = atomicLoad(&wg_species_id[s]);
            let cnt = atomicLoad(&wg_species_count[s]);
            if sid == 0u || cnt == 0u { continue; }

            let ghash = sid % 12u;
            for (var gp = 0u; gp < 12u; gp += 1u) {
                let gslot = (ghash + gp) % 12u;
                let goffset = 4u + gslot * 2u;
                let prev = atomicCompareExchangeWeak(&stats_buf[goffset], 0u, sid);
                if prev.exchanged || prev.old_value == sid {
                    atomicAdd(&stats_buf[goffset + 1u], cnt);
                    break;
                }
            }
        }
    }
}
