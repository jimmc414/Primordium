// ============================================================
// apply_commands.wgsl — M4: Apply player commands.
// Modifies the current READ buffer in-place.
// Runs BEFORE intent_declaration each tick.
// Prepended with common.wgsl at pipeline creation.
//
// Bind group 0:
//   [0] voxel_buf:   storage<array<u32>, read_write>  — current read buffer
//   [1] command_buf: storage<array<u32>, read>         — command count + data
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
    overlay_mode: f32,
    sparse_mode: f32,
    brick_grid_dim: f32,
    max_bricks: f32,
};

@group(0) @binding(0) var<storage, read_write> voxel_buf: array<u32>;
@group(0) @binding(1) var<storage, read> command_buf: array<u32>;
@group(0) @binding(2) var<uniform> params: SimParams;

// Command types
const CMD_NOOP: u32 = 0u;
const CMD_PLACE_VOXEL: u32 = 1u;
const CMD_REMOVE_VOXEL: u32 = 2u;
const CMD_SEED_PROTOCELLS: u32 = 3u;
const CMD_APPLY_TOXIN: u32 = 4u;

fn write_voxel_inplace(idx: u32, w0: u32, w1: u32, w2: u32, w3: u32, w4: u32, w5: u32, w6: u32, w7: u32) {
    let base = idx * VOXEL_STRIDE;
    voxel_buf[base]      = w0;
    voxel_buf[base + 1u] = w1;
    voxel_buf[base + 2u] = w2;
    voxel_buf[base + 3u] = w3;
    voxel_buf[base + 4u] = w4;
    voxel_buf[base + 5u] = w5;
    voxel_buf[base + 6u] = w6;
    voxel_buf[base + 7u] = w7;
}

fn read_voxel_type_rw(idx: u32) -> u32 {
    let base = idx * VOXEL_STRIDE;
    return voxel_buf[base] & 0xFFu;
}

@compute @workgroup_size(4, 4, 4)
fn apply_commands_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let gs = u32(params.grid_size);
    if gid.x >= gs || gid.y >= gs || gid.z >= gs {
        return;
    }

    let command_count = min(command_buf[0], 64u);
    if command_count == 0u {
        return;
    }

    var idx: u32;
    if params.sparse_mode > 0.0 {
        idx = sparse_voxel_index(gid, gs);
        if idx == 0xFFFFFFFFu { return; }
    } else {
        idx = grid_index(gid, gs);
    }
    let my_pos = vec3<i32>(gid);

    for (var c: u32 = 0u; c < command_count; c++) {
        let cmd_base = 4u + c * 16u;
        let cmd_type = command_buf[cmd_base];
        let cmd_x = command_buf[cmd_base + 1u];
        let cmd_y = command_buf[cmd_base + 2u];
        let cmd_z = command_buf[cmd_base + 3u];
        let cmd_radius = command_buf[cmd_base + 4u];
        let cmd_param_0 = command_buf[cmd_base + 5u];

        if cmd_type == CMD_NOOP {
            continue;
        }

        // Chebyshev distance for cube-shaped brush
        let cmd_pos = vec3<i32>(i32(cmd_x), i32(cmd_y), i32(cmd_z));
        let diff = abs(my_pos - cmd_pos);
        let dist = max(diff.x, max(diff.y, diff.z));
        if dist > i32(cmd_radius) {
            continue;
        }

        let current_type = read_voxel_type_rw(idx);

        switch cmd_type {
            case 1u: { // CMD_PLACE_VOXEL
                let vtype = cmd_param_0;
                var energy: u32 = 0u;
                if vtype == VOXEL_ENERGY_SOURCE {
                    energy = 500u;
                } else if vtype == VOXEL_NUTRIENT {
                    energy = u32(params.energy_from_nutrient);
                } else if vtype == VOXEL_HEAT_SOURCE || vtype == VOXEL_COLD_SOURCE {
                    energy = 1000u;
                }
                write_voxel_inplace(idx,
                    (vtype & 0xFFu) | ((energy & 0xFFFFu) << 16u),
                    0u, 0u, 0u, 0u, 0u, 0u, 0u);
            }
            case 2u: { // CMD_REMOVE_VOXEL
                write_voxel_inplace(idx, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u);
            }
            case 3u: { // CMD_SEED_PROTOCELLS
                if current_type == VOXEL_EMPTY {
                    // PRNG for random genome generation
                    var rng = prng_seed(idx, u32(params.tick_count), gs, 0x3u);
                    let g0 = pcg_next(&rng);
                    let g1 = pcg_next(&rng);
                    let g2 = pcg_next(&rng);
                    let g3 = pcg_next(&rng);
                    let species_id = compute_species_id(g0, g1, g2, g3);
                    let energy = min(cmd_param_0, 0xFFFFu);
                    write_voxel_inplace(idx,
                        (VOXEL_PROTOCELL & 0xFFu) | ((energy & 0xFFFFu) << 16u),
                        (species_id & 0xFFFFu) << 16u,
                        g0, g1, g2, g3, 0u, 0u);
                }
            }
            case 4u: { // CMD_APPLY_TOXIN
                if current_type == VOXEL_PROTOCELL {
                    let base = idx * VOXEL_STRIDE;
                    let g0 = voxel_buf[base + 2u];
                    let g1 = voxel_buf[base + 3u];
                    let g2 = voxel_buf[base + 4u];
                    let g3 = voxel_buf[base + 5u];
                    let toxin_resistance = genome_get_byte_from_words(g0, g1, g2, g3, 6u);
                    if toxin_resistance < cmd_param_0 {
                        let species_id = (voxel_buf[base + 1u] >> 16u) & 0xFFFFu;
                        write_voxel_inplace(idx,
                            VOXEL_WASTE & 0xFFu,
                            (species_id & 0xFFFFu) << 16u,
                            0u, 0u, 0u, 0u, 0u, 0u);
                    }
                }
            }
            default: {
                // Unknown command, skip
            }
        }
    }
}
