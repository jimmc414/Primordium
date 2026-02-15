use wasm_bindgen::prelude::*;
use std::cell::RefCell;
use glam::Vec4;

use crate::App;

thread_local! {
    pub static APP: RefCell<Option<App>> = RefCell::new(None);
}

#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    None = 0,
    Wall = 1,
    EnergySource = 2,
    Nutrient = 3,
    Seed = 4,
    Toxin = 5,
    Remove = 6,
    HeatSource = 7,
    ColdSource = 8,
}

#[wasm_bindgen]
pub fn on_mouse_move(dx: f32, dy: f32, buttons: u32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            if buttons & 2 != 0 {
                // Right mouse button: orbit
                app.camera.orbit(dx, dy);
            } else if buttons & 4 != 0 {
                // Middle mouse button: pan
                app.camera.pan(dx, dy);
            }
        }
    });
}

#[wasm_bindgen]
pub fn on_scroll(delta: f32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.camera.zoom(delta);
        }
    });
}

#[wasm_bindgen]
pub fn on_key_down(key: String) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            match key.as_str() {
                "c" | "C" => app.camera.cycle_clip_axis(),
                "ArrowUp" => app.camera.adjust_clip_position(0.02),
                "ArrowDown" => app.camera.adjust_clip_position(-0.02),
                "p" | "P" => app.timing.toggle_pause(),
                "n" | "N" => app.timing.request_single_step(),
                "1" => app.current_tool = Tool::Wall,
                "2" => app.current_tool = Tool::EnergySource,
                "3" => app.current_tool = Tool::Nutrient,
                "4" => app.current_tool = Tool::Seed,
                "5" => app.current_tool = Tool::Toxin,
                "6" => app.current_tool = Tool::Remove,
                "7" => app.current_tool = Tool::HeatSource,
                "8" => app.current_tool = Tool::ColdSource,
                "Escape" => app.current_tool = Tool::None,
                _ => {}
            }
        }
    });
}

#[wasm_bindgen]
pub fn set_paused(paused: bool) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.timing.set_paused(paused);
        }
    });
}

#[wasm_bindgen]
pub fn single_step() {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.timing.request_single_step();
        }
    });
}

#[wasm_bindgen]
pub fn set_tick_rate(rate: f32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.timing.set_tick_rate(rate);
        }
    });
}

#[wasm_bindgen]
pub fn set_tool(tool_id: u32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.current_tool = match tool_id {
                0 => Tool::None,
                1 => Tool::Wall,
                2 => Tool::EnergySource,
                3 => Tool::Nutrient,
                4 => Tool::Seed,
                5 => Tool::Toxin,
                6 => Tool::Remove,
                7 => Tool::HeatSource,
                8 => Tool::ColdSource,
                _ => Tool::None,
            };
        }
    });
}

#[wasm_bindgen]
pub fn set_overlay_mode(mode: u32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.overlay_mode = mode;
        }
    });
}

#[wasm_bindgen]
pub fn set_brush_radius(radius: u32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            app.brush_radius = radius.min(5);
        }
    });
}

#[wasm_bindgen]
pub fn on_mouse_down(canvas_x: f32, canvas_y: f32, canvas_w: f32, canvas_h: f32) {
    APP.with(|app| {
        if let Some(ref mut app) = *app.borrow_mut() {
            if app.current_tool == Tool::None {
                return;
            }

            let nx = canvas_x / canvas_w;
            let ny = canvas_y / canvas_h;
            let gs = app.sim_engine.grid_size();

            if let Some((x, y, z)) = ray_cast_grid(&app.camera, nx, ny, gs) {
                let cmd = match app.current_tool {
                    Tool::Wall => types::Command::new(
                        types::CommandType::PlaceVoxel, x, y, z, app.brush_radius, 1, 0,
                    ),
                    Tool::EnergySource => types::Command::new(
                        types::CommandType::PlaceVoxel, x, y, z, app.brush_radius, 3, 0,
                    ),
                    Tool::Nutrient => types::Command::new(
                        types::CommandType::PlaceVoxel, x, y, z, app.brush_radius, 2, 0,
                    ),
                    Tool::Seed => types::Command::new(
                        types::CommandType::SeedProtocells, x, y, z, app.brush_radius, 500, 0,
                    ),
                    Tool::Toxin => types::Command::new(
                        types::CommandType::ApplyToxin, x, y, z, app.brush_radius, 128, 0,
                    ),
                    Tool::Remove => types::Command::new(
                        types::CommandType::RemoveVoxel, x, y, z, app.brush_radius, 0, 0,
                    ),
                    Tool::HeatSource => types::Command::new(
                        types::CommandType::PlaceVoxel, x, y, z, app.brush_radius, 6, 0,
                    ),
                    Tool::ColdSource => types::Command::new(
                        types::CommandType::PlaceVoxel, x, y, z, app.brush_radius, 7, 0,
                    ),
                    Tool::None => return,
                };
                app.pending_commands.push(cmd);
            }
        }
    });
}

/// CPU ray cast: intersect screen point with grid AABB, return nearest grid cell.
fn ray_cast_grid(camera: &renderer::camera::Camera, nx: f32, ny: f32, grid_size: u32) -> Option<(u32, u32, u32)> {
    let inv_vp = camera.view_projection_inverse();
    let gs = grid_size as f32;

    // Unproject near and far plane points from NDC
    let ndc_near = Vec4::new(nx * 2.0 - 1.0, 1.0 - ny * 2.0, -1.0, 1.0);
    let ndc_far = Vec4::new(nx * 2.0 - 1.0, 1.0 - ny * 2.0, 1.0, 1.0);

    let w_near = inv_vp * ndc_near;
    if w_near.w.abs() < 1e-6 {
        return None;
    }
    let origin = w_near.truncate() / w_near.w;

    let w_far = inv_vp * ndc_far;
    if w_far.w.abs() < 1e-6 {
        return None;
    }
    let far_pt = w_far.truncate() / w_far.w;

    let dir = (far_pt - origin).normalize();

    // Ray-AABB slab intersection with [0, gs]^3
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;

    for i in 0..3 {
        let o = match i { 0 => origin.x, 1 => origin.y, _ => origin.z };
        let d = match i { 0 => dir.x, 1 => dir.y, _ => dir.z };
        if d.abs() < 1e-8 {
            if o < 0.0 || o > gs {
                return None;
            }
        } else {
            let t1 = (0.0 - o) / d;
            let t2 = (gs - o) / d;
            let t_near = t1.min(t2);
            let t_far = t1.max(t2);
            t_min = t_min.max(t_near);
            t_max = t_max.min(t_far);
            if t_min > t_max {
                return None;
            }
        }
    }

    // Get entry point (use t_min if positive, else origin is inside)
    let t = if t_min > 0.0 { t_min } else { 0.0 };
    let hit = origin + dir * t;

    // Snap to nearest integer grid coords, clamp to [0, gs-1]
    let x = (hit.x.round() as i32).clamp(0, grid_size as i32 - 1) as u32;
    let y = (hit.y.round() as i32).clamp(0, grid_size as i32 - 1) as u32;
    let z = (hit.z.round() as i32).clamp(0, grid_size as i32 - 1) as u32;

    Some((x, y, z))
}
