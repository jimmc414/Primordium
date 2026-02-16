// ============================================================
// brick_common.wgsl — Brick-aware indexing for sparse 256³ mode.
// Prepended between common.wgsl and each shader in sparse pipelines.
// NO entry points.
// ============================================================

// Brick table: maps brick coordinates to pool slot indices.
// 0xFFFFFFFF = unallocated brick.
@group(0) @binding(10) var<storage, read> brick_table: array<u32>;

// Brick coordinate to table index.
fn brick_coord_index(bx: u32, by: u32, bz: u32) -> u32 {
    let bgd = u32(params.brick_grid_dim);
    return bz * bgd * bgd + by * bgd + bx;
}

// Get pool-based flat index for a voxel at logical position.
// Returns 0xFFFFFFFF if the containing brick is unallocated.
fn sparse_voxel_index(pos: vec3<u32>, gs: u32) -> u32 {
    let bx = pos.x / 8u;
    let by = pos.y / 8u;
    let bz = pos.z / 8u;
    let slot = brick_table[brick_coord_index(bx, by, bz)];
    if slot == 0xFFFFFFFFu {
        return 0xFFFFFFFFu;
    }
    let local = (pos.z % 8u) * 64u + (pos.y % 8u) * 8u + (pos.x % 8u);
    return slot * 512u + local;
}

// Get pool index for a neighbor in a given direction.
// Returns 0xFFFFFFFF if out of bounds or in an unallocated brick.
fn sparse_neighbor(pos: vec3<u32>, dir: u32, gs: u32) -> u32 {
    let offset = NEIGHBORS[dir];
    let np = vec3<i32>(pos) + offset;
    if np.x < 0 || np.y < 0 || np.z < 0 ||
       np.x >= i32(gs) || np.y >= i32(gs) || np.z >= i32(gs) {
        return 0xFFFFFFFFu;
    }
    return sparse_voxel_index(vec3<u32>(np), gs);
}
