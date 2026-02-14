pub mod gpu;
pub mod timing;
pub mod bridge;

use wasm_bindgen::prelude::*;
use renderer::camera::Camera;
use renderer::Renderer;
use sim_core::SimEngine;
use timing::FrameTiming;
use bridge::Tool;

pub struct App {
    pub gpu: gpu::GpuContext,
    pub sim_engine: SimEngine,
    pub renderer: Renderer,
    pub camera: Camera,
    pub timing: FrameTiming,
    pub current_tool: Tool,
    pub brush_radius: u32,
    pub pending_commands: Vec<types::Command>,
}

#[wasm_bindgen]
pub async fn init() -> Result<(), JsValue> {
    // Get canvas from DOM
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let canvas = document
        .get_element_by_id("gpu-canvas")
        .ok_or("no canvas element with id 'gpu-canvas'")?;
    let canvas: web_sys::HtmlCanvasElement = canvas
        .dyn_into()
        .map_err(|_| "element is not a canvas")?;

    // Set canvas size to match CSS layout
    let dpr = window.device_pixel_ratio();
    let width = (canvas.client_width() as f64 * dpr) as u32;
    let height = (canvas.client_height() as f64 * dpr) as u32;
    canvas.set_width(width);
    canvas.set_height(height);

    web_sys::console::log_1(&format!("Canvas: {width}x{height} (dpr={dpr:.2})").into());

    // Initialize GPU
    let gpu = gpu::init_gpu(canvas).await.map_err(|e| JsValue::from_str(&e))?;

    // Determine grid size based on adapter limits
    let grid_size = 128u32;
    web_sys::console::log_1(&format!("Grid size: {grid_size}\u{00b3}").into());

    // Create sim engine and seed test voxels
    let sim_engine = SimEngine::new(&gpu.device, &gpu.queue, grid_size);
    sim_engine.initialize_grid(&gpu.queue);

    // Create renderer
    let renderer = Renderer::new(&gpu.device, &gpu.queue, &gpu.surface_config, grid_size);

    // Create camera
    let mut camera = Camera::new(grid_size);
    camera.aspect = gpu.surface_config.width as f32 / gpu.surface_config.height as f32;

    let timing = FrameTiming::new();

    let app = App {
        gpu,
        sim_engine,
        renderer,
        camera,
        timing,
        current_tool: Tool::None,
        brush_radius: 0,
        pending_commands: Vec::new(),
    };

    bridge::APP.with(|cell| {
        *cell.borrow_mut() = Some(app);
    });

    web_sys::console::log_1(&"Primordium initialized".into());
    Ok(())
}

#[wasm_bindgen]
pub fn frame(dt: f32) {
    bridge::APP.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let app = match borrow.as_mut() {
            Some(app) => app,
            None => return,
        };

        app.timing.update(dt);
        let ticks_to_run = app.timing.ticks_due(dt);

        // Get surface texture — don't panic on error
        let surface_texture = match app.gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost) => {
                app.gpu.surface.configure(&app.gpu.device, &app.gpu.surface_config);
                return;
            }
            Err(_) => return,
        };

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = app
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });

        // Drain pending commands for this frame
        let commands: Vec<types::Command> = app.pending_commands.drain(..).collect();

        // Run simulation ticks (commands applied only on first tick)
        for i in 0..ticks_to_run {
            let cmds = if i == 0 { &commands[..] } else { &[] };
            app.sim_engine.tick(&mut encoder, &app.gpu.queue, cmds);
        }

        // Update render texture from current read buffer
        app.renderer.update_render_texture(
            &mut encoder,
            &app.gpu.device,
            app.sim_engine.current_read_buffer(),
            app.sim_engine.params_buffer(),
        );

        // Render frame (ray march + wireframe)
        app.renderer.render_frame(
            &mut encoder,
            &surface_view,
            &app.camera,
            &app.gpu.queue,
            &app.gpu.device,
        );

        app.gpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    });
}
