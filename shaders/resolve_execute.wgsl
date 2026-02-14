// ============================================================
// resolve_execute.wgsl — M2: Metabolism, death, nutrient cycling.
// No intents, no movement, no replication yet.
// Prepended with common.wgsl at pipeline creation.
//
// Bind group 0:
//   [0] voxel_read:  storage<array<u32>, read>
//   [1] voxel_write: storage<array<u32>, read_write>
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
@group(0) @binding(1) var<storage, read_write> voxel_write: array<u32>;
@group(0) @binding(2) var<uniform> params: SimParams;

// ---- Local helpers ----

fn neighbor_idx(pos: vec3<u32>, dir: u32, gs: u32) -> u32 {
    let offset = NEIGHBORS[dir];
    let np = vec3<i32>(pos) + offset;
    if np.x < 0 || np.y < 0 || np.z < 0 ||
       np.x >= i32(gs) || np.y >= i32(gs) || np.z >= i32(gs) {
        return 0xFFFFFFFFu;
    }
    return grid_index(vec3<u32>(np), gs);
}

fn pack_word0(vtype: u32, flags: u32, energy: u32) -> u32 {
    return (vtype & 0xFFu) | ((flags & 0xFFu) << 8u) | ((energy & 0xFFFFu) << 16u);
}

fn pack_word1(age: u32, species_id: u32) -> u32 {
    return (age & 0xFFFFu) | ((species_id & 0xFFFFu) << 16u);
}

fn write_voxel(idx: u32, w0: u32, w1: u32, w2: u32, w3: u32, w4: u32, w5: u32, w6: u32, w7: u32) {
    let base = idx * VOXEL_STRIDE;
    voxel_write[base]      = w0;
    voxel_write[base + 1u] = w1;
    voxel_write[base + 2u] = w2;
    voxel_write[base + 3u] = w3;
    voxel_write[base + 4u] = w4;
    voxel_write[base + 5u] = w5;
    voxel_write[base + 6u] = w6;
    voxel_write[base + 7u] = w7;
}

fn copy_voxel(idx: u32) {
    let base = idx * VOXEL_STRIDE;
    voxel_write[base]      = voxel_read[base];
    voxel_write[base + 1u] = voxel_read[base + 1u];
    voxel_write[base + 2u] = voxel_read[base + 2u];
    voxel_write[base + 3u] = voxel_read[base + 3u];
    voxel_write[base + 4u] = voxel_read[base + 4u];
    voxel_write[base + 5u] = voxel_read[base + 5u];
    voxel_write[base + 6u] = voxel_read[base + 6u];
    voxel_write[base + 7u] = voxel_read[base + 7u];
}

fn write_empty(idx: u32) {
    write_voxel(idx, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u);
}

// ---- Entry point ----

@compute @workgroup_size(4, 4, 4)
fn resolve_execute_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let gs = u32(params.grid_size);
    if gid.x >= gs || gid.y >= gs || gid.z >= gs {
        return;
    }

    let idx = grid_index(gid, gs);
    let vtype = voxel_get_type(&voxel_read, idx);

    // Initialize PRNG with dispatch salt 0x2
    var rng = prng_seed(idx, u32(params.tick_count), gs, 0x2u);

    switch vtype {
        case 0u: { // EMPTY
            let roll = pcg_next(&rng);
            let threshold = u32(params.nutrient_spawn_rate * 4294967295.0);
            if roll < threshold {
                // Spawn nutrient with energy = energy_from_nutrient
                let energy = u32(params.energy_from_nutrient);
                write_voxel(idx,
                    pack_word0(VOXEL_NUTRIENT, 0u, energy),
                    pack_word1(0u, 0u),
                    0u, 0u, 0u, 0u, 0u, 0u);
            } else {
                write_empty(idx);
            }
        }
        case 4u: { // PROTOCELL
            let energy = voxel_get_energy(&voxel_read, idx);
            let age = voxel_get_age(&voxel_read, idx);
            let species_id = voxel_get_species_id(&voxel_read, idx);

            // Read genome bytes
            let metabolic_efficiency = genome_get_byte(&voxel_read, idx, 0u);
            let metabolic_rate = genome_get_byte(&voxel_read, idx, 1u);
            let photosynthetic_rate = genome_get_byte(&voxel_read, idx, 9u);

            // Scan 6 neighbors for energy gain
            var gain: u32 = 0u;
            for (var d: u32 = 0u; d < 6u; d++) {
                let ni = neighbor_idx(gid, d, gs);
                if ni == 0xFFFFFFFFu {
                    continue;
                }
                let ntype = voxel_get_type(&voxel_read, ni);
                if ntype == VOXEL_ENERGY_SOURCE {
                    // Photosynthetic gain per adjacent energy source
                    gain += (photosynthetic_rate * u32(params.energy_from_source)) / 255u;
                } else if ntype == VOXEL_NUTRIENT {
                    // Metabolic gain per adjacent nutrient
                    gain += (metabolic_efficiency * u32(params.energy_from_nutrient)) / 255u;
                }
            }

            // Compute metabolic cost: base * (1 + metabolic_rate/255)
            let cost = u32(params.metabolic_cost_base) * (255u + metabolic_rate) / 255u;

            // Apply gain, clamp to max_energy
            var new_energy = min(energy + gain, u32(params.max_energy));

            // Saturating subtract cost (SIM-4)
            new_energy = select(0u, new_energy - cost, new_energy >= cost);

            // Increment age with overflow guard
            let new_age = min(age + 1u, 0xFFFFu);

            // Copy genome words
            let g0 = voxel_get_genome_word(&voxel_read, idx, 0u);
            let g1 = voxel_get_genome_word(&voxel_read, idx, 1u);
            let g2 = voxel_get_genome_word(&voxel_read, idx, 2u);
            let g3 = voxel_get_genome_word(&voxel_read, idx, 3u);

            if new_energy == 0u {
                // Death -> WASTE (keep species_id for visual)
                write_voxel(idx,
                    pack_word0(VOXEL_WASTE, 0u, 0u),
                    pack_word1(0u, species_id),
                    0u, 0u, 0u, 0u, 0u, 0u);
            } else {
                // Write updated protocell
                write_voxel(idx,
                    pack_word0(VOXEL_PROTOCELL, 0u, new_energy),
                    pack_word1(new_age, species_id),
                    g0, g1, g2, g3, 0u, 0u);
            }
        }
        case 2u: { // NUTRIENT
            let energy = voxel_get_energy(&voxel_read, idx);
            let age = voxel_get_age(&voxel_read, idx);

            // Count adjacent protocells
            var adj_protocells: u32 = 0u;
            for (var d: u32 = 0u; d < 6u; d++) {
                let ni = neighbor_idx(gid, d, gs);
                if ni == 0xFFFFFFFFu {
                    continue;
                }
                if voxel_get_type(&voxel_read, ni) == VOXEL_PROTOCELL {
                    adj_protocells++;
                }
            }

            // Saturating subtract loss
            let new_energy = select(0u, energy - adj_protocells, energy >= adj_protocells);
            let new_age = min(age + 1u, 0xFFFFu);

            if new_energy == 0u {
                write_empty(idx);
            } else {
                write_voxel(idx,
                    pack_word0(VOXEL_NUTRIENT, 0u, new_energy),
                    pack_word1(new_age, 0u),
                    0u, 0u, 0u, 0u, 0u, 0u);
            }
        }
        case 5u: { // WASTE
            let age = voxel_get_age(&voxel_read, idx);
            let species_id = voxel_get_species_id(&voxel_read, idx);
            let new_age = min(age + 1u, 0xFFFFu);

            if new_age >= u32(params.waste_decay_ticks) {
                // Roll for nutrient recycling
                let roll = pcg_next(&rng);
                let threshold = u32(params.nutrient_recycle_rate * 4294967295.0);
                if roll < threshold {
                    // Recycle to nutrient
                    let energy = u32(params.energy_from_nutrient);
                    write_voxel(idx,
                        pack_word0(VOXEL_NUTRIENT, 0u, energy),
                        pack_word1(0u, 0u),
                        0u, 0u, 0u, 0u, 0u, 0u);
                } else {
                    write_empty(idx);
                }
            } else {
                // Still decaying
                write_voxel(idx,
                    pack_word0(VOXEL_WASTE, 0u, 0u),
                    pack_word1(new_age, species_id),
                    0u, 0u, 0u, 0u, 0u, 0u);
            }
        }
        default: {
            // WALL, ENERGY_SOURCE, HEAT_SOURCE, COLD_SOURCE — copy unchanged
            copy_voxel(idx);
        }
    }
}
