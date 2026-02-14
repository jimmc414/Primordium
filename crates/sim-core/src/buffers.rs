use wgpu;

const VOXEL_STRIDE: usize = 8; // 8 u32 per voxel = 32 bytes

pub struct VoxelBuffers {
    pub voxel_buf_a: wgpu::Buffer,
    grid_size: u32,
}

impl VoxelBuffers {
    pub fn new(device: &wgpu::Device, grid_size: u32) -> Self {
        let total_voxels = (grid_size as u64).pow(3);
        let buf_size = total_voxels * (VOXEL_STRIDE as u64) * 4; // 4 bytes per u32

        let voxel_buf_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("voxel_buf_a"),
            size: buf_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self {
            voxel_buf_a,
            grid_size,
        }
    }

    pub fn voxel_buffer(&self) -> &wgpu::Buffer {
        &self.voxel_buf_a
    }

    pub fn grid_size(&self) -> u32 {
        self.grid_size
    }
}
