use wgpu;

const VOXEL_STRIDE: usize = 8; // 8 u32 per voxel = 32 bytes

pub struct VoxelBuffers {
    voxel_buf_a: wgpu::Buffer,
    voxel_buf_b: wgpu::Buffer,
    intent_buf: wgpu::Buffer,
    grid_size: u32,
    current_read_is_a: bool,
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

        let voxel_buf_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("voxel_buf_b"),
            size: buf_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // 1 u32 per voxel for intent encoding
        let intent_size = total_voxels * 4;
        let intent_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("intent_buf"),
            size: intent_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            voxel_buf_a,
            voxel_buf_b,
            intent_buf,
            grid_size,
            current_read_is_a: true,
        }
    }

    pub fn buffer_a(&self) -> &wgpu::Buffer {
        &self.voxel_buf_a
    }

    pub fn buffer_b(&self) -> &wgpu::Buffer {
        &self.voxel_buf_b
    }

    pub fn current_read_buffer(&self) -> &wgpu::Buffer {
        if self.current_read_is_a {
            &self.voxel_buf_a
        } else {
            &self.voxel_buf_b
        }
    }

    pub fn current_write_buffer(&self) -> &wgpu::Buffer {
        if self.current_read_is_a {
            &self.voxel_buf_b
        } else {
            &self.voxel_buf_a
        }
    }

    pub fn swap(&mut self) {
        self.current_read_is_a = !self.current_read_is_a;
    }

    pub fn current_read_is_a(&self) -> bool {
        self.current_read_is_a
    }

    pub fn grid_size(&self) -> u32 {
        self.grid_size
    }

    pub fn intent_buffer(&self) -> &wgpu::Buffer {
        &self.intent_buf
    }
}
