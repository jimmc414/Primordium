// ============================================================
// resolve_execute.wgsl — M4: Intent-aware resolve + execute.
// Metabolism, death, nutrient cycling, replication, AND movement.
// Prepended with common.wgsl at pipeline creation.
//
// Bind group 0:
//   [0] voxel_read:   storage<array<u32>, read>
//   [1] voxel_write:  storage<array<u32>, read_write>
//   [2] params:       uniform<SimParams>
//   [3] intent_read:  storage<array<u32>, read>
// ============================================================
//
// ---- CASE ENUMERATION (SH-1: mandatory before implementation) ----
//
// EMPTY voxel at position P:
//   E1: No contenders → nutrient spawn roll or stay empty
//   E2: Exactly one REPLICATE contender → write offspring
//   E3: Exactly one MOVE contender → copy mover's state, apply movement cost + metabolism
//   E4: Multiple contenders (any mix of REPLICATE/MOVE) → highest bid wins
//       If winner is REPLICATE → apply E2
//       If winner is MOVE → apply E3
//
// PROTOCELL voxel at position P:
//   P1: own intent = DIE → WASTE
//   P2a: own REPLICATE won at target → deduct split energy, metabolism
//   P2b: own REPLICATE lost → keep energy, metabolism
//   P3: own intent = IDLE → metabolism only
//   P4a: own MOVE won at target → write EMPTY at source (mover left)
//   P4b: own MOVE lost → keep position, metabolism
//   All P cases: if energy reaches 0 after metabolism → WASTE
//
// NUTRIENT voxel at position P:
//   N1: no adjacent protocells → age++, copy
//   N2: adjacent protocells → deplete energy by count
//   N3: energy reaches 0 → convert to EMPTY
//
// WASTE voxel at position P:
//   W1: age < waste_decay_ticks → age++, copy
//   W2: age >= waste_decay_ticks → roll for nutrient recycle or EMPTY
//
// Others (WALL, ENERGY_SOURCE, HEAT_SOURCE, COLD_SOURCE):
//   X1: copy unchanged
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
@group(0) @binding(3) var<storage, read> intent_read: array<u32>;

// ---- Local helpers ----

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

// ---- Contender winner resolution ----
// Reads 6 neighbors of target_pos. For each: check if intent action is REPLICATE
// or MOVE and direction points toward target_pos (using opposite_direction).
// Returns vec3(winner_voxel_index, winner_bid, winner_action).
// If no winner, returns (0xFFFFFFFF, 0, 0).

fn find_contender_winner(target_pos: vec3<u32>, gs: u32) -> vec3<u32> {
    var best_idx: u32 = 0xFFFFFFFFu;
    var best_bid: u32 = 0u;
    var best_action: u32 = 0u;

    for (var d: u32 = 0u; d < 6u; d++) {
        let ni = neighbor_in_direction(target_pos, d, gs);
        if ni == 0xFFFFFFFFu {
            continue;
        }
        let intent = intent_read[ni];
        let action = intent_get_action(intent);
        if action != ACTION_REPLICATE && action != ACTION_MOVE {
            continue;
        }
        // Check if this neighbor's intent direction points toward target_pos.
        // Neighbor is in direction d from target. For it to target us,
        // its direction must be opposite_direction(d).
        let intent_dir = intent_get_direction(intent);
        if intent_dir != opposite_direction(d) {
            continue;
        }
        let bid = intent_get_bid(intent);
        // Highest bid wins; tie-break: higher voxel index
        if bid > best_bid || (bid == best_bid && ni > best_idx) {
            best_bid = bid;
            best_idx = ni;
            best_action = action;
        }
    }

    return vec3<u32>(best_idx, best_bid, best_action);
}

// ---- Mutation ----
// 16 PRNG advances (one per genome byte).
// If (roll & 0xFF) < mutation_rate → replace byte with (roll >> 8) & 0xFF.

fn mutate_genome(rng_ptr: ptr<function, u32>, mutation_rate: u32,
                 g0_ptr: ptr<function, u32>, g1_ptr: ptr<function, u32>,
                 g2_ptr: ptr<function, u32>, g3_ptr: ptr<function, u32>) {
    var words = array<u32, 4>(*g0_ptr, *g1_ptr, *g2_ptr, *g3_ptr);
    for (var byte_i: u32 = 0u; byte_i < 16u; byte_i++) {
        let roll = pcg_next(rng_ptr);
        let word_i = byte_i / 4u;
        let shift = (byte_i % 4u) * 8u;
        if (roll & 0xFFu) < mutation_rate {
            let new_byte = (roll >> 8u) & 0xFFu;
            // Clear old byte and set new one
            words[word_i] = (words[word_i] & ~(0xFFu << shift)) | (new_byte << shift);
        }
    }
    *g0_ptr = words[0];
    *g1_ptr = words[1];
    *g2_ptr = words[2];
    *g3_ptr = words[3];
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
        case 0u: { // EMPTY — cases E1, E2, E3, E4
            // Check if any neighbor wants to replicate or move into this cell
            let winner = find_contender_winner(gid, gs);
            let winner_idx = winner.x;
            let winner_action = winner.z;

            if winner_idx == 0xFFFFFFFFu {
                // E1: No contenders — nutrient spawn or stay empty
                let roll = pcg_next(&rng);
                let threshold = u32(params.nutrient_spawn_rate * 4294967295.0);
                if roll < threshold {
                    let energy = u32(params.energy_from_nutrient);
                    write_voxel(idx,
                        pack_word0(VOXEL_NUTRIENT, 0u, energy),
                        pack_word1(0u, 0u),
                        0u, 0u, 0u, 0u, 0u, 0u);
                } else {
                    write_empty(idx);
                }
            } else if winner_action == ACTION_REPLICATE {
                // E2/E4 (REPLICATE winner): Write offspring into this cell
                let parent_energy = voxel_get_energy(&voxel_read, winner_idx);
                let split_ratio_byte = genome_get_byte(&voxel_read, winner_idx, 10u);
                let mutation_rate = genome_get_byte(&voxel_read, winner_idx, 3u);

                // Offspring energy = parent_energy * (255 - split_ratio) / 255
                let offspring_energy = (parent_energy * (255u - split_ratio_byte)) / 255u;

                // Copy parent genome
                var g0 = voxel_get_genome_word(&voxel_read, winner_idx, 0u);
                var g1 = voxel_get_genome_word(&voxel_read, winner_idx, 1u);
                var g2 = voxel_get_genome_word(&voxel_read, winner_idx, 2u);
                var g3 = voxel_get_genome_word(&voxel_read, winner_idx, 3u);

                // Mutate genome (16 PRNG advances)
                mutate_genome(&rng, mutation_rate, &g0, &g1, &g2, &g3);

                // Compute species_id from MUTATED genome (SIM-5: never 0)
                let species_id = compute_species_id(g0, g1, g2, g3);

                // Write offspring: age=0, offspring energy, mutated genome
                write_voxel(idx,
                    pack_word0(VOXEL_PROTOCELL, 0u, offspring_energy),
                    pack_word1(0u, species_id),
                    g0, g1, g2, g3, 0u, 0u);
            } else {
                // E3/E4 (MOVE winner): Copy mover's state to destination
                let mover_energy = voxel_get_energy(&voxel_read, winner_idx);
                let mover_age = voxel_get_age(&voxel_read, winner_idx);
                let mover_species = voxel_get_species_id(&voxel_read, winner_idx);
                let g0 = voxel_get_genome_word(&voxel_read, winner_idx, 0u);
                let g1 = voxel_get_genome_word(&voxel_read, winner_idx, 1u);
                let g2 = voxel_get_genome_word(&voxel_read, winner_idx, 2u);
                let g3 = voxel_get_genome_word(&voxel_read, winner_idx, 3u);

                // Read genome params from raw words (no mutation on move)
                let metabolic_efficiency = genome_get_byte_from_words(g0, g1, g2, g3, 0u);
                let metabolic_rate = genome_get_byte_from_words(g0, g1, g2, g3, 1u);
                let photosynthetic_rate = genome_get_byte_from_words(g0, g1, g2, g3, 9u);

                // Metabolism at destination: scan OWN neighbors for energy gain
                var gain: u32 = 0u;
                for (var d: u32 = 0u; d < 6u; d++) {
                    let ni = neighbor_in_direction(gid, d, gs);
                    if ni == 0xFFFFFFFFu {
                        continue;
                    }
                    let ntype = voxel_get_type(&voxel_read, ni);
                    if ntype == VOXEL_ENERGY_SOURCE {
                        gain += (photosynthetic_rate * u32(params.energy_from_source)) / 255u;
                    } else if ntype == VOXEL_NUTRIENT {
                        gain += (metabolic_efficiency * u32(params.energy_from_nutrient)) / 255u;
                    }
                }

                let cost = u32(params.metabolic_cost_base) * (255u + metabolic_rate) / 255u;
                let movement_cost = u32(params.movement_energy_cost);

                var new_energy = min(mover_energy + gain, u32(params.max_energy));
                // Saturating subtract movement cost (SIM-4)
                new_energy = select(0u, new_energy - movement_cost, new_energy >= movement_cost);
                // Saturating subtract metabolic cost (SIM-4)
                new_energy = select(0u, new_energy - cost, new_energy >= cost);

                let new_age = min(mover_age + 1u, 0xFFFFu);

                if new_energy == 0u {
                    // Death at destination → WASTE
                    write_voxel(idx,
                        pack_word0(VOXEL_WASTE, 0u, 0u),
                        pack_word1(0u, mover_species),
                        0u, 0u, 0u, 0u, 0u, 0u);
                } else {
                    write_voxel(idx,
                        pack_word0(VOXEL_PROTOCELL, 0u, new_energy),
                        pack_word1(new_age, mover_species),
                        g0, g1, g2, g3, 0u, 0u);
                }
            }
        }
        case 4u: { // PROTOCELL — cases P1, P2a, P2b, P3, P4a, P4b
            let energy = voxel_get_energy(&voxel_read, idx);
            let age = voxel_get_age(&voxel_read, idx);
            let species_id = voxel_get_species_id(&voxel_read, idx);

            // Read genome
            let g0 = voxel_get_genome_word(&voxel_read, idx, 0u);
            let g1 = voxel_get_genome_word(&voxel_read, idx, 1u);
            let g2 = voxel_get_genome_word(&voxel_read, idx, 2u);
            let g3 = voxel_get_genome_word(&voxel_read, idx, 3u);

            let metabolic_efficiency = genome_get_byte(&voxel_read, idx, 0u);
            let metabolic_rate = genome_get_byte(&voxel_read, idx, 1u);
            let photosynthetic_rate = genome_get_byte(&voxel_read, idx, 9u);
            let split_ratio_byte = genome_get_byte(&voxel_read, idx, 10u);

            // Always consume 16 PRNG advances for determinism (mutation slots)
            for (var i: u32 = 0u; i < 16u; i++) {
                _ = pcg_next(&rng);
            }

            // Read own intent
            let my_intent = intent_read[idx];
            let my_action = intent_get_action(my_intent);

            // P1: DIE
            if my_action == ACTION_DIE {
                write_voxel(idx,
                    pack_word0(VOXEL_WASTE, 0u, 0u),
                    pack_word1(0u, species_id),
                    0u, 0u, 0u, 0u, 0u, 0u);
                return;
            }

            // Determine energy after replication/move cost
            var work_energy = energy;
            var moved_away = false;

            if my_action == ACTION_REPLICATE {
                // Compute target position
                let my_dir = intent_get_direction(my_intent);
                let target_ni = neighbor_in_direction(gid, my_dir, gs);

                if target_ni != 0xFFFFFFFFu {
                    let target_pos = grid_coords(target_ni, gs);
                    let winner = find_contender_winner(target_pos, gs);

                    if winner.x == idx {
                        // P2a: Won the replication contest
                        // Parent keeps: energy * split_ratio / 255
                        work_energy = (energy * split_ratio_byte) / 255u;
                    }
                    // P2b: Lost — work_energy stays as full energy
                }
            } else if my_action == ACTION_MOVE {
                let my_dir = intent_get_direction(my_intent);
                let target_ni = neighbor_in_direction(gid, my_dir, gs);

                if target_ni != 0xFFFFFFFFu {
                    let target_pos = grid_coords(target_ni, gs);
                    let winner = find_contender_winner(target_pos, gs);

                    if winner.x == idx {
                        // P4a: Won the move contest — this cell becomes EMPTY
                        moved_away = true;
                    }
                    // P4b: Lost — stay in place, metabolism as normal
                }
            }
            // P3: IDLE — work_energy stays as full energy

            if moved_away {
                // P4a: Protocell moved away, write EMPTY at source
                write_empty(idx);
                return;
            }

            // Metabolism: scan neighbors for energy gain
            var gain: u32 = 0u;
            for (var d: u32 = 0u; d < 6u; d++) {
                let ni = neighbor_in_direction(gid, d, gs);
                if ni == 0xFFFFFFFFu {
                    continue;
                }
                let ntype = voxel_get_type(&voxel_read, ni);
                if ntype == VOXEL_ENERGY_SOURCE {
                    gain += (photosynthetic_rate * u32(params.energy_from_source)) / 255u;
                } else if ntype == VOXEL_NUTRIENT {
                    gain += (metabolic_efficiency * u32(params.energy_from_nutrient)) / 255u;
                }
            }

            // Metabolic cost: base * (1 + metabolic_rate/255)
            let cost = u32(params.metabolic_cost_base) * (255u + metabolic_rate) / 255u;

            // Apply gain, clamp to max_energy
            var new_energy = min(work_energy + gain, u32(params.max_energy));

            // Saturating subtract cost (SIM-4)
            new_energy = select(0u, new_energy - cost, new_energy >= cost);

            let new_age = min(age + 1u, 0xFFFFu);

            if new_energy == 0u {
                // Death after metabolism → WASTE
                write_voxel(idx,
                    pack_word0(VOXEL_WASTE, 0u, 0u),
                    pack_word1(0u, species_id),
                    0u, 0u, 0u, 0u, 0u, 0u);
            } else {
                write_voxel(idx,
                    pack_word0(VOXEL_PROTOCELL, 0u, new_energy),
                    pack_word1(new_age, species_id),
                    g0, g1, g2, g3, 0u, 0u);
            }
        }
        case 2u: { // NUTRIENT — cases N1, N2, N3
            let energy = voxel_get_energy(&voxel_read, idx);
            let age = voxel_get_age(&voxel_read, idx);

            var adj_protocells: u32 = 0u;
            for (var d: u32 = 0u; d < 6u; d++) {
                let ni = neighbor_in_direction(gid, d, gs);
                if ni == 0xFFFFFFFFu {
                    continue;
                }
                if voxel_get_type(&voxel_read, ni) == VOXEL_PROTOCELL {
                    adj_protocells++;
                }
            }

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
        case 5u: { // WASTE — cases W1, W2
            let age = voxel_get_age(&voxel_read, idx);
            let species_id = voxel_get_species_id(&voxel_read, idx);
            let new_age = min(age + 1u, 0xFFFFu);

            if new_age >= u32(params.waste_decay_ticks) {
                let roll = pcg_next(&rng);
                let threshold = u32(params.nutrient_recycle_rate * 4294967295.0);
                if roll < threshold {
                    let energy = u32(params.energy_from_nutrient);
                    write_voxel(idx,
                        pack_word0(VOXEL_NUTRIENT, 0u, energy),
                        pack_word1(0u, 0u),
                        0u, 0u, 0u, 0u, 0u, 0u);
                } else {
                    write_empty(idx);
                }
            } else {
                write_voxel(idx,
                    pack_word0(VOXEL_WASTE, 0u, 0u),
                    pack_word1(new_age, species_id),
                    0u, 0u, 0u, 0u, 0u, 0u);
            }
        }
        default: {
            // WALL, ENERGY_SOURCE, HEAT_SOURCE, COLD_SOURCE — copy unchanged (X1)
            copy_voxel(idx);
        }
    }
}
