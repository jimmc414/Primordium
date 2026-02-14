use wgpu;
use web_sys::HtmlCanvasElement;

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
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
    })
}
