pub mod gpu;
pub mod timing;
pub mod bridge;

use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use renderer::camera::Camera;
use renderer::Renderer;
use renderer::VoxelPicker;
use sim_core::SimEngine;
use sim_core::SimStats;
use timing::FrameTiming;
use bridge::Tool;

/// Async readback state machine: Idle -> CopyIssued -> MapRequested -> Ready
#[derive(Clone, Copy, PartialEq)]
pub enum ReadbackState {
    Idle,
    CopyIssued,
    MapRequested,
}

pub struct App {
    pub gpu: gpu::GpuContext,
    pub sim_engine: SimEngine,
    pub renderer: Renderer,
    pub camera: Camera,
    pub timing: FrameTiming,
    pub current_tool: Tool,
    pub brush_radius: u32,
    pub pending_commands: Vec<types::Command>,
    pub overlay_mode: u32,
    pub picker: VoxelPicker,
    pub latest_stats: Option<SimStats>,
    pub pick_requested: bool,
    pub pick_coords: Option<(u32, u32, u32)>,
    pub pick_state: ReadbackState,
    pub pick_ready: Rc<Cell<bool>>,
    pub latest_pick: Option<renderer::PickResult>,
    pub stats_tick_counter: u32,
    pub stats_state: ReadbackState,
    pub stats_ready: Rc<Cell<bool>>,
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

    // Try grid sizes from detected tier downward
    let tiers = [128u32, 96, 64];
    let start_idx = match gpu.grid_size {
        128 => 0,
        96 => 1,
        _ => 2,
    };

    let mut grid_size = 0u32;
    let mut sim_engine = None;

    for &tier_size in &tiers[start_idx..] {
        web_sys::console::log_1(&format!("Trying grid {}³...", tier_size).into());
        match SimEngine::try_new(&gpu.device, &gpu.queue, tier_size) {
            Ok(engine) => {
                grid_size = tier_size;
                sim_engine = Some(engine);
                web_sys::console::log_1(
                    &format!("Grid size: {grid_size}\u{00b3}").into(),
                );
                break;
            }
            Err(e) => {
                web_sys::console::warn_1(
                    &format!("Grid {}³ failed: {}. Trying smaller...", tier_size, e).into(),
                );
            }
        }
    }

    let sim_engine = sim_engine.ok_or_else(|| {
        JsValue::from_str("Failed to allocate GPU buffers. GPU may lack sufficient memory.")
    })?;
    sim_engine.initialize_grid(&gpu.queue);

    // Create renderer
    let renderer = Renderer::new(&gpu.device, &gpu.queue, &gpu.surface_config, grid_size);

    // Create camera
    let mut camera = Camera::new(grid_size);
    camera.aspect = gpu.surface_config.width as f32 / gpu.surface_config.height as f32;

    let timing = FrameTiming::new();

    let picker = VoxelPicker::new(&gpu.device);

    let app = App {
        gpu,
        sim_engine,
        renderer,
        camera,
        timing,
        current_tool: Tool::None,
        brush_radius: 0,
        pending_commands: Vec::new(),
        overlay_mode: 0,
        picker,
        latest_stats: None,
        pick_requested: false,
        pick_coords: None,
        pick_state: ReadbackState::Idle,
        pick_ready: Rc::new(Cell::new(false)),
        latest_pick: None,
        stats_tick_counter: 0,
        stats_state: ReadbackState::Idle,
        stats_ready: Rc::new(Cell::new(false)),
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

        // Set overlay mode in params before ticks
        app.sim_engine.params.overlay_mode = app.overlay_mode as f32;

        // Run simulation ticks (commands applied only on first tick)
        for i in 0..ticks_to_run {
            let cmds = if i == 0 { &commands[..] } else { &[] };
            app.sim_engine.tick(&mut encoder, &app.gpu.queue, cmds);
        }

        // Handle pick request: copy voxel data to pick staging buffer
        if app.pick_requested && app.pick_state == ReadbackState::Idle {
            if let Some((x, y, z)) = app.pick_coords {
                let gs = app.sim_engine.grid_size();
                let idx = types::grid_index(x, y, z, gs);
                app.picker.request_pick(
                    &mut encoder,
                    app.sim_engine.current_read_buffer(),
                    idx as u32,
                );
                app.pick_state = ReadbackState::CopyIssued;
            }
        }

        // Track stats readback cadence (every 10 ticks)
        if ticks_to_run > 0 {
            app.stats_tick_counter += ticks_to_run;
        }

        // Update render texture from current read buffer
        app.renderer.update_render_texture(
            &mut encoder,
            &app.gpu.device,
            app.sim_engine.current_read_buffer(),
            app.sim_engine.params_buffer(),
            app.sim_engine.current_temp_buffer(),
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

        // --- Stats readback state machine ---
        // Transition CopyIssued -> MapRequested (issue map_async once)
        if app.stats_tick_counter >= 10 && app.stats_state == ReadbackState::Idle {
            // Stats copy happens every tick via encoder (always copies to staging).
            // We just need to request mapping.
            app.stats_tick_counter = 0;
            app.stats_ready.set(false);
            let flag = app.stats_ready.clone();
            app.sim_engine.stats_staging_buffer().slice(..).map_async(
                wgpu::MapMode::Read,
                move |result| {
                    if result.is_ok() {
                        flag.set(true);
                    }
                },
            );
            app.stats_state = ReadbackState::MapRequested;
        }

        // Transition MapRequested -> Idle (read data when ready)
        if app.stats_state == ReadbackState::MapRequested && app.stats_ready.get() {
            let slice = app.sim_engine.stats_staging_buffer().slice(..);
            let data = slice.get_mapped_range();
            let words: &[u32] = bytemuck::cast_slice(&data);
            let mut arr = [0u32; 32];
            let len = words.len().min(32);
            arr[..len].copy_from_slice(&words[..len]);
            drop(data);
            app.sim_engine.stats_staging_buffer().unmap();
            app.latest_stats = Some(SimStats::from_words(&arr));
            app.stats_state = ReadbackState::Idle;
        }

        // --- Pick readback state machine ---
        // Transition CopyIssued -> MapRequested
        if app.pick_state == ReadbackState::CopyIssued {
            app.pick_ready.set(false);
            let flag = app.pick_ready.clone();
            app.picker.staging_buffer().slice(..).map_async(
                wgpu::MapMode::Read,
                move |result| {
                    if result.is_ok() {
                        flag.set(true);
                    }
                },
            );
            app.pick_state = ReadbackState::MapRequested;
        }

        // Transition MapRequested -> Idle (read data when ready)
        if app.pick_state == ReadbackState::MapRequested && app.pick_ready.get() {
            let slice = app.picker.staging_buffer().slice(..);
            let data = slice.get_mapped_range();
            let bytes: Vec<u8> = data.to_vec();
            drop(data);
            app.picker.staging_buffer().unmap();
            if let Some((x, y, z)) = app.pick_coords {
                app.latest_pick = Some(VoxelPicker::parse_pick(&bytes, x, y, z));
            }
            app.pick_requested = false;
            app.pick_state = ReadbackState::Idle;
        }
    });
}
