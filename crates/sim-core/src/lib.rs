pub mod buffers;
pub mod uniform;
pub mod pipelines;
pub mod tick;
pub mod stats;
pub mod sparse;

pub use stats::SimStats;

use buffers::{VoxelBuffers, SparseVoxelBuffers};
use uniform::ParamsUniform;
use pipelines::{SimPipelines, SparsePipelines};
use sparse::SparseGrid;
use types::{SimParams, Voxel, VoxelType, Genome};

/// Dense mode: all bind groups for the 5-dispatch pipeline.
pub(crate) struct DenseMode {
    pub(crate) buffers: VoxelBuffers,
    pub(crate) pipelines: SimPipelines,
    pub(crate) intent_bg_even: wgpu::BindGroup,
    pub(crate) intent_bg_odd: wgpu::BindGroup,
    pub(crate) resolve_bg_even: wgpu::BindGroup,
    pub(crate) resolve_bg_odd: wgpu::BindGroup,
    pub(crate) apply_cmd_bg_even: wgpu::BindGroup,
    pub(crate) apply_cmd_bg_odd: wgpu::BindGroup,
    pub(crate) temp_diffusion_bg_even: wgpu::BindGroup,
    pub(crate) temp_diffusion_bg_odd: wgpu::BindGroup,
    pub(crate) stats_bg_even: wgpu::BindGroup,
    pub(crate) stats_bg_odd: wgpu::BindGroup,
}

/// Sparse mode: pool-based buffers + brick_table bind groups.
pub(crate) struct SparseMode {
    pub(crate) buffers: SparseVoxelBuffers,
    pub(crate) grid: SparseGrid,
    pub(crate) pipelines: SparsePipelines,
    pub(crate) intent_bg_even: wgpu::BindGroup,
    pub(crate) intent_bg_odd: wgpu::BindGroup,
    pub(crate) resolve_bg_even: wgpu::BindGroup,
    pub(crate) resolve_bg_odd: wgpu::BindGroup,
    pub(crate) apply_cmd_bg_even: wgpu::BindGroup,
    pub(crate) apply_cmd_bg_odd: wgpu::BindGroup,
    pub(crate) temp_diffusion_bg_even: wgpu::BindGroup,
    pub(crate) temp_diffusion_bg_odd: wgpu::BindGroup,
    pub(crate) stats_bg_even: wgpu::BindGroup,
    pub(crate) stats_bg_odd: wgpu::BindGroup,
    pub(crate) border_alloc_counter: u32,
}

pub(crate) enum SimMode {
    Dense(DenseMode),
    Sparse(SparseMode),
}

pub struct SimEngine {
    mode: SimMode,
    params_uniform: ParamsUniform,
    pub params: SimParams,
    tick_count: u32,
}

impl SimEngine {
    pub fn try_new(device: &wgpu::Device, _queue: &wgpu::Queue, grid_size: u32) -> Result<Self, String> {
        let mut params = SimParams::default();
        params.grid_size = grid_size as f32;
        let buffers = VoxelBuffers::try_new(device, grid_size)?;
        let params_uniform = ParamsUniform::new(device, &params);
        let pipelines = SimPipelines::new(device);

        let intent_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("intent_bg_even"),
            layout: &pipelines.intent_declaration_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.buffer_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.intent_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: buffers.temp_buffer_b().as_entire_binding() },
            ],
        });

        let intent_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("intent_bg_odd"),
            layout: &pipelines.intent_declaration_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.buffer_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.intent_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: buffers.temp_buffer_a().as_entire_binding() },
            ],
        });

        let resolve_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("resolve_bg_even"),
            layout: &pipelines.resolve_execute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.buffer_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.buffer_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: buffers.intent_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: buffers.temp_buffer_b().as_entire_binding() },
            ],
        });

        let resolve_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("resolve_bg_odd"),
            layout: &pipelines.resolve_execute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.buffer_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.buffer_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: buffers.intent_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: buffers.temp_buffer_a().as_entire_binding() },
            ],
        });

        let apply_cmd_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("apply_cmd_bg_even"),
            layout: &pipelines.apply_commands_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.buffer_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.command_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
            ],
        });

        let apply_cmd_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("apply_cmd_bg_odd"),
            layout: &pipelines.apply_commands_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.buffer_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.command_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
            ],
        });

        let temp_diffusion_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("temp_diffusion_bg_even"),
            layout: &pipelines.temperature_diffusion_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.temp_buffer_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.temp_buffer_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: buffers.buffer_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params_uniform.buffer.as_entire_binding() },
            ],
        });

        let temp_diffusion_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("temp_diffusion_bg_odd"),
            layout: &pipelines.temperature_diffusion_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.temp_buffer_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.temp_buffer_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: buffers.buffer_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params_uniform.buffer.as_entire_binding() },
            ],
        });

        let stats_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stats_bg_even"),
            layout: &pipelines.stats_reduction_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.buffer_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.stats_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
            ],
        });

        let stats_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stats_bg_odd"),
            layout: &pipelines.stats_reduction_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.buffer_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.stats_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
            ],
        });

        let dense = DenseMode {
            buffers, pipelines,
            intent_bg_even, intent_bg_odd,
            resolve_bg_even, resolve_bg_odd,
            apply_cmd_bg_even, apply_cmd_bg_odd,
            temp_diffusion_bg_even, temp_diffusion_bg_odd,
            stats_bg_even, stats_bg_odd,
        };

        Ok(Self {
            mode: SimMode::Dense(dense),
            params_uniform,
            params,
            tick_count: 0,
        })
    }

    /// Create a sparse 256³ engine with brick-based storage.
    pub fn try_new_sparse(device: &wgpu::Device, _queue: &wgpu::Queue, grid_size: u32, max_bricks: u32) -> Result<Self, String> {
        let brick_grid_dim = grid_size / 8;
        let mut params = SimParams::default();
        params.grid_size = grid_size as f32;
        params.sparse_mode = 1.0;
        params.brick_grid_dim = brick_grid_dim as f32;
        params.max_bricks = max_bricks as f32;

        let buffers = SparseVoxelBuffers::try_new(device, grid_size, max_bricks)?;
        let grid = SparseGrid::new(device, brick_grid_dim, max_bricks);
        let params_uniform = ParamsUniform::new(device, &params);
        let pipelines = SparsePipelines::new(device);

        let bt = grid.brick_table_buffer();

        let intent_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparse_intent_bg_even"),
            layout: &pipelines.intent_declaration_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.pool_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.intent_pool().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: buffers.temp_pool_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 10, resource: bt.as_entire_binding() },
            ],
        });

        let intent_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparse_intent_bg_odd"),
            layout: &pipelines.intent_declaration_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.pool_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.intent_pool().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: buffers.temp_pool_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 10, resource: bt.as_entire_binding() },
            ],
        });

        let resolve_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparse_resolve_bg_even"),
            layout: &pipelines.resolve_execute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.pool_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.pool_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: buffers.intent_pool().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: buffers.temp_pool_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 10, resource: bt.as_entire_binding() },
            ],
        });

        let resolve_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparse_resolve_bg_odd"),
            layout: &pipelines.resolve_execute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.pool_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.pool_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: buffers.intent_pool().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: buffers.temp_pool_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 10, resource: bt.as_entire_binding() },
            ],
        });

        let apply_cmd_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparse_apply_cmd_bg_even"),
            layout: &pipelines.apply_commands_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.pool_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.command_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 10, resource: bt.as_entire_binding() },
            ],
        });

        let apply_cmd_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparse_apply_cmd_bg_odd"),
            layout: &pipelines.apply_commands_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.pool_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.command_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 10, resource: bt.as_entire_binding() },
            ],
        });

        let temp_diffusion_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparse_temp_diffusion_bg_even"),
            layout: &pipelines.temperature_diffusion_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.temp_pool_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.temp_pool_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: buffers.pool_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 10, resource: bt.as_entire_binding() },
            ],
        });

        let temp_diffusion_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparse_temp_diffusion_bg_odd"),
            layout: &pipelines.temperature_diffusion_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.temp_pool_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.temp_pool_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: buffers.pool_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 10, resource: bt.as_entire_binding() },
            ],
        });

        let stats_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparse_stats_bg_even"),
            layout: &pipelines.stats_reduction_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.pool_b().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.stats_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 10, resource: bt.as_entire_binding() },
            ],
        });

        let stats_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sparse_stats_bg_odd"),
            layout: &pipelines.stats_reduction_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffers.pool_a().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffers.stats_buffer().as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params_uniform.buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 10, resource: bt.as_entire_binding() },
            ],
        });

        let sparse = SparseMode {
            buffers, grid, pipelines,
            intent_bg_even, intent_bg_odd,
            resolve_bg_even, resolve_bg_odd,
            apply_cmd_bg_even, apply_cmd_bg_odd,
            temp_diffusion_bg_even, temp_diffusion_bg_odd,
            stats_bg_even, stats_bg_odd,
            border_alloc_counter: 0,
        };

        Ok(Self {
            mode: SimMode::Sparse(sparse),
            params_uniform,
            params,
            tick_count: 0,
        })
    }

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, grid_size: u32) -> Self {
        Self::try_new(device, queue, grid_size).expect("Failed to create SimEngine")
    }

    pub fn is_sparse(&self) -> bool {
        matches!(self.mode, SimMode::Sparse(_))
    }

    /// Seed the grid with default initial conditions (Petri Dish preset).
    pub fn initialize_grid(&mut self, queue: &wgpu::Queue) {
        self.seed_petri_dish(queue);
    }

    pub fn current_read_buffer(&self) -> &wgpu::Buffer {
        match &self.mode {
            SimMode::Dense(d) => d.buffers.current_read_buffer(),
            SimMode::Sparse(s) => s.buffers.current_read_pool(),
        }
    }

    pub fn params_buffer(&self) -> &wgpu::Buffer {
        &self.params_uniform.buffer
    }

    pub fn grid_size(&self) -> u32 {
        match &self.mode {
            SimMode::Dense(d) => d.buffers.grid_size(),
            SimMode::Sparse(s) => s.buffers.grid_size(),
        }
    }

    pub fn command_buffer(&self) -> &wgpu::Buffer {
        match &self.mode {
            SimMode::Dense(d) => d.buffers.command_buffer(),
            SimMode::Sparse(s) => s.buffers.command_buffer(),
        }
    }

    pub fn current_temp_buffer(&self) -> &wgpu::Buffer {
        match &self.mode {
            SimMode::Dense(d) => d.buffers.current_temp_read(),
            SimMode::Sparse(s) => s.buffers.current_temp_read(),
        }
    }

    pub fn stats_staging_buffer(&self) -> &wgpu::Buffer {
        match &self.mode {
            SimMode::Dense(d) => d.buffers.stats_staging_buffer(),
            SimMode::Sparse(s) => s.buffers.stats_staging_buffer(),
        }
    }

    pub fn tick_count(&self) -> u32 {
        self.tick_count
    }

    pub fn current_write_buffer(&self) -> &wgpu::Buffer {
        match &self.mode {
            SimMode::Dense(d) => d.buffers.current_write_buffer(),
            SimMode::Sparse(s) => s.buffers.current_write_pool(),
        }
    }

    pub fn brick_table_buffer(&self) -> Option<&wgpu::Buffer> {
        match &self.mode {
            SimMode::Dense(_) => None,
            SimMode::Sparse(s) => Some(s.grid.brick_table_buffer()),
        }
    }

    pub fn reset_tick_count(&mut self) {
        self.tick_count = 0;
        match &mut self.mode {
            SimMode::Dense(d) => d.buffers.reset_read_is_a(),
            SimMode::Sparse(s) => s.buffers.reset_read_is_a(),
        }
    }

    /// Load a preset by ID: 0=Petri Dish, 1=Gradient, 2=Arena
    pub fn initialize_grid_with_preset(&mut self, queue: &wgpu::Queue, preset: u32) {
        self.clear_voxel_buffer_a(queue);
        match preset {
            0 => self.seed_petri_dish(queue),
            1 => self.seed_gradient(queue),
            2 => self.seed_arena(queue),
            _ => self.seed_petri_dish(queue),
        }
    }

    /// Clear the primary voxel buffer (A) to zeros.
    fn clear_voxel_buffer_a(&mut self, queue: &wgpu::Queue) {
        match &mut self.mode {
            SimMode::Dense(d) => {
                let gs = d.buffers.grid_size();
                let total = (gs as usize).pow(3);
                let zero_data = vec![0u8; total * 32];
                queue.write_buffer(d.buffers.buffer_a(), 0, &zero_data);
            }
            SimMode::Sparse(s) => {
                // Clear entire pool A and reset brick table
                let pool_size = (s.buffers.max_bricks() as usize) * 512 * 32;
                let zero_data = vec![0u8; pool_size];
                queue.write_buffer(s.buffers.pool_a(), 0, &zero_data);
                // Reset all brick allocations
                let dim = s.grid.brick_grid_dim();
                for bz in 0..dim {
                    for by in 0..dim {
                        for bx in 0..dim {
                            if s.grid.is_allocated(bx, by, bz) {
                                s.grid.deallocate_brick(bx, by, bz);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Write a single voxel to buffer A (used during seeding).
    fn write_voxel(&mut self, queue: &wgpu::Queue, x: u32, y: u32, z: u32, words: &[u32; 8]) {
        let bytes: &[u8] = bytemuck::cast_slice(words.as_slice());
        match &mut self.mode {
            SimMode::Dense(d) => {
                let gs = d.buffers.grid_size();
                let idx = types::grid_index(x, y, z, gs);
                let byte_offset = (idx as u64) * 32;
                queue.write_buffer(d.buffers.buffer_a(), byte_offset, bytes);
            }
            SimMode::Sparse(s) => {
                s.grid.ensure_brick_for_voxel(x, y, z);
                if let Some(pool_idx) = s.grid.voxel_pool_index(x, y, z) {
                    let byte_offset = (pool_idx as u64) * 32;
                    queue.write_buffer(s.buffers.pool_a(), byte_offset, bytes);
                }
            }
        }
    }

    fn seed_petri_dish(&mut self, queue: &wgpu::Queue) {
        let gs = self.grid_size();
        let center = gs / 2;
        let mut voxel_data: Vec<(u32, u32, u32, [u32; 8])> = Vec::new();

        // Walls (5 scattered)
        for i in 0..5u32 {
            let x = center.saturating_sub(15) + i * 3;
            let y = center.saturating_sub(15);
            let z = center;
            let v = Voxel {
                voxel_type: VoxelType::Wall,
                energy: 0,
                ..Default::default()
            };
            voxel_data.push((x.min(gs - 1), y.min(gs - 1), z, v.pack()));
        }

        // Nutrient field (scaled to grid size)
        let nutrient_half = (gs / 10).max(4);
        for dx in 0..(nutrient_half * 2) {
            for dy in 0..(nutrient_half * 2) {
                for dz in 0..(nutrient_half * 2) {
                    let x = center - nutrient_half + dx;
                    let y = center - nutrient_half + dy;
                    let z = center - nutrient_half + dz;
                    if x < gs && y < gs && z < gs {
                        let v = Voxel {
                            voxel_type: VoxelType::Nutrient,
                            energy: 200,
                            ..Default::default()
                        };
                        voxel_data.push((x, y, z, v.pack()));
                    }
                }
            }
        }

        // Energy sources (3 near center)
        for i in 0..3u32 {
            let x = (center - 1 + i).min(gs - 1);
            let v = Voxel {
                voxel_type: VoxelType::EnergySource,
                energy: 500,
                ..Default::default()
            };
            voxel_data.push((x, center, center, v.pack()));
        }

        // Protocells (~50 in tight cluster near center)
        for i in 0..50u32 {
            let angle = (i as f32) * 0.126;
            let radius = 1.0 + (i as f32) * 0.08;
            let layer = (i / 16) as f32;
            let x = ((center as f32 + angle.cos() * radius).round() as u32).min(gs - 1);
            let y = ((center as f32 + angle.sin() * radius).round() as u32).min(gs - 1);
            let z = ((center as f32 - 2.0 + layer).round() as u32).min(gs - 1);

            let mut genome = Genome::default();
            genome.bytes[0] = (80 + (i % 20) * 8) as u8;
            genome.bytes[1] = (30 + (i % 15) * 5) as u8;
            genome.bytes[2] = 200;
            genome.bytes[3] = (i * 3) as u8;
            genome.bytes[4] = (60 + (i % 10) * 15) as u8;
            genome.bytes[5] = (40 + (i % 8) * 20) as u8;
            genome.bytes[9] = (60 + (i % 10) * 15) as u8;
            genome.bytes[10] = 128;
            let species = genome.species_id();
            let v = Voxel {
                voxel_type: VoxelType::Protocell,
                energy: 500,
                species_id: species,
                genome,
                ..Default::default()
            };
            voxel_data.push((x, y, z, v.pack()));
        }

        // Waste (5 voxels)
        for i in 0..5u32 {
            let x = (center + 8 + i).min(gs - 1);
            let y = (center + 8).min(gs - 1);
            let v = Voxel {
                voxel_type: VoxelType::Waste,
                age: i as u16 * 20,
                ..Default::default()
            };
            voxel_data.push((x, y, center, v.pack()));
        }

        // Heat source
        {
            let x = (center + 10).min(gs - 1);
            let z = (center + 10).min(gs - 1);
            let v = Voxel {
                voxel_type: VoxelType::HeatSource,
                energy: 1000,
                ..Default::default()
            };
            voxel_data.push((x, center, z, v.pack()));
        }

        // Cold source
        {
            let x = center.saturating_sub(10);
            let z = (center + 10).min(gs - 1);
            let v = Voxel {
                voxel_type: VoxelType::ColdSource,
                energy: 1000,
                ..Default::default()
            };
            voxel_data.push((x, center, z, v.pack()));
        }

        // Upload all voxels
        for (x, y, z, words) in &voxel_data {
            self.write_voxel(queue, *x, *y, *z, words);
        }

        self.finalize_seed(queue);
    }

    fn seed_gradient(&mut self, queue: &wgpu::Queue) {
        let gs = self.grid_size();
        let mut voxel_data: Vec<(u32, u32, u32, [u32; 8])> = Vec::new();

        // Heat sources along x=0 face
        for y in (0..gs).step_by((gs / 8) as usize) {
            for z in (0..gs).step_by((gs / 8) as usize) {
                let v = Voxel { voxel_type: VoxelType::HeatSource, energy: 1000, ..Default::default() };
                voxel_data.push((0, y, z, v.pack()));
            }
        }

        // Cold sources along x=gs-1 face
        for y in (0..gs).step_by((gs / 8) as usize) {
            for z in (0..gs).step_by((gs / 8) as usize) {
                let v = Voxel { voxel_type: VoxelType::ColdSource, energy: 1000, ..Default::default() };
                voxel_data.push((gs - 1, y, z, v.pack()));
            }
        }

        // Scattered nutrients in the middle third
        let third = gs / 3;
        for dx in 0..third {
            for dy in (0..gs).step_by(3) {
                for dz in (0..gs).step_by(3) {
                    let x = third + dx;
                    if x < gs && dy < gs && dz < gs {
                        let v = Voxel { voxel_type: VoxelType::Nutrient, energy: 200, ..Default::default() };
                        voxel_data.push((x, dy, dz, v.pack()));
                    }
                }
            }
        }

        // Energy sources in center strip
        let center = gs / 2;
        for y in (0..gs).step_by((gs / 6).max(1) as usize) {
            for z in (0..gs).step_by((gs / 6).max(1) as usize) {
                let v = Voxel { voxel_type: VoxelType::EnergySource, energy: 500, ..Default::default() };
                voxel_data.push((center, y, z, v.pack()));
            }
        }

        // Protocells scattered across the grid
        for i in 0..80u32 {
            let x = (third + (i * 7) % third).min(gs - 1);
            let y = ((i * 13) % gs).min(gs - 1);
            let z = ((i * 17) % gs).min(gs - 1);

            let mut genome = Genome::default();
            genome.bytes[0] = (100 + (i % 15) * 10) as u8;
            genome.bytes[1] = (40 + (i % 10) * 8) as u8;
            genome.bytes[2] = 180;
            genome.bytes[3] = (20 + i * 2) as u8;
            genome.bytes[4] = (80 + (i % 8) * 15) as u8;
            genome.bytes[5] = (60 + (i % 6) * 25) as u8;
            genome.bytes[9] = (50 + (i % 12) * 15) as u8;
            genome.bytes[10] = 128;
            let species = genome.species_id();
            let v = Voxel {
                voxel_type: VoxelType::Protocell,
                energy: 500,
                species_id: species,
                genome,
                ..Default::default()
            };
            voxel_data.push((x, y, z, v.pack()));
        }

        for (x, y, z, words) in &voxel_data {
            self.write_voxel(queue, *x, *y, *z, words);
        }

        self.finalize_seed(queue);
    }

    fn seed_arena(&mut self, queue: &wgpu::Queue) {
        let gs = self.grid_size();
        let center = gs / 2;
        let mut voxel_data: Vec<(u32, u32, u32, [u32; 8])> = Vec::new();

        let gap = (gs / 16).max(2);
        for i in 0..gs {
            for z in 0..gs {
                if !(center.saturating_sub(gap)..=center + gap).contains(&i) {
                    let v = Voxel { voxel_type: VoxelType::Wall, ..Default::default() };
                    voxel_data.push((center, i, z, v.pack()));
                }
                if !(center.saturating_sub(gap)..=center + gap).contains(&i) {
                    let v = Voxel { voxel_type: VoxelType::Wall, ..Default::default() };
                    voxel_data.push((i, center, z, v.pack()));
                }
            }
        }

        let q_size = center.saturating_sub(1);
        for dx in (1..q_size).step_by(2) {
            for dy in (1..q_size).step_by(2) {
                for dz in (0..gs).step_by(4) {
                    let v = Voxel { voxel_type: VoxelType::Nutrient, energy: 300, ..Default::default() };
                    voxel_data.push((dx, dy, dz, v.pack()));
                }
            }
        }

        for i in 0..4u32 {
            let x = (center + 2 + i * (q_size / 5)).min(gs - 1);
            let y = q_size / 2;
            let v = Voxel { voxel_type: VoxelType::HeatSource, energy: 1000, ..Default::default() };
            voxel_data.push((x, y, center, v.pack()));
        }
        for i in 0..6u32 {
            let x = (center + 2 + i * (q_size / 7)).min(gs - 1);
            let y = (1 + i * (q_size / 7)).min(center.saturating_sub(2));
            let v = Voxel { voxel_type: VoxelType::EnergySource, energy: 500, ..Default::default() };
            voxel_data.push((x, y, center, v.pack()));
        }

        for i in 0..4u32 {
            let x = (1 + i * (q_size / 5)).min(center.saturating_sub(2));
            let y = (center + 2 + i * (q_size / 5)).min(gs - 1);
            let v = Voxel { voxel_type: VoxelType::ColdSource, energy: 1000, ..Default::default() };
            voxel_data.push((x, y, center, v.pack()));
        }
        for dx in (1..q_size).step_by(6) {
            for dy in (center + 2..gs.saturating_sub(1)).step_by(6) {
                let v = Voxel { voxel_type: VoxelType::Nutrient, energy: 100, ..Default::default() };
                voxel_data.push((dx, dy, center, v.pack()));
            }
        }

        for i in 0..3u32 {
            let x = (center + 2 + i * (q_size / 4)).min(gs - 1);
            let y = (center + 2 + i * (q_size / 4)).min(gs - 1);
            let v = Voxel { voxel_type: VoxelType::EnergySource, energy: 500, ..Default::default() };
            voxel_data.push((x, y, center, v.pack()));
        }
        for dx in (center + 2..gs.saturating_sub(1)).step_by(4) {
            for dy in (center + 2..gs.saturating_sub(1)).step_by(4) {
                let v = Voxel { voxel_type: VoxelType::Nutrient, energy: 200, ..Default::default() };
                voxel_data.push((dx, dy, center, v.pack()));
            }
        }

        let quadrant_centers = [
            (center / 2, center / 2),
            (center + center / 2, center / 2),
            (center / 2, center + center / 2),
            (center + center / 2, center + center / 2),
        ];
        for (qi, &(qx, qy)) in quadrant_centers.iter().enumerate() {
            for i in 0..15u32 {
                let angle = (i as f32) * 0.42;
                let radius = 1.0 + (i as f32) * 0.15;
                let x = ((qx as f32 + angle.cos() * radius).round() as u32).min(gs - 1);
                let y = ((qy as f32 + angle.sin() * radius).round() as u32).min(gs - 1);

                let mut genome = Genome::default();
                genome.bytes[0] = 80 + (qi as u8) * 30 + (i as u8) * 5;
                genome.bytes[1] = 40 + (qi as u8) * 20 + (i as u8) * 3;
                genome.bytes[2] = 200;
                genome.bytes[3] = 10 + (qi as u8) * 15;
                genome.bytes[4] = 60 + (i % 8) as u8 * 15;
                genome.bytes[5] = 40 + (i % 6) as u8 * 20;
                genome.bytes[9] = 50 + (qi as u8) * 30;
                genome.bytes[10] = 128;
                let species = genome.species_id();
                let v = Voxel {
                    voxel_type: VoxelType::Protocell,
                    energy: 500,
                    species_id: species,
                    genome,
                    ..Default::default()
                };
                voxel_data.push((x, y, center, v.pack()));
            }
        }

        for (x, y, z, words) in &voxel_data {
            self.write_voxel(queue, *x, *y, *z, words);
        }

        self.finalize_seed(queue);
    }

    /// Seed ~30% of voxels as protocells for benchmarking. Returns count placed.
    pub fn seed_benchmark(&mut self, queue: &wgpu::Queue) -> u32 {
        let gs = self.grid_size();
        self.clear_voxel_buffer_a(queue);

        let mut count = 0u32;
        for x in 0..gs {
            for y in 0..gs {
                for z in 0..gs {
                    let h = x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663) ^ z.wrapping_mul(83492791);
                    if h % 10 < 3 {
                        let mut genome = Genome::default();
                        genome.bytes[0] = ((h >> 8) & 0xFF) as u8;
                        genome.bytes[1] = ((h >> 16) & 0xFF) as u8;
                        genome.bytes[2] = 200;
                        genome.bytes[3] = 30;
                        genome.bytes[4] = ((h >> 4) & 0xFF) as u8;
                        genome.bytes[5] = ((h >> 12) & 0xFF) as u8;
                        genome.bytes[9] = ((h >> 20) & 0xFF) as u8;
                        genome.bytes[10] = 128;
                        let species = genome.species_id();
                        let v = Voxel {
                            voxel_type: VoxelType::Protocell,
                            energy: 500,
                            species_id: species,
                            genome,
                            ..Default::default()
                        };
                        let words = v.pack();
                        self.write_voxel(queue, x, y, z, &words);
                        count += 1;
                    }
                }
            }
        }

        self.init_temperature(queue);
        self.reset_tick_count();
        self.params_uniform.upload(queue, &self.params);
        count
    }

    /// Common finalization after any seeding method.
    fn finalize_seed(&mut self, queue: &wgpu::Queue) {
        // For sparse mode, allocate border bricks and upload table
        if let SimMode::Sparse(s) = &mut self.mode {
            s.grid.proactive_border_alloc();
            s.grid.upload_if_dirty(queue);
        }
        self.init_temperature(queue);
        self.params_uniform.upload(queue, &self.params);
    }

    fn init_temperature(&self, queue: &wgpu::Queue) {
        let ambient = 0.5f32;
        let ambient_bytes = ambient.to_le_bytes();
        match &self.mode {
            SimMode::Dense(d) => {
                let gs = d.buffers.grid_size();
                let total_voxels = (gs as usize).pow(3);
                let init_data: Vec<u8> = ambient_bytes.repeat(total_voxels);
                queue.write_buffer(d.buffers.temp_buffer_a(), 0, &init_data);
            }
            SimMode::Sparse(s) => {
                // Fill entire temperature pool with ambient temp
                let pool_voxels = (s.buffers.max_bricks() as usize) * 512;
                let init_data: Vec<u8> = ambient_bytes.repeat(pool_voxels);
                queue.write_buffer(s.buffers.temp_pool_a(), 0, &init_data);
            }
        }
    }
}
