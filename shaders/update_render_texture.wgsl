// ============================================================
// update_render_texture.wgsl — Maps voxel data to RGBA in 3D texture.
// Prepended with common.wgsl at pipeline creation.
//
// Bind group 0:
//   [0] voxel_buf: storage<array<u32>, read>
//   [1] render_tex: texture_storage_3d<rgba8unorm, write>
//   [2] params: uniform<SimParams>
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

@group(0) @binding(0) var<storage, read> voxel_buf: array<u32>;
@group(0) @binding(1) var render_tex: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: SimParams;

@compute @workgroup_size(4, 4, 4)
fn update_render_texture_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let gs = u32(params.grid_size);
    if gid.x >= gs || gid.y >= gs || gid.z >= gs {
        return;
    }

    let idx = grid_index(gid, gs);
    let base = idx * VOXEL_STRIDE;
    let word0 = voxel_buf[base];
    let word1 = voxel_buf[base + 1u];

    let vtype = word0 & 0xFFu;
    let energy = (word0 >> 16u) & 0xFFFFu;
    let age = word1 & 0xFFFFu;
    let species_id = (word1 >> 16u) & 0xFFFFu;

    var color: vec4<f32>;

    switch vtype {
        case 0u: {
            // EMPTY — transparent
            color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        case 1u: {
            // WALL — gray
            color = vec4<f32>(0.5, 0.5, 0.5, 1.0);
        }
        case 2u: {
            // NUTRIENT — green
            color = vec4<f32>(0.2, 0.8, 0.2, 0.8);
        }
        case 3u: {
            // ENERGY_SOURCE — bright yellow
            color = vec4<f32>(1.0, 0.95, 0.2, 1.0);
        }
        case 4u: {
            // PROTOCELL — HSV from species_id and energy
            let hue = fract(f32(species_id) * 0.618033988749);
            let val = clamp(f32(energy) / params.max_energy, 0.1, 1.0);
            let rgb = hsv_to_rgb(hue, 0.7, val);
            color = vec4<f32>(rgb, 1.0);
        }
        case 5u: {
            // WASTE — dark brown, alpha decays with age
            let alpha = clamp(1.0 - f32(age) / params.waste_decay_ticks, 0.2, 0.9);
            color = vec4<f32>(0.35, 0.2, 0.1, alpha);
        }
        case 6u: {
            // HEAT_SOURCE — orange-red
            color = vec4<f32>(1.0, 0.4, 0.1, 1.0);
        }
        case 7u: {
            // COLD_SOURCE — ice blue
            color = vec4<f32>(0.3, 0.6, 1.0, 1.0);
        }
        default: {
            color = vec4<f32>(1.0, 0.0, 1.0, 1.0); // magenta = error
        }
    }

    textureStore(render_tex, gid, color);
}
