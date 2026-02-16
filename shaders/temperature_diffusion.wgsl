// ============================================================
// temperature_diffusion.wgsl — M5: Temperature field diffusion.
// Reads temp_read, writes temp_write. Heat/cold sources are
// Dirichlet boundaries. Walls are insulators.
// Prepended with common.wgsl at pipeline creation.
//
// Bind group 0:
//   [0] temp_read:   storage<array<f32>, read>
//   [1] temp_write:  storage<array<f32>, read_write>
//   [2] voxel_read:  storage<array<u32>, read>
//   [3] params:      uniform<SimParams>
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

@group(0) @binding(0) var<storage, read> temp_read: array<f32>;
@group(0) @binding(1) var<storage, read_write> temp_write: array<f32>;
@group(0) @binding(2) var<storage, read> voxel_read: array<u32>;
@group(0) @binding(3) var<uniform> params: SimParams;

@compute @workgroup_size(4, 4, 4)
fn temperature_diffusion_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let gs = u32(params.grid_size);
    if gid.x >= gs || gid.y >= gs || gid.z >= gs {
        return;
    }

    var idx: u32;
    if params.sparse_mode > 0.0 {
        idx = sparse_voxel_index(gid, gs);
        if idx == 0xFFFFFFFFu { return; }
    } else {
        idx = grid_index(gid, gs);
    }
    let vtype = voxel_get_type(&voxel_read, idx);
    let own_temp = temp_read[idx];

    // WALL: insulator, keep own temperature unchanged
    if vtype == VOXEL_WALL {
        temp_write[idx] = own_temp;
        return;
    }

    // HEAT_SOURCE: Dirichlet boundary at 1.0
    if vtype == VOXEL_HEAT_SOURCE {
        temp_write[idx] = 1.0;
        return;
    }

    // COLD_SOURCE: Dirichlet boundary at 0.0
    if vtype == VOXEL_COLD_SOURCE {
        temp_write[idx] = 0.0;
        return;
    }

    // All others: diffuse from non-wall, in-bounds neighbors
    var neighbor_sum: f32 = 0.0;
    var neighbor_count: f32 = 0.0;

    for (var d: u32 = 0u; d < 6u; d++) {
        var ni: u32;
        if params.sparse_mode > 0.0 {
            ni = sparse_neighbor(gid, d, gs);
        } else {
            ni = neighbor_in_direction(gid, d, gs);
        }
        if ni == 0xFFFFFFFFu {
            continue;
        }
        let ntype = voxel_get_type(&voxel_read, ni);
        if ntype == VOXEL_WALL {
            continue;
        }
        neighbor_sum += temp_read[ni];
        neighbor_count += 1.0;
    }

    var t_new: f32;
    if neighbor_count > 0.0 {
        let t_avg = neighbor_sum / neighbor_count;
        t_new = own_temp + params.diffusion_rate * (t_avg - own_temp);
    } else {
        t_new = own_temp;
    }

    // SIM-6: clamp to [0.0, 1.0]
    temp_write[idx] = clamp(t_new, 0.0, 1.0);
}
