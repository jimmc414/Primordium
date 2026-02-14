pub mod buffers;
pub mod uniform;

use buffers::VoxelBuffers;
use uniform::ParamsUniform;
use types::{SimParams, Voxel, VoxelType, Genome};

pub struct SimEngine {
    buffers: VoxelBuffers,
    params_uniform: ParamsUniform,
    params: SimParams,
}

impl SimEngine {
    pub fn new(device: &wgpu::Device, _queue: &wgpu::Queue, grid_size: u32) -> Self {
        let mut params = SimParams::default();
        params.grid_size = grid_size as f32;
        let buffers = VoxelBuffers::new(device, grid_size);
        let params_uniform = ParamsUniform::new(device, &params);
        Self {
            buffers,
            params_uniform,
            params,
        }
    }

    /// Write ~100 test voxels near center of grid for visual verification.
    pub fn initialize_grid(&self, queue: &wgpu::Queue) {
        let gs = self.buffers.grid_size();
        let center = gs / 2;
        let mut voxel_data: Vec<(usize, [u32; 8])> = Vec::new();

        // Wall cluster (3x3x3 block offset from center)
        for dx in 0..3u32 {
            for dy in 0..3u32 {
                for dz in 0..3u32 {
                    let x = center - 10 + dx;
                    let y = center - 10 + dy;
                    let z = center - 10 + dz;
                    let v = Voxel {
                        voxel_type: VoxelType::Wall,
                        energy: 0,
                        ..Default::default()
                    };
                    let idx = types::grid_index(x, y, z, gs);
                    voxel_data.push((idx, v.pack()));
                }
            }
        }

        // Nutrient field (4x4x4 block)
        for dx in 0..4u32 {
            for dy in 0..4u32 {
                for dz in 0..2u32 {
                    let x = center + 5 + dx;
                    let y = center + dy;
                    let z = center + 5 + dz;
                    let v = Voxel {
                        voxel_type: VoxelType::Nutrient,
                        energy: 100,
                        ..Default::default()
                    };
                    let idx = types::grid_index(x, y, z, gs);
                    voxel_data.push((idx, v.pack()));
                }
            }
        }

        // Energy sources (3)
        for i in 0..3u32 {
            let x = center + i * 8;
            let y = center;
            let z = center - 5;
            let v = Voxel {
                voxel_type: VoxelType::EnergySource,
                energy: 500,
                ..Default::default()
            };
            let idx = types::grid_index(x, y, z, gs);
            voxel_data.push((idx, v.pack()));
        }

        // Protocells with varied genomes (20)
        for i in 0..20u32 {
            let angle = (i as f32) * 0.314;
            let radius = 5.0 + (i as f32) * 0.3;
            let x = (center as f32 + angle.cos() * radius) as u32;
            let y = center + (i % 5);
            let z = (center as f32 + angle.sin() * radius) as u32;
            let mut genome = Genome::default();
            genome.bytes[0] = (50 + i * 10) as u8; // metabolic_efficiency
            genome.bytes[1] = (20 + i * 5) as u8;  // metabolic_rate
            genome.bytes[2] = 200;                   // replication_threshold
            genome.bytes[3] = (i * 3) as u8;        // mutation_rate
            genome.bytes[9] = (i * 12) as u8;       // photosynthetic_rate
            genome.bytes[10] = 128;                  // energy_split_ratio
            let species = genome.species_id();
            let v = Voxel {
                voxel_type: VoxelType::Protocell,
                energy: 300 + (i * 20) as u16,
                species_id: species,
                genome,
                ..Default::default()
            };
            let idx = types::grid_index(x, y, z, gs);
            voxel_data.push((idx, v.pack()));
        }

        // Waste (5 voxels)
        for i in 0..5u32 {
            let x = center - 5 + i;
            let y = center + 3;
            let z = center + 3;
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

        // Upload all voxels
        for (idx, words) in &voxel_data {
            let byte_offset = (*idx as u64) * 32; // 8 u32 * 4 bytes
            let bytes: &[u8] = bytemuck::cast_slice(words.as_slice());
            queue.write_buffer(self.buffers.voxel_buffer(), byte_offset, bytes);
        }

        // Upload params
        self.params_uniform.upload(queue, &self.params);
    }

    pub fn voxel_buffer(&self) -> &wgpu::Buffer {
        self.buffers.voxel_buffer()
    }

    pub fn params_buffer(&self) -> &wgpu::Buffer {
        &self.params_uniform.buffer
    }

    pub fn grid_size(&self) -> u32 {
        self.buffers.grid_size()
    }
}
