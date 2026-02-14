// ============================================================
// intent_declaration.wgsl — M3: Intent declaration pass.
// Each protocell declares one intent (DIE, REPLICATE, or IDLE).
// Prepended with common.wgsl at pipeline creation.
//
// Bind group 0:
//   [0] voxel_read:  storage<array<u32>, read>
//   [1] intent_buf:  storage<array<u32>, read_write>
//   [2] params:      uniform<SimParams>
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
};

@group(0) @binding(0) var<storage, read> voxel_read: array<u32>;
@group(0) @binding(1) var<storage, read_write> intent_buf: array<u32>;
@group(0) @binding(2) var<uniform> params: SimParams;

@compute @workgroup_size(4, 4, 4)
fn intent_declaration_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let gs = u32(params.grid_size);
    if gid.x >= gs || gid.y >= gs || gid.z >= gs {
        return;
    }

    let idx = grid_index(gid, gs);
    let vtype = voxel_get_type(&voxel_read, idx);

    // Non-protocells: write NO_ACTION and return
    if vtype != VOXEL_PROTOCELL {
        intent_buf[idx] = 0u;
        return;
    }

    // ---- Protocell intent declaration ----
    // PRNG with dispatch salt 0x1 for intent pass
    var rng = prng_seed(idx, u32(params.tick_count), gs, 0x1u);

    let energy = voxel_get_energy(&voxel_read, idx);

    // Exactly 5 PRNG advances per protocell, always consumed regardless of branch
    let roll_movement_decision = pcg_next(&rng);   // advance 1 (unused in M3)
    let roll_movement_direction = pcg_next(&rng);   // advance 2 (unused in M3)
    let roll_predation_target = pcg_next(&rng);     // advance 3 (unused in M3)
    let roll_replication_target = pcg_next(&rng);    // advance 4
    let roll_bid = pcg_next(&rng);                   // advance 5

    // Priority 1: DIE — energy == 0
    if energy == 0u {
        intent_buf[idx] = intent_encode(ACTION_DIE, DIR_SELF, 0u);
        return;
    }

    // Priority 2: REPLICATE — energy > threshold AND empty neighbor exists
    let replication_threshold_byte = genome_get_byte(&voxel_read, idx, 2u);
    let threshold = (u32(params.replication_energy_min) * replication_threshold_byte) / 255u;

    if energy > threshold {
        // Find empty neighbors
        var empty_count: u32 = 0u;
        var empty_dirs: array<u32, 6>;
        for (var d: u32 = 0u; d < 6u; d++) {
            let ni = neighbor_in_direction(gid, d, gs);
            if ni != 0xFFFFFFFFu {
                if voxel_get_type(&voxel_read, ni) == VOXEL_EMPTY {
                    empty_dirs[empty_count] = d;
                    empty_count++;
                }
            }
        }

        if empty_count > 0u {
            // Pick random empty neighbor using replication target roll
            let chosen = roll_replication_target % empty_count;
            let target_dir = empty_dirs[chosen];
            // Bid = prng value modulated by energy
            let bid = roll_bid % (energy + 1u);
            intent_buf[idx] = intent_encode(ACTION_REPLICATE, target_dir, bid);
            return;
        }
    }

    // Priority 3: IDLE — fallback
    intent_buf[idx] = intent_encode(ACTION_IDLE, DIR_SELF, 0u);
}
