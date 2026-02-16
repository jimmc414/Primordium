pub mod buffers;
pub mod uniform;
pub mod pipelines;
pub mod tick;
pub mod stats;

pub use stats::SimStats;

use buffers::VoxelBuffers;
use uniform::ParamsUniform;
use pipelines::SimPipelines;
use types::{SimParams, Voxel, VoxelType, Genome};

pub struct SimEngine {
    buffers: VoxelBuffers,
    params_uniform: ParamsUniform,
    pub params: SimParams,
    pipelines: SimPipelines,
    intent_bg_even: wgpu::BindGroup,     // intent: reads buf_a + temp_b
    intent_bg_odd: wgpu::BindGroup,      // intent: reads buf_b + temp_a
    resolve_bg_even: wgpu::BindGroup,    // resolve: reads A, writes B, reads intent + temp_b
    resolve_bg_odd: wgpu::BindGroup,     // resolve: reads B, writes A, reads intent + temp_a
    apply_cmd_bg_even: wgpu::BindGroup,  // apply_commands: reads/writes buf_a
    apply_cmd_bg_odd: wgpu::BindGroup,   // apply_commands: reads/writes buf_b
    temp_diffusion_bg_even: wgpu::BindGroup,  // reads temp_a, writes temp_b, reads voxel_a
    temp_diffusion_bg_odd: wgpu::BindGroup,   // reads temp_b, writes temp_a, reads voxel_b
    stats_bg_even: wgpu::BindGroup,  // stats reads voxel_buf_b (write buffer on even ticks)
    stats_bg_odd: wgpu::BindGroup,   // stats reads voxel_buf_a (write buffer on odd ticks)
    tick_count: u32,
}

impl SimEngine {
    pub fn try_new(device: &wgpu::Device, _queue: &wgpu::Queue, grid_size: u32) -> Result<Self, String> {
        let mut params = SimParams::default();
        params.grid_size = grid_size as f32;
        let buffers = VoxelBuffers::try_new(device, grid_size)?;
        let params_uniform = ParamsUniform::new(device, &params);
        let pipelines = SimPipelines::new(device);

        // Intent bind groups (4 entries each): voxel_read, intent_buf, params, temp_read
        // Even tick: diffusion writes temp_b, so intent reads temp_b
        // Odd tick: diffusion writes temp_a, so intent reads temp_a
        let intent_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("intent_bg_even"),
            layout: &pipelines.intent_declaration_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.buffer_a().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.intent_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_uniform.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.temp_buffer_b().as_entire_binding(),
                },
            ],
        });

        let intent_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("intent_bg_odd"),
            layout: &pipelines.intent_declaration_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.buffer_b().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.intent_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_uniform.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.temp_buffer_a().as_entire_binding(),
                },
            ],
        });

        // Resolve bind groups (5 entries each): voxel_read, voxel_write, params, intent_buf, temp_read
        // Even tick: diffusion writes temp_b, so resolve reads temp_b
        // Odd tick: diffusion writes temp_a, so resolve reads temp_a
        let resolve_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("resolve_bg_even"),
            layout: &pipelines.resolve_execute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.buffer_a().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.buffer_b().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_uniform.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.intent_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buffers.temp_buffer_b().as_entire_binding(),
                },
            ],
        });

        let resolve_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("resolve_bg_odd"),
            layout: &pipelines.resolve_execute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.buffer_b().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.buffer_a().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_uniform.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.intent_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buffers.temp_buffer_a().as_entire_binding(),
                },
            ],
        });

        // Apply commands bind groups (3 entries each): voxel_buf (rw), command_buf, params
        let apply_cmd_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("apply_cmd_bg_even"),
            layout: &pipelines.apply_commands_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.buffer_a().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.command_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_uniform.buffer.as_entire_binding(),
                },
            ],
        });

        let apply_cmd_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("apply_cmd_bg_odd"),
            layout: &pipelines.apply_commands_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.buffer_b().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.command_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_uniform.buffer.as_entire_binding(),
                },
            ],
        });

        // Temperature diffusion bind groups (4 entries each): temp_read, temp_write, voxel_read, params
        let temp_diffusion_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("temp_diffusion_bg_even"),
            layout: &pipelines.temperature_diffusion_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.temp_buffer_a().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.temp_buffer_b().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.buffer_a().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_uniform.buffer.as_entire_binding(),
                },
            ],
        });

        let temp_diffusion_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("temp_diffusion_bg_odd"),
            layout: &pipelines.temperature_diffusion_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.temp_buffer_b().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.temp_buffer_a().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.buffer_b().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_uniform.buffer.as_entire_binding(),
                },
            ],
        });

        // Stats bind groups (3 entries each): voxel_write, stats_buf, params
        // Even ticks: resolve writes B, so stats reads B
        let stats_bg_even = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stats_bg_even"),
            layout: &pipelines.stats_reduction_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.buffer_b().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.stats_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_uniform.buffer.as_entire_binding(),
                },
            ],
        });

        // Odd ticks: resolve writes A, so stats reads A
        let stats_bg_odd = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stats_bg_odd"),
            layout: &pipelines.stats_reduction_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.buffer_a().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.stats_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_uniform.buffer.as_entire_binding(),
                },
            ],
        });

        Ok(Self {
            buffers,
            params_uniform,
            params,
            pipelines,
            intent_bg_even,
            intent_bg_odd,
            resolve_bg_even,
            resolve_bg_odd,
            apply_cmd_bg_even,
            apply_cmd_bg_odd,
            temp_diffusion_bg_even,
            temp_diffusion_bg_odd,
            stats_bg_even,
            stats_bg_odd,
            tick_count: 0,
        })
    }

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, grid_size: u32) -> Self {
        Self::try_new(device, queue, grid_size).expect("Failed to create SimEngine")
    }

    /// Seed the grid with default initial conditions (Petri Dish preset).
    pub fn initialize_grid(&self, queue: &wgpu::Queue) {
        self.seed_petri_dish(queue);
    }

    pub fn current_read_buffer(&self) -> &wgpu::Buffer {
        self.buffers.current_read_buffer()
    }

    pub fn params_buffer(&self) -> &wgpu::Buffer {
        &self.params_uniform.buffer
    }

    pub fn grid_size(&self) -> u32 {
        self.buffers.grid_size()
    }

    pub fn command_buffer(&self) -> &wgpu::Buffer {
        self.buffers.command_buffer()
    }

    pub fn current_temp_buffer(&self) -> &wgpu::Buffer {
        self.buffers.current_temp_read()
    }

    pub fn stats_staging_buffer(&self) -> &wgpu::Buffer {
        self.buffers.stats_staging_buffer()
    }

    pub fn tick_count(&self) -> u32 {
        self.tick_count
    }

    pub fn current_write_buffer(&self) -> &wgpu::Buffer {
        self.buffers.current_write_buffer()
    }

    pub fn reset_tick_count(&mut self) {
        self.tick_count = 0;
        self.buffers.reset_read_is_a();
    }

    /// Load a preset by ID: 0=Petri Dish, 1=Gradient, 2=Arena
    pub fn initialize_grid_with_preset(&mut self, queue: &wgpu::Queue, preset: u32) {
        // Clear buffer A to zeros
        let gs = self.buffers.grid_size();
        let total_voxels = (gs as usize).pow(3);
        let zero_data = vec![0u8; total_voxels * 32]; // 8 u32 * 4 bytes
        queue.write_buffer(self.buffers.buffer_a(), 0, &zero_data);

        match preset {
            0 => self.seed_petri_dish(queue),
            1 => self.seed_gradient(queue),
            2 => self.seed_arena(queue),
            _ => self.seed_petri_dish(queue),
        }
    }

    fn seed_petri_dish(&self, queue: &wgpu::Queue) {
        // Original initialize_grid logic
        let gs = self.buffers.grid_size();
        let center = gs / 2;
        let mut voxel_data: Vec<(usize, [u32; 8])> = Vec::new();

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
            let idx = types::grid_index(x.min(gs - 1), y.min(gs - 1), z, gs);
            voxel_data.push((idx, v.pack()));
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
                        let idx = types::grid_index(x, y, z, gs);
                        voxel_data.push((idx, v.pack()));
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
            let idx = types::grid_index(x, center, center, gs);
            voxel_data.push((idx, v.pack()));
        }

        // Protocells (~50 in tight cluster near center)
        for i in 0..50u32 {
            let angle = (i as f32) * 0.126;
            let radius = 1.0 + (i as f32) * 0.08;
            let layer = (i / 16) as f32;
            let x = (center as f32 + angle.cos() * radius).round() as u32;
            let y = (center as f32 + angle.sin() * radius).round() as u32;
            let z = (center as f32 - 2.0 + layer).round() as u32;
            let x = x.min(gs - 1);
            let y = y.min(gs - 1);
            let z = z.min(gs - 1);

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
            let idx = types::grid_index(x, y, z, gs);
            voxel_data.push((idx, v.pack()));
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
            let idx = types::grid_index(x, y, center, gs);
            voxel_data.push((idx, v.pack()));
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
            let idx = types::grid_index(x, center, z, gs);
            voxel_data.push((idx, v.pack()));
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
            let idx = types::grid_index(x, center, z, gs);
            voxel_data.push((idx, v.pack()));
        }

        // Upload voxels to buffer A
        for (idx, words) in &voxel_data {
            let byte_offset = (*idx as u64) * 32;
            let bytes: &[u8] = bytemuck::cast_slice(words.as_slice());
            queue.write_buffer(self.buffers.buffer_a(), byte_offset, bytes);
        }

        // Initialize temperature to ambient
        self.init_temperature(queue);
        self.params_uniform.upload(queue, &self.params);
    }

    fn seed_gradient(&self, queue: &wgpu::Queue) {
        let gs = self.buffers.grid_size();
        let mut voxel_data: Vec<(usize, [u32; 8])> = Vec::new();

        // Heat sources along x=0 face
        for y in (0..gs).step_by((gs / 8) as usize) {
            for z in (0..gs).step_by((gs / 8) as usize) {
                let v = Voxel {
                    voxel_type: VoxelType::HeatSource,
                    energy: 1000,
                    ..Default::default()
                };
                let idx = types::grid_index(0, y, z, gs);
                voxel_data.push((idx, v.pack()));
            }
        }

        // Cold sources along x=gs-1 face
        for y in (0..gs).step_by((gs / 8) as usize) {
            for z in (0..gs).step_by((gs / 8) as usize) {
                let v = Voxel {
                    voxel_type: VoxelType::ColdSource,
                    energy: 1000,
                    ..Default::default()
                };
                let idx = types::grid_index(gs - 1, y, z, gs);
                voxel_data.push((idx, v.pack()));
            }
        }

        // Scattered nutrients in the middle third
        let third = gs / 3;
        for dx in 0..third {
            for dy in (0..gs).step_by(3) {
                for dz in (0..gs).step_by(3) {
                    let x = third + dx;
                    if x < gs && dy < gs && dz < gs {
                        let v = Voxel {
                            voxel_type: VoxelType::Nutrient,
                            energy: 200,
                            ..Default::default()
                        };
                        let idx = types::grid_index(x, dy, dz, gs);
                        voxel_data.push((idx, v.pack()));
                    }
                }
            }
        }

        // Energy sources in center strip
        let center = gs / 2;
        for y in (0..gs).step_by((gs / 6).max(1) as usize) {
            for z in (0..gs).step_by((gs / 6).max(1) as usize) {
                let v = Voxel {
                    voxel_type: VoxelType::EnergySource,
                    energy: 500,
                    ..Default::default()
                };
                let idx = types::grid_index(center, y, z, gs);
                voxel_data.push((idx, v.pack()));
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
            let idx = types::grid_index(x, y, z, gs);
            voxel_data.push((idx, v.pack()));
        }

        for (idx, words) in &voxel_data {
            let byte_offset = (*idx as u64) * 32;
            let bytes: &[u8] = bytemuck::cast_slice(words.as_slice());
            queue.write_buffer(self.buffers.buffer_a(), byte_offset, bytes);
        }

        self.init_temperature(queue);
        self.params_uniform.upload(queue, &self.params);
    }

    fn seed_arena(&self, queue: &wgpu::Queue) {
        let gs = self.buffers.grid_size();
        let center = gs / 2;
        let mut voxel_data: Vec<(usize, [u32; 8])> = Vec::new();

        // Wall cross dividing grid into 4 quadrants (along y=center and x=center planes)
        // with gaps at center ± gap_size for migration
        let gap = (gs / 16).max(2);
        for i in 0..gs {
            for z in 0..gs {
                // Wall along x=center (skip gap around y=center)
                if !(center.saturating_sub(gap)..=center + gap).contains(&i) {
                    let v = Voxel { voxel_type: VoxelType::Wall, ..Default::default() };
                    let idx = types::grid_index(center, i, z, gs);
                    voxel_data.push((idx, v.pack()));
                }
                // Wall along y=center (skip gap around x=center)
                if !(center.saturating_sub(gap)..=center + gap).contains(&i) {
                    let v = Voxel { voxel_type: VoxelType::Wall, ..Default::default() };
                    let idx = types::grid_index(i, center, z, gs);
                    voxel_data.push((idx, v.pack()));
                }
            }
        }

        // Quadrant 1 (low x, low y): nutrient-rich
        let q_size = center.saturating_sub(1);
        for dx in (1..q_size).step_by(2) {
            for dy in (1..q_size).step_by(2) {
                for dz in (0..gs).step_by(4) {
                    let v = Voxel {
                        voxel_type: VoxelType::Nutrient,
                        energy: 300,
                        ..Default::default()
                    };
                    let idx = types::grid_index(dx, dy, dz, gs);
                    voxel_data.push((idx, v.pack()));
                }
            }
        }

        // Quadrant 2 (high x, low y): hot + energy sources
        for i in 0..4u32 {
            let x = (center + 2 + i * (q_size / 5)).min(gs - 1);
            let y = q_size / 2;
            let v = Voxel {
                voxel_type: VoxelType::HeatSource,
                energy: 1000,
                ..Default::default()
            };
            let idx = types::grid_index(x, y, center, gs);
            voxel_data.push((idx, v.pack()));
        }
        for i in 0..6u32 {
            let x = (center + 2 + i * (q_size / 7)).min(gs - 1);
            let y = (1 + i * (q_size / 7)).min(center.saturating_sub(2));
            let v = Voxel {
                voxel_type: VoxelType::EnergySource,
                energy: 500,
                ..Default::default()
            };
            let idx = types::grid_index(x, y, center, gs);
            voxel_data.push((idx, v.pack()));
        }

        // Quadrant 3 (low x, high y): cold + sparse nutrients
        for i in 0..4u32 {
            let x = (1 + i * (q_size / 5)).min(center.saturating_sub(2));
            let y = (center + 2 + i * (q_size / 5)).min(gs - 1);
            let v = Voxel {
                voxel_type: VoxelType::ColdSource,
                energy: 1000,
                ..Default::default()
            };
            let idx = types::grid_index(x, y, center, gs);
            voxel_data.push((idx, v.pack()));
        }
        for dx in (1..q_size).step_by(6) {
            for dy in (center + 2..gs.saturating_sub(1)).step_by(6) {
                let v = Voxel {
                    voxel_type: VoxelType::Nutrient,
                    energy: 100,
                    ..Default::default()
                };
                let idx = types::grid_index(dx, dy, center, gs);
                voxel_data.push((idx, v.pack()));
            }
        }

        // Quadrant 4 (high x, high y): balanced
        for i in 0..3u32 {
            let x = (center + 2 + i * (q_size / 4)).min(gs - 1);
            let y = (center + 2 + i * (q_size / 4)).min(gs - 1);
            let v = Voxel {
                voxel_type: VoxelType::EnergySource,
                energy: 500,
                ..Default::default()
            };
            let idx = types::grid_index(x, y, center, gs);
            voxel_data.push((idx, v.pack()));
        }
        for dx in (center + 2..gs.saturating_sub(1)).step_by(4) {
            for dy in (center + 2..gs.saturating_sub(1)).step_by(4) {
                let v = Voxel {
                    voxel_type: VoxelType::Nutrient,
                    energy: 200,
                    ..Default::default()
                };
                let idx = types::grid_index(dx, dy, center, gs);
                voxel_data.push((idx, v.pack()));
            }
        }

        // Seed protocells in each quadrant (15 each)
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
                let x = (qx as f32 + angle.cos() * radius).round() as u32;
                let y = (qy as f32 + angle.sin() * radius).round() as u32;
                let x = x.min(gs - 1);
                let y = y.min(gs - 1);

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
                let idx = types::grid_index(x, y, center, gs);
                voxel_data.push((idx, v.pack()));
            }
        }

        for (idx, words) in &voxel_data {
            let byte_offset = (*idx as u64) * 32;
            let bytes: &[u8] = bytemuck::cast_slice(words.as_slice());
            queue.write_buffer(self.buffers.buffer_a(), byte_offset, bytes);
        }

        self.init_temperature(queue);
        self.params_uniform.upload(queue, &self.params);
    }

    /// Seed ~30% of voxels as protocells for benchmarking. Returns count placed.
    pub fn seed_benchmark(&mut self, queue: &wgpu::Queue) -> u32 {
        let gs = self.buffers.grid_size();
        let total_voxels = (gs as usize).pow(3);

        // Clear buffer A
        let zero_data = vec![0u8; total_voxels * 32];
        queue.write_buffer(self.buffers.buffer_a(), 0, &zero_data);

        let mut count = 0u32;
        // Place ~30% as protocells using deterministic pattern
        for x in 0..gs {
            for y in 0..gs {
                for z in 0..gs {
                    // Simple hash to select ~30%
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
                        let idx = types::grid_index(x, y, z, gs);
                        let byte_offset = (idx as u64) * 32;
                        let words = v.pack();
                        let bytes: &[u8] = bytemuck::cast_slice(words.as_slice());
                        queue.write_buffer(self.buffers.buffer_a(), byte_offset, bytes);
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

    fn init_temperature(&self, queue: &wgpu::Queue) {
        let gs = self.buffers.grid_size();
        let ambient = 0.5f32;
        let ambient_bytes = ambient.to_le_bytes();
        let total_voxels = (gs as usize).pow(3);
        let init_data: Vec<u8> = ambient_bytes.repeat(total_voxels);
        queue.write_buffer(self.buffers.temp_buffer_a(), 0, &init_data);
    }
}
