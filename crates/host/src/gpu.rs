use wgpu;
use web_sys::HtmlCanvasElement;

#[derive(Debug, Clone, Copy)]
pub enum GpuTier {
    Sparse256, // 256³ sparse — discrete GPU with ≥ 50 MB buffer limits
    High,      // 128³ dense  — discrete GPU with sufficient buffer limits
    Medium,    // 96³  dense  — discrete GPU with smaller limits
    Low,       // 64³  dense  — integrated GPU
}

impl GpuTier {
    pub fn grid_size(self) -> u32 {
        match self {
            GpuTier::Sparse256 => 256,
            GpuTier::High => 128,
            GpuTier::Medium => 96,
            GpuTier::Low => 64,
        }
    }

    pub fn is_sparse(self) -> bool {
        matches!(self, GpuTier::Sparse256)
    }
}

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub tier: GpuTier,
    pub grid_size: u32,
}

pub async fn init_gpu(canvas: HtmlCanvasElement) -> Result<GpuContext, String> {
    let width = canvas.client_width().max(1) as u32;
    let height = canvas.client_height().max(1) as u32;

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });

    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|e| format!("Failed to create surface: {e}"))?;

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .map_err(|e| format!("No suitable GPU adapter: {e}"))?;

    let info = adapter.get_info();
    web_sys::console::log_1(
        &format!(
            "GPU adapter: {} ({:?}), backend: {:?}",
            info.name, info.device_type, info.backend
        )
        .into(),
    );

    let limits = adapter.limits();
    web_sys::console::log_1(
        &format!(
            "Max buffer size: {} MB, max storage buffer: {} MB",
            limits.max_buffer_size / (1024 * 1024),
            limits.max_storage_buffer_binding_size / (1024 * 1024),
        )
        .into(),
    );

    // Detect GPU tier based on adapter type and buffer limits
    let tier = detect_gpu_tier(&info, &limits);
    let grid_size = tier.grid_size();
    web_sys::console::log_1(
        &format!("GPU tier: {:?}, grid size: {}³", tier, grid_size).into(),
    );

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("primordium_device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|e| format!("Failed to create device: {e}"))?;

    let surface_caps = surface.get_capabilities(&adapter);
    let format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &surface_config);

    web_sys::console::log_1(
        &format!("Surface configured: {width}x{height}, format: {format:?}").into(),
    );

    Ok(GpuContext {
        device,
        queue,
        surface,
        surface_config,
        tier,
        grid_size,
    })
}

fn detect_gpu_tier(info: &wgpu::AdapterInfo, limits: &wgpu::Limits) -> GpuTier {
    if info.device_type == wgpu::DeviceType::IntegratedGpu {
        return GpuTier::Low;
    }

    // Sparse 256³ needs ~50 MB per pool buffer (at ~3200 max bricks)
    let sparse_pool = 50u64 * 1024 * 1024; // 50 MB
    if limits.max_buffer_size >= sparse_pool
        && (limits.max_storage_buffer_binding_size as u64) >= sparse_pool
    {
        return GpuTier::Sparse256;
    }

    // 128³ voxel buffer = 128³ * 8 u32 * 4 bytes = 67,108,864 bytes
    let buf_128 = 128u64 * 128 * 128 * 8 * 4;
    if limits.max_buffer_size >= buf_128
        && (limits.max_storage_buffer_binding_size as u64) >= buf_128
    {
        return GpuTier::High;
    }

    // 96³ voxel buffer = 96³ * 8 * 4 = 28,311,552 bytes
    let buf_96 = 96u64 * 96 * 96 * 8 * 4;
    if limits.max_buffer_size >= buf_96
        && (limits.max_storage_buffer_binding_size as u64) >= buf_96
    {
        return GpuTier::Medium;
    }

    GpuTier::Low
}
