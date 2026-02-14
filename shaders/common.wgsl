// ============================================================
// common.wgsl — Shared constants, accessors, and helpers.
// Prepended to all compute shaders. NO entry points.
// ============================================================

// Voxel type constants
const VOXEL_EMPTY: u32 = 0u;
const VOXEL_WALL: u32 = 1u;
const VOXEL_NUTRIENT: u32 = 2u;
const VOXEL_ENERGY_SOURCE: u32 = 3u;
const VOXEL_PROTOCELL: u32 = 4u;
const VOXEL_WASTE: u32 = 5u;
const VOXEL_HEAT_SOURCE: u32 = 6u;
const VOXEL_COLD_SOURCE: u32 = 7u;

// Each voxel is 8 × u32 = 32 bytes
const VOXEL_STRIDE: u32 = 8u;

// Von Neumann neighborhood (6 face-adjacent offsets)
const NEIGHBORS = array<vec3<i32>, 6>(
    vec3<i32>( 1,  0,  0),
    vec3<i32>(-1,  0,  0),
    vec3<i32>( 0,  1,  0),
    vec3<i32>( 0, -1,  0),
    vec3<i32>( 0,  0,  1),
    vec3<i32>( 0,  0, -1),
);

// ---- Grid coordinate helpers ----

fn grid_index(pos: vec3<u32>, grid_size: u32) -> u32 {
    return pos.z * grid_size * grid_size + pos.y * grid_size + pos.x;
}

fn grid_coords(index: u32, grid_size: u32) -> vec3<u32> {
    let x = index % grid_size;
    let y = (index / grid_size) % grid_size;
    let z = index / (grid_size * grid_size);
    return vec3<u32>(x, y, z);
}

// ---- Voxel accessors (array<u32>, NOT struct) ----
// Word 0: [0:7] type  [8:15] flags  [16:31] energy (u16)
// Word 1: [0:15] age (u16)  [16:31] species_id (u16)
// Words 2-5: genome (4 × u32)
// Words 6-7: extra

fn voxel_get_type(buf: ptr<storage, array<u32>, read>, idx: u32) -> u32 {
    let base = idx * VOXEL_STRIDE;
    return (*buf)[base] & 0xFFu;
}

fn voxel_get_flags(buf: ptr<storage, array<u32>, read>, idx: u32) -> u32 {
    let base = idx * VOXEL_STRIDE;
    return ((*buf)[base] >> 8u) & 0xFFu;
}

fn voxel_get_energy(buf: ptr<storage, array<u32>, read>, idx: u32) -> u32 {
    let base = idx * VOXEL_STRIDE;
    return ((*buf)[base] >> 16u) & 0xFFFFu;
}

fn voxel_get_age(buf: ptr<storage, array<u32>, read>, idx: u32) -> u32 {
    let base = idx * VOXEL_STRIDE;
    return (*buf)[base + 1u] & 0xFFFFu;
}

fn voxel_get_species_id(buf: ptr<storage, array<u32>, read>, idx: u32) -> u32 {
    let base = idx * VOXEL_STRIDE;
    return ((*buf)[base + 1u] >> 16u) & 0xFFFFu;
}

fn voxel_get_genome_word(buf: ptr<storage, array<u32>, read>, idx: u32, word: u32) -> u32 {
    let base = idx * VOXEL_STRIDE;
    return (*buf)[base + 2u + word];
}

fn voxel_get_extra(buf: ptr<storage, array<u32>, read>, idx: u32, word: u32) -> u32 {
    let base = idx * VOXEL_STRIDE;
    return (*buf)[base + 6u + word];
}

// ---- PCG-RXS-M-XS-32 PRNG ----

fn pcg_hash(input: u32) -> u32 {
    var state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn pcg_next(state: ptr<function, u32>) -> u32 {
    let old = *state;
    *state = old * 747796405u + 2891336453u;
    let word = ((old >> ((old >> 28u) + 4u)) ^ old) * 277803737u;
    return (word >> 22u) ^ word;
}

fn prng_seed(voxel_index: u32, tick_count: u32, grid_size: u32, dispatch_salt: u32) -> u32 {
    return pcg_hash(voxel_index ^ (tick_count * 0x9E3779B9u) ^ (grid_size * 0x85EBCA6Bu) ^ dispatch_salt);
}

// ---- Genome byte accessor ----

fn genome_get_byte(buf: ptr<storage, array<u32>, read>, idx: u32, byte_index: u32) -> u32 {
    let word_index = byte_index / 4u;
    let byte_in_word = byte_index % 4u;
    let word = voxel_get_genome_word(buf, idx, word_index);
    return (word >> (byte_in_word * 8u)) & 0xFFu;
}

// ---- Genome byte accessor from raw words (not buffer pointer) ----

fn genome_get_byte_from_words(g0: u32, g1: u32, g2: u32, g3: u32, byte_index: u32) -> u32 {
    var words = array<u32, 4>(g0, g1, g2, g3);
    let word_i = byte_index / 4u;
    let shift = (byte_index % 4u) * 8u;
    return (words[word_i] >> shift) & 0xFFu;
}

// ---- Intent constants ----

const ACTION_NO_ACTION: u32 = 0u;
const ACTION_DIE: u32 = 1u;
const ACTION_PREDATE: u32 = 2u;
const ACTION_REPLICATE: u32 = 3u;
const ACTION_MOVE: u32 = 4u;
const ACTION_IDLE: u32 = 5u;
const DIR_SELF: u32 = 6u;

// ---- Intent encode/decode ----
// Intent word (u32): [0:2] direction, [3:5] action, [6:31] bid

fn intent_encode(action: u32, direction: u32, bid: u32) -> u32 {
    return (direction & 7u) | ((action & 7u) << 3u) | ((bid & 0x03FFFFFFu) << 6u);
}

fn intent_get_direction(intent: u32) -> u32 {
    return intent & 7u;
}

fn intent_get_action(intent: u32) -> u32 {
    return (intent >> 3u) & 7u;
}

fn intent_get_bid(intent: u32) -> u32 {
    return (intent >> 6u) & 0x03FFFFFFu;
}

// ---- Species ID hash (matches Genome::species_id() in Rust) ----

fn compute_species_id(g0: u32, g1: u32, g2: u32, g3: u32) -> u32 {
    var x = g0 ^ g1 ^ g2 ^ g3;
    x = ((x >> 8u) ^ x) * 0x6979u;
    x = ((x >> 8u) ^ x) * 0x0235u;
    x = (x >> 16u) ^ x;
    let id = x & 0xFFFFu;
    return select(id, 1u, id == 0u);  // SIM-5: never zero
}

// ---- Neighbor / direction utilities ----

fn neighbor_in_direction(pos: vec3<u32>, dir: u32, gs: u32) -> u32 {
    let offset = NEIGHBORS[dir];
    let np = vec3<i32>(pos) + offset;
    if np.x < 0 || np.y < 0 || np.z < 0 ||
       np.x >= i32(gs) || np.y >= i32(gs) || np.z >= i32(gs) {
        return 0xFFFFFFFFu;
    }
    return grid_index(vec3<u32>(np), gs);
}

fn opposite_direction(d: u32) -> u32 {
    return d ^ 1u;
}

// ---- Color helpers ----

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let hp = h * 6.0;
    let x = c * (1.0 - abs(hp % 2.0 - 1.0));
    let m = v - c;
    var rgb: vec3<f32>;
    if hp < 1.0 {
        rgb = vec3<f32>(c, x, 0.0);
    } else if hp < 2.0 {
        rgb = vec3<f32>(x, c, 0.0);
    } else if hp < 3.0 {
        rgb = vec3<f32>(0.0, c, x);
    } else if hp < 4.0 {
        rgb = vec3<f32>(0.0, x, c);
    } else if hp < 5.0 {
        rgb = vec3<f32>(x, 0.0, c);
    } else {
        rgb = vec3<f32>(c, 0.0, x);
    }
    return rgb + vec3<f32>(m, m, m);
}
