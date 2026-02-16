use wgpu;

const COMMON_WGSL: &str = include_str!("../../../shaders/common.wgsl");
const BRICK_COMMON_WGSL: &str = include_str!("../../../shaders/brick_common.wgsl");
const INTENT_DECLARATION_WGSL: &str = include_str!("../../../shaders/intent_declaration.wgsl");
const RESOLVE_EXECUTE_WGSL: &str = include_str!("../../../shaders/resolve_execute.wgsl");
const APPLY_COMMANDS_WGSL: &str = include_str!("../../../shaders/apply_commands.wgsl");
const TEMPERATURE_DIFFUSION_WGSL: &str = include_str!("../../../shaders/temperature_diffusion.wgsl");
const STATS_REDUCTION_WGSL: &str = include_str!("../../../shaders/stats_reduction.wgsl");

pub struct SimPipelines {
    pub intent_declaration: wgpu::ComputePipeline,
    pub intent_declaration_bgl: wgpu::BindGroupLayout,
    pub resolve_execute: wgpu::ComputePipeline,
    pub resolve_execute_bgl: wgpu::BindGroupLayout,
    pub apply_commands: wgpu::ComputePipeline,
    pub apply_commands_bgl: wgpu::BindGroupLayout,
    pub temperature_diffusion: wgpu::ComputePipeline,
    pub temperature_diffusion_bgl: wgpu::BindGroupLayout,
    pub stats_reduction: wgpu::ComputePipeline,
    pub stats_reduction_bgl: wgpu::BindGroupLayout,
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
                    // binding 3: temp read buffer (read-only storage)
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
                    // binding 4: temp read buffer (read-only storage)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
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

        // ---- Apply commands pipeline ----
        let apply_source = format!("{}\n{}", COMMON_WGSL, APPLY_COMMANDS_WGSL);
        let apply_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("apply_commands"),
            source: wgpu::ShaderSource::Wgsl(apply_source.into()),
        });

        let apply_commands_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("apply_commands_bgl"),
                entries: &[
                    // binding 0: voxel buffer (read_write — modified in-place)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // binding 1: command buffer (read-only storage)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
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

        let apply_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("apply_commands_pl"),
            bind_group_layouts: &[&apply_commands_bgl],
            push_constant_ranges: &[],
        });

        let apply_commands =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("apply_commands_pipeline"),
                layout: Some(&apply_pl),
                module: &apply_shader,
                entry_point: Some("apply_commands_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        // ---- Temperature diffusion pipeline ----
        let temp_source = format!("{}\n{}", COMMON_WGSL, TEMPERATURE_DIFFUSION_WGSL);
        let temp_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("temperature_diffusion"),
            source: wgpu::ShaderSource::Wgsl(temp_source.into()),
        });

        let temperature_diffusion_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("temperature_diffusion_bgl"),
                entries: &[
                    // binding 0: temp read buffer (read-only storage)
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
                    // binding 1: temp write buffer (read_write storage)
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
                    // binding 2: voxel read buffer (read-only storage)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // binding 3: sim params uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
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

        let temp_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("temperature_diffusion_pl"),
            bind_group_layouts: &[&temperature_diffusion_bgl],
            push_constant_ranges: &[],
        });

        let temperature_diffusion =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("temperature_diffusion_pipeline"),
                layout: Some(&temp_pl),
                module: &temp_shader,
                entry_point: Some("temperature_diffusion_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        // ---- Stats reduction pipeline ----
        let stats_source = format!("{}\n{}", COMMON_WGSL, STATS_REDUCTION_WGSL);
        let stats_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("stats_reduction"),
            source: wgpu::ShaderSource::Wgsl(stats_source.into()),
        });

        let stats_reduction_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("stats_reduction_bgl"),
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
                    // binding 1: stats buffer (read_write storage)
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

        let stats_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("stats_reduction_pl"),
            bind_group_layouts: &[&stats_reduction_bgl],
            push_constant_ranges: &[],
        });

        let stats_reduction =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("stats_reduction_pipeline"),
                layout: Some(&stats_pl),
                module: &stats_shader,
                entry_point: Some("stats_reduction_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        Self {
            intent_declaration,
            intent_declaration_bgl,
            resolve_execute,
            resolve_execute_bgl,
            apply_commands,
            apply_commands_bgl,
            temperature_diffusion,
            temperature_diffusion_bgl,
            stats_reduction,
            stats_reduction_bgl,
        }
    }
}

/// Brick table BGL entry for binding 10 (read-only storage).
fn brick_table_bgl_entry() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 10,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// Sparse pipelines — same 5 compute shaders but compiled with brick_common.wgsl
/// prefix and binding 10 for brick_table.
pub struct SparsePipelines {
    pub intent_declaration: wgpu::ComputePipeline,
    pub intent_declaration_bgl: wgpu::BindGroupLayout,
    pub resolve_execute: wgpu::ComputePipeline,
    pub resolve_execute_bgl: wgpu::BindGroupLayout,
    pub apply_commands: wgpu::ComputePipeline,
    pub apply_commands_bgl: wgpu::BindGroupLayout,
    pub temperature_diffusion: wgpu::ComputePipeline,
    pub temperature_diffusion_bgl: wgpu::BindGroupLayout,
    pub stats_reduction: wgpu::ComputePipeline,
    pub stats_reduction_bgl: wgpu::BindGroupLayout,
}

impl SparsePipelines {
    pub fn new(device: &wgpu::Device) -> Self {
        // ---- Intent declaration pipeline (sparse) ----
        let intent_source = format!("{}\n{}\n{}", COMMON_WGSL, BRICK_COMMON_WGSL, INTENT_DECLARATION_WGSL);
        let intent_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sparse_intent_declaration"),
            source: wgpu::ShaderSource::Wgsl(intent_source.into()),
        });

        let intent_declaration_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sparse_intent_declaration_bgl"),
                entries: &[
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
                    brick_table_bgl_entry(),
                ],
            });

        let intent_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sparse_intent_declaration_pl"),
            bind_group_layouts: &[&intent_declaration_bgl],
            push_constant_ranges: &[],
        });

        let intent_declaration =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("sparse_intent_declaration_pipeline"),
                layout: Some(&intent_pl),
                module: &intent_shader,
                entry_point: Some("intent_declaration_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        // ---- Resolve execute pipeline (sparse) ----
        let resolve_source = format!("{}\n{}\n{}", COMMON_WGSL, BRICK_COMMON_WGSL, RESOLVE_EXECUTE_WGSL);
        let resolve_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sparse_resolve_execute"),
            source: wgpu::ShaderSource::Wgsl(resolve_source.into()),
        });

        let resolve_execute_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sparse_resolve_execute_bgl"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    brick_table_bgl_entry(),
                ],
            });

        let resolve_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sparse_resolve_execute_pl"),
            bind_group_layouts: &[&resolve_execute_bgl],
            push_constant_ranges: &[],
        });

        let resolve_execute =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("sparse_resolve_execute_pipeline"),
                layout: Some(&resolve_pl),
                module: &resolve_shader,
                entry_point: Some("resolve_execute_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        // ---- Apply commands pipeline (sparse) ----
        let apply_source = format!("{}\n{}\n{}", COMMON_WGSL, BRICK_COMMON_WGSL, APPLY_COMMANDS_WGSL);
        let apply_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sparse_apply_commands"),
            source: wgpu::ShaderSource::Wgsl(apply_source.into()),
        });

        let apply_commands_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sparse_apply_commands_bgl"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
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
                    brick_table_bgl_entry(),
                ],
            });

        let apply_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sparse_apply_commands_pl"),
            bind_group_layouts: &[&apply_commands_bgl],
            push_constant_ranges: &[],
        });

        let apply_commands =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("sparse_apply_commands_pipeline"),
                layout: Some(&apply_pl),
                module: &apply_shader,
                entry_point: Some("apply_commands_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        // ---- Temperature diffusion pipeline (sparse) ----
        let temp_source = format!("{}\n{}\n{}", COMMON_WGSL, BRICK_COMMON_WGSL, TEMPERATURE_DIFFUSION_WGSL);
        let temp_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sparse_temperature_diffusion"),
            source: wgpu::ShaderSource::Wgsl(temp_source.into()),
        });

        let temperature_diffusion_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sparse_temperature_diffusion_bgl"),
                entries: &[
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    brick_table_bgl_entry(),
                ],
            });

        let temp_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sparse_temperature_diffusion_pl"),
            bind_group_layouts: &[&temperature_diffusion_bgl],
            push_constant_ranges: &[],
        });

        let temperature_diffusion =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("sparse_temperature_diffusion_pipeline"),
                layout: Some(&temp_pl),
                module: &temp_shader,
                entry_point: Some("temperature_diffusion_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        // ---- Stats reduction pipeline (sparse) ----
        let stats_source = format!("{}\n{}\n{}", COMMON_WGSL, BRICK_COMMON_WGSL, STATS_REDUCTION_WGSL);
        let stats_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sparse_stats_reduction"),
            source: wgpu::ShaderSource::Wgsl(stats_source.into()),
        });

        let stats_reduction_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sparse_stats_reduction_bgl"),
                entries: &[
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
                    brick_table_bgl_entry(),
                ],
            });

        let stats_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sparse_stats_reduction_pl"),
            bind_group_layouts: &[&stats_reduction_bgl],
            push_constant_ranges: &[],
        });

        let stats_reduction =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("sparse_stats_reduction_pipeline"),
                layout: Some(&stats_pl),
                module: &stats_shader,
                entry_point: Some("stats_reduction_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        Self {
            intent_declaration,
            intent_declaration_bgl,
            resolve_execute,
            resolve_execute_bgl,
            apply_commands,
            apply_commands_bgl,
            temperature_diffusion,
            temperature_diffusion_bgl,
            stats_reduction,
            stats_reduction_bgl,
        }
    }
}
