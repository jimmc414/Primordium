pub mod buffers;
pub mod uniform;
pub mod pipelines;
pub mod tick;

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
    tick_count: u32,
}

impl SimEngine {
    pub fn new(device: &wgpu::Device, _queue: &wgpu::Queue, grid_size: u32) -> Self {
        let mut params = SimParams::default();
        params.grid_size = grid_size as f32;
        let buffers = VoxelBuffers::new(device, grid_size);
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

        Self {
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
            tick_count: 0,
        }
    }

    /// Seed the grid with M2-friendly initial conditions:
    /// ~1728 nutrients, 3 energy sources, ~50 protocells, 5 walls, 5 waste.
    pub fn initialize_grid(&self, queue: &wgpu::Queue) {
        let gs = self.buffers.grid_size();
        let center = gs / 2;
        let mut voxel_data: Vec<(usize, [u32; 8])> = Vec::new();

        // Walls (5 scattered)
        for i in 0..5u32 {
            let x = center - 15 + i * 3;
            let y = center - 15;
            let z = center;
            let v = Voxel {
                voxel_type: VoxelType::Wall,
                energy: 0,
                ..Default::default()
            };
            let idx = types::grid_index(x, y, z, gs);
            voxel_data.push((idx, v.pack()));
        }

        // Nutrient field (12x12x12 block around center, concentration=200)
        for dx in 0..12u32 {
            for dy in 0..12u32 {
                for dz in 0..12u32 {
                    let x = center - 6 + dx;
                    let y = center - 6 + dy;
                    let z = center - 6 + dz;
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

        // Energy sources (3 near center)
        for i in 0..3u32 {
            let x = center - 1 + i;
            let y = center;
            let z = center;
            let v = Voxel {
                voxel_type: VoxelType::EnergySource,
                energy: 500,
                ..Default::default()
            };
            let idx = types::grid_index(x, y, z, gs);
            voxel_data.push((idx, v.pack()));
        }

        // Protocells (~50 in tight cluster near center, placed AFTER nutrients)
        for i in 0..50u32 {
            let angle = (i as f32) * 0.126;
            let radius = 1.0 + (i as f32) * 0.08;
            let layer = (i / 16) as f32;
            let x = (center as f32 + angle.cos() * radius).round() as u32;
            let y = (center as f32 + angle.sin() * radius).round() as u32;
            let z = (center as f32 - 2.0 + layer).round() as u32;

            // Clamp to grid bounds
            let x = x.min(gs - 1);
            let y = y.min(gs - 1);
            let z = z.min(gs - 1);

            let mut genome = Genome::default();
            genome.bytes[0] = (80 + (i % 20) * 8) as u8;  // metabolic_efficiency
            genome.bytes[1] = (30 + (i % 15) * 5) as u8;   // metabolic_rate
            genome.bytes[2] = 200;                           // replication_threshold
            genome.bytes[3] = (i * 3) as u8;                // mutation_rate
            genome.bytes[4] = (60 + (i % 10) * 15) as u8;  // movement_bias
            genome.bytes[5] = (40 + (i % 8) * 20) as u8;   // chemotaxis_strength
            genome.bytes[9] = (60 + (i % 10) * 15) as u8;  // photosynthetic_rate
            genome.bytes[10] = 128;                          // energy_split_ratio
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
            let x = center + 8 + i;
            let y = center + 8;
            let z = center;
            let v = Voxel {
                voxel_type: VoxelType::Waste,
                age: i as u16 * 20,
                ..Default::default()
            };
            let idx = types::grid_index(x, y, z, gs);
            voxel_data.push((idx, v.pack()));
        }

        // Heat source
        {
            let v = Voxel {
                voxel_type: VoxelType::HeatSource,
                energy: 1000,
                ..Default::default()
            };
            let idx = types::grid_index(center + 10, center, center + 10, gs);
            voxel_data.push((idx, v.pack()));
        }

        // Cold source
        {
            let v = Voxel {
                voxel_type: VoxelType::ColdSource,
                energy: 1000,
                ..Default::default()
            };
            let idx = types::grid_index(center - 10, center, center + 10, gs);
            voxel_data.push((idx, v.pack()));
        }

        // Upload all voxels to buffer A (the initial read buffer)
        for (idx, words) in &voxel_data {
            let byte_offset = (*idx as u64) * 32; // 8 u32 * 4 bytes
            let bytes: &[u8] = bytemuck::cast_slice(words.as_slice());
            queue.write_buffer(self.buffers.buffer_a(), byte_offset, bytes);
        }

        // Initialize temperature buffer A to ambient temperature (0.5)
        let ambient = 0.5f32;
        let ambient_bytes = ambient.to_le_bytes();
        let total_voxels = (gs as usize).pow(3);
        let init_data: Vec<u8> = ambient_bytes.repeat(total_voxels);
        queue.write_buffer(self.buffers.temp_buffer_a(), 0, &init_data);

        // Upload params
        self.params_uniform.upload(queue, &self.params);
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
}
