use wgpu;

const COMMON_WGSL: &str = include_str!("../../../shaders/common.wgsl");
const UPDATE_RENDER_TEXTURE_WGSL: &str = include_str!("../../../shaders/update_render_texture.wgsl");

pub struct RenderTexturePipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    grid_size: u32,
}

impl RenderTexturePipeline {
    pub fn new(device: &wgpu::Device, grid_size: u32) -> Self {
        let shader_source = format!("{}\n{}", COMMON_WGSL, UPDATE_RENDER_TEXTURE_WGSL);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("update_render_texture"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_texture_bgl"),
            entries: &[
                // binding 0: voxel buffer (read-only storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 1: render texture (write-only storage texture)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D3,
                    },
                    count: None,
                },
                // binding 2: sim params uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // binding 3: temp buffer (read-only storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_texture_pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("update_render_texture_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("update_render_texture_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render_tex_3d"),
            size: wgpu::Extent3d {
                width: grid_size,
                height: grid_size,
                depth_or_array_layers: grid_size,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            pipeline,
            bind_group_layout,
            texture,
            texture_view,
            grid_size,
        }
    }

    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        voxel_buf: &wgpu::Buffer,
        params_buf: &wgpu::Buffer,
        temp_buf: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_texture_bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: voxel_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: temp_buf.as_entire_binding(),
                },
            ],
        })
    }

    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder, bind_group: &wgpu::BindGroup) {
        let wg = self.grid_size / 4;
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("update_render_texture_pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(wg, wg, wg);
    }
}
