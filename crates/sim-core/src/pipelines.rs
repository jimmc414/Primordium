use wgpu;

const COMMON_WGSL: &str = include_str!("../../../shaders/common.wgsl");
const RESOLVE_EXECUTE_WGSL: &str = include_str!("../../../shaders/resolve_execute.wgsl");

pub struct SimPipelines {
    pub resolve_execute: wgpu::ComputePipeline,
    pub resolve_execute_bgl: wgpu::BindGroupLayout,
}

impl SimPipelines {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader_source = format!("{}\n{}", COMMON_WGSL, RESOLVE_EXECUTE_WGSL);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("resolve_execute"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let resolve_execute_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("resolve_execute_bgl"),
                entries: &[
                    // binding 0: voxel read buffer (read-only storage)
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
                    // binding 1: voxel write buffer (read_write storage)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
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
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("resolve_execute_pl"),
            bind_group_layouts: &[&resolve_execute_bgl],
            push_constant_ranges: &[],
        });

        let resolve_execute = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("resolve_execute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("resolve_execute_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            resolve_execute,
            resolve_execute_bgl,
        }
    }
}
