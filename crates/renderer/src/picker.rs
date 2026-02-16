use wgpu;

pub struct PickResult {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub voxel_type: u8,
    pub energy: u16,
    pub age: u16,
    pub species_id: u16,
    pub genome: [u8; 16],
}

pub struct VoxelPicker {
    staging_buf: wgpu::Buffer,
}

impl VoxelPicker {
    pub fn new(device: &wgpu::Device) -> Self {
        let staging_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pick_staging"),
            size: 32, // 1 voxel = 8 × u32 = 32 bytes
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Self { staging_buf }
    }

    pub fn request_pick(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        voxel_buf: &wgpu::Buffer,
        voxel_index: u32,
    ) {
        let byte_offset = voxel_index as u64 * 32;
        encoder.copy_buffer_to_buffer(voxel_buf, byte_offset, &self.staging_buf, 0, 32);
    }

    pub fn staging_buffer(&self) -> &wgpu::Buffer {
        &self.staging_buf
    }

    pub fn parse_pick(data: &[u8], x: u32, y: u32, z: u32) -> PickResult {
        let words: &[u32] = bytemuck::cast_slice(data);
        let word0 = words[0];
        let word1 = words[1];

        let voxel_type = (word0 & 0xFF) as u8;
        let energy = ((word0 >> 16) & 0xFFFF) as u16;
        let age = (word1 & 0xFFFF) as u16;
        let species_id = ((word1 >> 16) & 0xFFFF) as u16;

        let mut genome = [0u8; 16];
        let genome_bytes: &[u8] = bytemuck::cast_slice(&words[2..6]);
        genome.copy_from_slice(genome_bytes);

        PickResult {
            x,
            y,
            z,
            voxel_type,
            energy,
            age,
            species_id,
            genome,
        }
    }
}
