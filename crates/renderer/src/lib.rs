pub mod camera;
pub mod render_texture;
pub mod ray_march;
pub mod wireframe;
pub mod picker;

use camera::Camera;
use render_texture::RenderTexturePipeline;
use ray_march::RayMarchPipeline;
use wireframe::WireframePipeline;
pub use picker::{VoxelPicker, PickResult};

pub struct Renderer {
    render_texture: RenderTexturePipeline,
    ray_march: RayMarchPipeline,
    wireframe: WireframePipeline,
    camera_buffer: wgpu::Buffer,
    wireframe_uniform_buffer: wgpu::Buffer,
    grid_size: u32,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        surface_config: &wgpu::SurfaceConfiguration,
        grid_size: u32,
    ) -> Self {
        let render_texture = RenderTexturePipeline::new(device, grid_size);
        let ray_march = RayMarchPipeline::new(device, surface_config.format);
        let wireframe = WireframePipeline::new(device, surface_config.format);

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_uniform"),
            size: 96, // mat4(64) + vec4(16) + vec4(16)
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // wireframe uniform: mat4(64) + vec4(16) = 80 bytes
        let wireframe_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wireframe_uniform"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            render_texture,
            ray_march,
            wireframe,
            camera_buffer,
            wireframe_uniform_buffer,
            grid_size,
        }
    }

    pub fn volume_texture_view(&self) -> &wgpu::TextureView {
        &self.render_texture.texture_view
    }

    pub fn update_render_texture(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        voxel_buf: &wgpu::Buffer,
        params_buf: &wgpu::Buffer,
        temp_buf: &wgpu::Buffer,
    ) {
        let bg = self.render_texture.create_bind_group(device, voxel_buf, params_buf, temp_buf);
        self.render_texture.encode(encoder, &bg);
    }

    pub fn render_frame(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        camera: &Camera,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
    ) {
        // Upload camera uniform
        let camera_data = camera.to_uniform_bytes(self.grid_size);
        queue.write_buffer(&self.camera_buffer, 0, &camera_data);

        // Upload wireframe uniform (view_proj + grid_size)
        let vp = camera.view_projection();
        let mut wf_data = Vec::with_capacity(80);
        for col in 0..4 {
            let c = vp.col(col);
            wf_data.extend_from_slice(&c.x.to_le_bytes());
            wf_data.extend_from_slice(&c.y.to_le_bytes());
            wf_data.extend_from_slice(&c.z.to_le_bytes());
            wf_data.extend_from_slice(&c.w.to_le_bytes());
        }
        wf_data.extend_from_slice(&(self.grid_size as f32).to_le_bytes());
        wf_data.extend_from_slice(&0.0f32.to_le_bytes());
        wf_data.extend_from_slice(&0.0f32.to_le_bytes());
        wf_data.extend_from_slice(&0.0f32.to_le_bytes());
        queue.write_buffer(&self.wireframe_uniform_buffer, 0, &wf_data);

        // Ray march pass
        let rm_bg = self.ray_march.create_bind_group(
            device,
            &self.render_texture.texture_view,
            &self.camera_buffer,
        );
        self.ray_march.encode(encoder, surface_view, &rm_bg);

        // Wireframe pass (over ray march output)
        let wf_bg = self.wireframe.create_bind_group(device, &self.wireframe_uniform_buffer);
        self.wireframe.encode(encoder, surface_view, &wf_bg);
    }
}
