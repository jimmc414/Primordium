use wgpu;

/// CPU-managed brick allocation table for sparse 256³ grids.
/// Maps brick coordinates (8³ voxels each) to pool slot indices.
/// 0xFFFFFFFF = unallocated brick.
pub struct SparseGrid {
    brick_table: Vec<u32>,
    free_list: Vec<u32>,
    brick_grid_dim: u32,
    max_bricks: u32,
    active_brick_count: u32,
    brick_table_buf: wgpu::Buffer,
    brick_table_dirty: bool,
}

impl SparseGrid {
    pub fn new(device: &wgpu::Device, brick_grid_dim: u32, max_bricks: u32) -> Self {
        let table_size = (brick_grid_dim as usize).pow(3);
        let brick_table = vec![0xFFFFFFFFu32; table_size];

        // Free list: all pool slots available, ordered 0..max_bricks
        let free_list: Vec<u32> = (0..max_bricks).rev().collect();

        let brick_table_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("brick_table"),
            size: (table_size * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            brick_table,
            free_list,
            brick_grid_dim,
            max_bricks,
            active_brick_count: 0,
            brick_table_buf,
            brick_table_dirty: true, // upload initial state
        }
    }

    fn table_index(&self, bx: u32, by: u32, bz: u32) -> usize {
        let dim = self.brick_grid_dim as usize;
        (bz as usize) * dim * dim + (by as usize) * dim + (bx as usize)
    }

    /// Allocate a brick at (bx, by, bz). Returns the pool slot index, or None if full.
    pub fn allocate_brick(&mut self, bx: u32, by: u32, bz: u32) -> Option<u32> {
        let idx = self.table_index(bx, by, bz);
        if self.brick_table[idx] != 0xFFFFFFFF {
            return Some(self.brick_table[idx]); // already allocated
        }
        let slot = self.free_list.pop()?;
        self.brick_table[idx] = slot;
        self.active_brick_count += 1;
        self.brick_table_dirty = true;
        Some(slot)
    }

    /// Deallocate a brick at (bx, by, bz).
    pub fn deallocate_brick(&mut self, bx: u32, by: u32, bz: u32) {
        let idx = self.table_index(bx, by, bz);
        if self.brick_table[idx] == 0xFFFFFFFF {
            return;
        }
        let slot = self.brick_table[idx];
        self.brick_table[idx] = 0xFFFFFFFF;
        self.free_list.push(slot);
        self.active_brick_count -= 1;
        self.brick_table_dirty = true;
    }

    /// Ensure a brick is allocated for the voxel at (x, y, z).
    pub fn ensure_brick_for_voxel(&mut self, x: u32, y: u32, z: u32) -> Option<u32> {
        let bx = x / 8;
        let by = y / 8;
        let bz = z / 8;
        self.allocate_brick(bx, by, bz)
    }

    /// Returns the pool slot for a voxel, or None if the brick is unallocated.
    pub fn voxel_pool_index(&self, x: u32, y: u32, z: u32) -> Option<u32> {
        let bx = x / 8;
        let by = y / 8;
        let bz = z / 8;
        let idx = self.table_index(bx, by, bz);
        let slot = self.brick_table[idx];
        if slot == 0xFFFFFFFF {
            return None;
        }
        let local = (z % 8) * 64 + (y % 8) * 8 + (x % 8);
        Some(slot * 512 + local)
    }

    /// Upload brick table to GPU if dirty.
    pub fn upload_if_dirty(&mut self, queue: &wgpu::Queue) {
        if !self.brick_table_dirty {
            return;
        }
        let bytes: &[u8] = bytemuck::cast_slice(&self.brick_table);
        queue.write_buffer(&self.brick_table_buf, 0, bytes);
        self.brick_table_dirty = false;
    }

    /// For each allocated brick, allocate all 6 face-adjacent bricks if not present.
    pub fn proactive_border_alloc(&mut self) {
        let dim = self.brick_grid_dim;
        // Collect currently allocated brick coords to avoid mutation during iteration
        let mut allocated = Vec::new();
        for bz in 0..dim {
            for by in 0..dim {
                for bx in 0..dim {
                    let idx = self.table_index(bx, by, bz);
                    if self.brick_table[idx] != 0xFFFFFFFF {
                        allocated.push((bx, by, bz));
                    }
                }
            }
        }

        let offsets: [(i32, i32, i32); 6] = [
            (1, 0, 0), (-1, 0, 0),
            (0, 1, 0), (0, -1, 0),
            (0, 0, 1), (0, 0, -1),
        ];

        for (bx, by, bz) in allocated {
            for (dx, dy, dz) in &offsets {
                let nx = bx as i32 + dx;
                let ny = by as i32 + dy;
                let nz = bz as i32 + dz;
                if nx >= 0 && nx < dim as i32 && ny >= 0 && ny < dim as i32 && nz >= 0 && nz < dim as i32 {
                    let _ = self.allocate_brick(nx as u32, ny as u32, nz as u32);
                }
            }
        }
    }

    pub fn brick_table_buffer(&self) -> &wgpu::Buffer {
        &self.brick_table_buf
    }

    pub fn active_brick_count(&self) -> u32 {
        self.active_brick_count
    }

    pub fn max_bricks(&self) -> u32 {
        self.max_bricks
    }

    pub fn brick_grid_dim(&self) -> u32 {
        self.brick_grid_dim
    }

    /// Check if a brick at (bx, by, bz) is allocated.
    pub fn is_allocated(&self, bx: u32, by: u32, bz: u32) -> bool {
        let idx = self.table_index(bx, by, bz);
        self.brick_table[idx] != 0xFFFFFFFF
    }

    /// Deallocate bricks that have zero voxel occupancy.
    /// `occupancy` is a slice of per-brick voxel counts read back from GPU.
    pub fn deallocate_empty_bricks(&mut self, occupancy: &[u32]) {
        let dim = self.brick_grid_dim;
        for bz in 0..dim {
            for by in 0..dim {
                for bx in 0..dim {
                    let idx = self.table_index(bx, by, bz);
                    let slot = self.brick_table[idx];
                    if slot == 0xFFFFFFFF {
                        continue;
                    }
                    if (slot as usize) < occupancy.len() && occupancy[slot as usize] == 0 {
                        self.deallocate_brick(bx, by, bz);
                    }
                }
            }
        }
    }
}
