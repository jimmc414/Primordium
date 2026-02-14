use wgpu;
use types::SimParams;

pub struct ParamsUniform {
    pub buffer: wgpu::Buffer,
}

impl ParamsUniform {
    pub fn new(device: &wgpu::Device, params: &SimParams) -> Self {
        let data = params.to_bytes();
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sim_params"),
            size: data.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buffer }
    }

    pub fn upload(&self, queue: &wgpu::Queue, params: &SimParams) {
        queue.write_buffer(&self.buffer, 0, &params.to_bytes());
    }
}
