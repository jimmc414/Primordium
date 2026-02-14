use wgpu;

const COMMON_WGSL: &str = include_str!("../../../shaders/common.wgsl");
const INTENT_DECLARATION_WGSL: &str = include_str!("../../../shaders/intent_declaration.wgsl");
const RESOLVE_EXECUTE_WGSL: &str = include_str!("../../../shaders/resolve_execute.wgsl");

pub struct SimPipelines {
    pub intent_declaration: wgpu::ComputePipeline,
    pub intent_declaration_bgl: wgpu::BindGroupLayout,
    pub resolve_execute: wgpu::ComputePipeline,
    pub resolve_execute_bgl: wgpu::BindGroupLayout,
}

impl SimPipelines {
    pub fn new(device: &wgpu::Device) -> Self {
        // ---- Intent declaration pipeline ----
        let intent_source = format!("{}\n{}", COMMON_WGSL, INTENT_DECLARATION_WGSL);
        let intent_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("intent_declaration"),
            source: wgpu::ShaderSource::Wgsl(intent_source.into()),
        });

        let intent_declaration_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("intent_declaration_bgl"),
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
                    // binding 1: intent buffer (read_write storage)
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

        let intent_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("intent_declaration_pl"),
            bind_group_layouts: &[&intent_declaration_bgl],
            push_constant_ranges: &[],
        });

        let intent_declaration =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("intent_declaration_pipeline"),
                layout: Some(&intent_pl),
                module: &intent_shader,
                entry_point: Some("intent_declaration_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        // ---- Resolve execute pipeline ----
        let resolve_source = format!("{}\n{}", COMMON_WGSL, RESOLVE_EXECUTE_WGSL);
        let resolve_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("resolve_execute"),
            source: wgpu::ShaderSource::Wgsl(resolve_source.into()),
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
                    // binding 3: intent buffer (read-only storage)
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

        let resolve_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("resolve_execute_pl"),
            bind_group_layouts: &[&resolve_execute_bgl],
            push_constant_ranges: &[],
        });

        let resolve_execute =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("resolve_execute_pipeline"),
                layout: Some(&resolve_pl),
                module: &resolve_shader,
                entry_point: Some("resolve_execute_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        Self {
            intent_declaration,
            intent_declaration_bgl,
            resolve_execute,
            resolve_execute_bgl,
        }
    }
}
